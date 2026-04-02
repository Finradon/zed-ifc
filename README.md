# zed-ifc

IFC language support for [Zed](https://zed.dev/).

This extension adds support for Industry Foundation Classes (`.ifc`) files using:

- [`IFC-Language-Server`](https://github.com/NepomukWolf/IFC-Language-Server) for language features
- [`tree-sitter-ifc`](https://github.com/NepomukWolf/tree-sitter-ifc) for parsing and syntax highlighting

## Features

- Syntax highlighting for STEP/IFC source
- Automatic `.ifc` file detection
- Hover for IFC entity names and references
- Go to definition for local `#123` references
- Find references within the current document

## Installation

Install the extension from the Zed extensions page.

For local development, use `zed: install dev extension` and select this repository.

## Language Server

The extension looks for `ifc-language-server` on your `PATH` first. If it is not available, Zed downloads the matching release asset from GitHub automatically.

Supported auto-download targets:

- macOS `arm64`
- Linux `x86_64`
- Windows `x86_64`

Pinned upstream versions in this repository:

- `IFC-Language-Server` `v0.2.0`, published April 1, 2026
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

## Development

```bash
cargo check
```

Generated local extension artifacts such as `extension.wasm` and fetched grammar caches are intentionally ignored and are not meant to be committed.
