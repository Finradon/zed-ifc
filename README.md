# zed-ifc

IFC language support for [Zed](https://zed.dev/).

This extension adds support for Industry Foundation Classes (`.ifc`) files using:

- [`IFC-Language-Server`](https://github.com/NepomukWolf/IFC-Language-Server) for language features
- [`tree-sitter-ifc`](https://github.com/NepomukWolf/tree-sitter-ifc) for parsing and syntax highlighting

## Features

- Syntax highlighting for STEP/IFC source
- Automatic `.ifc` file detection
- Hover for IFC entity names and references
- Hover information for derived attributes
- Go to definition for local `#123` references
- Find references within the current document
- Schema-aware diagnostics for IFC 2x3, IFC 4, and IFC 4x3 files
- Range-based semantic tokens for syntax highlighting
- Configurable AST parsing threshold for large IFC files
- Optional local EXPRESS schema configuration for custom IFC schemas

## Installation

Install the extension from the Zed extensions page.

For local development, use `zed: install dev extension` and select this repository.

## Language Server

The extension looks for `ifc-language-server` on your `PATH` first. If it is not available, Zed downloads the matching release asset from GitHub automatically.

`cargo clean` only removes Rust build artifacts for this extension. It does not remove a previously downloaded IFC language server from Zed's extension working directory.
If Zed does not see the same `PATH` as your shell, set an explicit binary path in Zed settings instead of relying on auto-discovery.
The extension launches the server through a small wrapper that appends server `stderr` tracing to `logs/ifc-language-server.log` in the extension working directory. It sets `IFC_LSP_LOG` to `ifc_language_server=info,tower_lsp=warn` by default; set `IFC_LSP_LOG` in the shell environment before launching Zed to override it.

Supported auto-download targets:

- macOS `arm64`
- Linux `x86_64`
- Windows `x86_64`

Pinned upstream versions in this repository:

- `IFC-Language-Server` `v0.4.0`, published June 1, 2026
- `tree-sitter-ifc` commit `bd5039f5d7929a9a8e1c138cd449ec385f17789e`, dated March 24, 2026

## Configuration

To use a manually installed language server binary instead of the auto-downloaded one:

```jsonc
{
  "lsp": {
    "ifc-language-server": {
      "binary": {
        "path": "/absolute/path/to/ifc-language-server"
      }
    }
  }
}
```

### Large files and semantic tokens

The extension sends these IFC language server `initialization_options` by default:

```jsonc
{
  "astFileSizeLimitMb": 70,
  "semanticTokensEnabled": true
}
```

Override the AST threshold for tree-sitter-backed diagnostics and derived-value hover with `astFileSizeLimitMb`.
Files above the threshold keep basic hover, local navigation, find references, and range-based semantic tokens available, but skip AST-backed diagnostics and derived-value hover.

```jsonc
{
  "lsp": {
    "ifc-language-server": {
      "initialization_options": {
        "astFileSizeLimitMb": 128,
        "semanticTokensEnabled": true
      }
    }
  },
  "languages": {
    "IFC": {
      "semantic_tokens": "combined"
    }
  }
}
```

Zed does not request semantic tokens by default. Use `"semantic_tokens": "combined"` to overlay LSP semantic tokens on tree-sitter highlighting, or `"full"` to use only LSP semantic tokens.
Configuration changes are only applied when the language server restarts; use `editor: restart language server` from the Command Palette after changing these values.

### Custom EXPRESS schemas

The IFC language server reads custom schema configuration from LSP `initialization_options`.
Open Zed's settings from the Command Palette with `zed: open settings` or `zed: open project settings`, then add:

```jsonc
{
  "lsp": {
    "ifc-language-server": {
      "initialization_options": {
        "overwriteExpSchemaWithLocal": "/absolute/path/to/IFC4x2.exp",
        "addLocalSchemaToSelection": [
          "/absolute/path/to/IFC4x1.exp",
          "/absolute/path/to/custom-schemas"
        ]
      }
    }
  }
}
```

On Windows, escape backslashes in JSON strings:

```jsonc
{
  "lsp": {
    "ifc-language-server": {
      "initialization_options": {
        "overwriteExpSchemaWithLocal": "C:\\Users\\alice\\express\\IFC4x2.exp",
        "addLocalSchemaToSelection": [
          "C:\\Users\\alice\\express\\custom-schemas"
        ]
      }
    }
  }
}
```

`overwriteExpSchemaWithLocal` forces one `.exp` schema for diagnostics and hover.
`addLocalSchemaToSelection` adds `.exp` files or directories containing `.exp` files to the schema lookup pool.
Configuration changes are only applied when the language server restarts; use `editor: restart language server` from the Command Palette after changing these values.

## Development

```bash
cargo check
```

Generated local extension artifacts such as `extension.wasm` and fetched grammar caches are intentionally ignored and are not meant to be committed.
