use std::fs;

use zed_extension_api::{
    self as zed, serde_json, set_language_server_installation_status as set_install_status,
    settings::LspSettings, LanguageServerId, LanguageServerInstallationStatus as Status, Result,
};

const LANGUAGE_SERVER_ID: &str = "ifc-language-server";
const LANGUAGE_SERVER_REPOSITORY: &str = "NepomukWolf/IFC-Language-Server";
const BINARY_NAME: &str = "ifc-language-server";
const WINDOWS_BINARY_NAME: &str = "ifc-language-server.exe";
const VERSION_DIR_PREFIX: &str = "ifc-language-server-";

struct IfcBinary {
    path: String,
    args: Vec<String>,
}

struct IfcExtension {
    cached_binary_path: Option<String>,
}

impl IfcExtension {
    fn language_server_binary(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<IfcBinary> {
        let settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree);
        let binary = settings.ok().and_then(|settings| settings.binary);
        let args = binary
            .as_ref()
            .and_then(|binary| binary.arguments.clone())
            .unwrap_or_default();

        let path = binary
            .and_then(|binary| binary.path)
            .or_else(|| worktree.which(BINARY_NAME))
            .unwrap_or(self.zed_managed_binary_path(language_server_id)?);

        Ok(IfcBinary { path, args })
    }

    fn zed_managed_binary_path(&mut self, language_server_id: &LanguageServerId) -> Result<String> {
        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).is_ok_and(|metadata| metadata.is_file()) {
                return Ok(path.clone());
            }
        }

        set_install_status(language_server_id, &Status::CheckingForUpdate);
        let release = zed::latest_github_release(
            LANGUAGE_SERVER_REPOSITORY,
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let (platform, architecture) = zed::current_platform();
        let (asset_name, binary_name, file_type) = match (platform, architecture) {
            (zed::Os::Mac, zed::Architecture::Aarch64) => (
                format!("ifc-language-server-{}-macos-arm64.tar.gz", release.version),
                BINARY_NAME,
                zed::DownloadedFileType::GzipTar,
            ),
            (zed::Os::Linux, zed::Architecture::X8664) => (
                format!("ifc-language-server-{}-linux-x86_64.tar.gz", release.version),
                BINARY_NAME,
                zed::DownloadedFileType::GzipTar,
            ),
            (zed::Os::Windows, zed::Architecture::X8664) => (
                format!("ifc-language-server-{}-windows-x86_64.zip", release.version),
                WINDOWS_BINARY_NAME,
                zed::DownloadedFileType::Zip,
            ),
            _ => {
                return Err(
                    "unsupported platform for IFC language server; supported targets are macOS arm64, Linux x86_64, and Windows x86_64"
                        .to_string(),
                )
            }
        };

        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("no asset found matching {asset_name:?}"))?;

        let version_dir = format!("{VERSION_DIR_PREFIX}{}", release.version);
        let binary_path = format!("{version_dir}/{binary_name}");

        if !fs::metadata(&binary_path).is_ok_and(|metadata| metadata.is_file()) {
            fs::create_dir_all(&version_dir)
                .map_err(|err| format!("failed to create {version_dir:?}: {err}"))?;

            set_install_status(language_server_id, &Status::Downloading);
            zed::download_file(&asset.download_url, &version_dir, file_type)
                .map_err(|err| format!("failed to download IFC language server: {err}"))?;

            if platform != zed::Os::Windows {
                zed::make_file_executable(&binary_path)?;
            }

            self.remove_old_installations(&version_dir);
        }

        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
    }

    fn remove_old_installations(&self, current_version_dir: &str) {
        let Ok(entries) = fs::read_dir(".") else {
            return;
        };

        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };

            if !file_type.is_dir() {
                continue;
            }

            let Some(name) = entry.file_name().to_str().map(ToOwned::to_owned) else {
                continue;
            };

            if name.starts_with(VERSION_DIR_PREFIX) && name != current_version_dir {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
    }
}

impl zed::Extension for IfcExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        if language_server_id.as_ref() != LANGUAGE_SERVER_ID {
            return Err(format!(
                "unrecognized language server for IFC extension: {language_server_id}"
            ));
        }

        let binary = self.language_server_binary(language_server_id, worktree)?;
        Ok(zed::Command {
            command: binary.path,
            args: binary.args,
            env: vec![],
        })
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        let settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.initialization_options.clone())
            .unwrap_or_default();

        Ok(Some(settings))
    }

    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        let settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.settings.clone())
            .unwrap_or_default();

        Ok(Some(settings))
    }
}

zed::register_extension!(IfcExtension);
