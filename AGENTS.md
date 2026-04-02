# Zed-IFC: A zed extension for Industry Foundation Classes (IFC) Language Support

This repository contains a zed extension for the IFC file format. It uses the IFC-Language-Server as well as the trr-sitter-ifc parser.

## General Approach

The features are already handled through these two pieces of software:

- IFC-Language-Server: https://github.com/NepomukWolf/IFC-Language-Server
- tree-sitter-ifc: https://github.com/NepomukWolf/tree-sitter-ifc

The IFC-LS is written in rust and results in a ifc-language-server-binary, these binaries are compiled for all three platforms and published as a release. The latest release 0.2.0 is accessible via: https://github.com/NepomukWolf/IFC-Language-Server/releases/tag/v0.2.0

Tree-sitter-ifc is a tree sitter for the STEP syntax and it also creates the AST. This repository is public. 

The Zed-IFC repository siply serves as a client for these two pieces of software, to be used in the Zed Editor. The binary and the tree-sitter should be pulled when installing the extension. The binary should not be bundled in this repo. 

This extension should work for MacOS (ARM) and Linux/Windows x86/64.
