use std::{fs, path::PathBuf};

use zed_extension_api::{
    self as zed, LanguageServerId, LanguageServerInstallationStatus as Status, Result, serde_json,
    set_language_server_installation_status as set_install_status, settings::LspSettings,
};

const LANGUAGE_SERVER_ID: &str = "ifc-language-server";
const LANGUAGE_SERVER_REPOSITORY: &str = "NepomukWolf/IFC-Language-Server";
const LANGUAGE_SERVER_VERSION: &str = "v0.4.0";
const BINARY_NAME: &str = "ifc-language-server";
const WINDOWS_BINARY_NAME: &str = "ifc-language-server.exe";
const VERSION_DIR_PREFIX: &str = "ifc-language-server-";
const DEFAULT_AST_FILE_SIZE_LIMIT_MB: u64 = 70;
const DEFAULT_LOG_FILTER: &str = "ifc_language_server=info,tower_lsp=warn";
const IFC_LSP_LOG_ENV: &str = "IFC_LSP_LOG";
const LOG_DIR: &str = "logs";
const UNIX_WRAPPER: &str = "./ifc-language-server-wrapper";
const WINDOWS_WRAPPER: &str = ".\\ifc-language-server-wrapper.cmd";

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

        let path = if let Some(path) = binary
            .and_then(|binary| binary.path)
            .or_else(|| self.path_binary(worktree))
        {
            path
        } else {
            self.zed_managed_binary_path(language_server_id)?
        };

        Ok(IfcBinary { path, args })
    }

    fn path_binary(&self, worktree: &zed::Worktree) -> Option<String> {
        worktree.which(BINARY_NAME).or_else(|| {
            let binary_name = match zed::current_platform() {
                (zed::Os::Windows, _) => WINDOWS_BINARY_NAME,
                _ => BINARY_NAME,
            };

            let path = worktree
                .shell_env()
                .into_iter()
                .find_map(|(key, value)| key.eq_ignore_ascii_case("PATH").then_some(value))?;

            self.find_binary_in_path(&path, binary_name)
        })
    }

    fn find_binary_in_path(&self, path: &str, binary_name: &str) -> Option<String> {
        let separator = match zed::current_platform() {
            (zed::Os::Windows, _) => ';',
            _ => ':',
        };

        path.split(separator).find_map(|entry| {
            let candidate = PathBuf::from(entry).join(binary_name);
            fs::metadata(&candidate)
                .ok()
                .filter(|metadata| metadata.is_file())
                .and_then(|_| candidate.to_str().map(ToOwned::to_owned))
        })
    }

    fn zed_managed_binary_path(&mut self, language_server_id: &LanguageServerId) -> Result<String> {
        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).is_ok_and(|metadata| metadata.is_file()) {
                return Ok(path.clone());
            }
        }

        set_install_status(language_server_id, &Status::CheckingForUpdate);
        let release =
            zed::github_release_by_tag_name(LANGUAGE_SERVER_REPOSITORY, LANGUAGE_SERVER_VERSION)?;

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
            .ok_or_else(|| {
                format!(
                    "no asset found matching {asset_name:?} for IFC language server {LANGUAGE_SERVER_VERSION}"
                )
            })?;

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

    fn logging_wrapper_path(&self, binary_path: &str) -> Result<String> {
        fs::create_dir_all(LOG_DIR)
            .map_err(|err| format!("failed to create {LOG_DIR:?}: {err}"))?;

        let binary_path = fs::canonicalize(binary_path)
            .map_err(|err| format!("failed to resolve binary path {binary_path:?}: {err}"))?;
        let log_dir = fs::canonicalize(LOG_DIR)
            .map_err(|err| format!("failed to resolve log directory {LOG_DIR:?}: {err}"))?;
        let log_file = log_dir.join("ifc-language-server.log");

        let (platform, _) = zed::current_platform();
        let (wrapper_path, wrapper_contents) = match platform {
            zed::Os::Windows => (
                WINDOWS_WRAPPER,
                format!(
                    "@echo off\r\nif not exist {log_dir} mkdir {log_dir}\r\n{binary_path} %* 2>>{log_file}\r\n",
                    log_dir = windows_batch_quote(&log_dir.display().to_string()),
                    binary_path = windows_batch_quote(&binary_path.display().to_string()),
                    log_file = windows_batch_quote(&log_file.display().to_string()),
                ),
            ),
            _ => (
                UNIX_WRAPPER,
                format!(
                    "#!/bin/sh\nmkdir -p {log_dir}\nexec {binary_path} \"$@\" 2>>{log_file}\n",
                    log_dir = shell_single_quote(&log_dir.display().to_string()),
                    binary_path = shell_single_quote(&binary_path.display().to_string()),
                    log_file = shell_single_quote(&log_file.display().to_string()),
                ),
            ),
        };

        fs::write(wrapper_path, wrapper_contents)
            .map_err(|err| format!("failed to write {wrapper_path:?}: {err}"))?;

        if platform != zed::Os::Windows {
            zed::make_file_executable(wrapper_path)?;
        }

        fs::canonicalize(wrapper_path)
            .map(|path| path.display().to_string())
            .map_err(|err| format!("failed to resolve wrapper path {wrapper_path:?}: {err}"))
    }

    fn language_server_env(&self, worktree: &zed::Worktree) -> zed::EnvVars {
        let log_filter = worktree
            .shell_env()
            .into_iter()
            .find_map(|(key, value)| key.eq_ignore_ascii_case(IFC_LSP_LOG_ENV).then_some(value))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_LOG_FILTER.to_string());

        vec![(IFC_LSP_LOG_ENV.to_string(), log_filter)]
    }

    fn initialization_options(&self, worktree: &zed::Worktree) -> Option<serde_json::Value> {
        let mut defaults = serde_json::Map::new();
        defaults.insert(
            "astFileSizeLimitMb".to_string(),
            DEFAULT_AST_FILE_SIZE_LIMIT_MB.into(),
        );
        defaults.insert("semanticTokensEnabled".to_string(), true.into());

        match LspSettings::for_worktree(LANGUAGE_SERVER_ID, worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.initialization_options.clone())
        {
            Some(serde_json::Value::Object(user_options)) => {
                defaults.extend(user_options);
                Some(serde_json::Value::Object(defaults))
            }
            Some(user_options) => Some(user_options),
            None => Some(serde_json::Value::Object(defaults)),
        }
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
        validate_language_server_id(language_server_id)?;

        let binary = self.language_server_binary(language_server_id, worktree)?;
        let command = self.logging_wrapper_path(&binary.path)?;
        Ok(zed::Command {
            command,
            args: binary.args,
            env: self.language_server_env(worktree),
        })
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        validate_language_server_id(language_server_id)?;

        Ok(self.initialization_options(worktree))
    }

    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        validate_language_server_id(language_server_id)?;

        let settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.settings.clone());

        Ok(settings)
    }
}

zed::register_extension!(IfcExtension);

fn validate_language_server_id(language_server_id: &LanguageServerId) -> Result<()> {
    if language_server_id.as_ref() == LANGUAGE_SERVER_ID {
        return Ok(());
    }

    Err(format!(
        "unrecognized language server for IFC extension: {language_server_id}"
    ))
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn windows_batch_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('%', "%%").replace('"', "\"\""))
}
