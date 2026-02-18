# Knox Language (VS Code)

Syntax highlighting and language configuration for Knox (`.kx` files).

## Features

- **TextMate grammar** for keywords, strings, numbers, comments, types, operators, and function names
- **Language configuration** for brackets, comments, and auto-closing pairs

## Installation (development)

1. Open this folder in VS Code: `tools/vscode-knox`
2. Run **Extensions: Install from VSIX...** and select the packaged `.vsix`, or:
   - Run `npm install` then `npm run package` to create `knox-lang-0.1.0.vsix`
   - Install the VSIX from the command line: `code --install-extension knox-lang-0.1.0.vsix`

## LSP

Optional: LSP (go-to-def, diagnostics, hover) can be added later; MVP ships highlighting only.
