# Knox Editor Tools

## VS Code Extension (MVP)

- **Location:** `tools/vscode-knox/`
- **MVP scope:** Syntax highlighting via TextMate grammar; language configuration (brackets, comments). Optional: LSP scaffold (client/server) but not required to ship.

## Syntax Highlighting

- **File:** `syntaxes/knox.tmLanguage.json`
- **Scope:** `.kx` files.
- **Tokens:** Keywords (`fn`, `let`, `mut`, `if`, `match`, `return`, `Ok`, `Err`, `Option`, `Result`, `dynamic`, `true`, `false`), strings, numbers, comments, function names (calls/definitions), types (e.g. `u64`, `int`, `string`, `bool`, `Option`, `Result`), operators (e.g. `->`, `=>`, `?`, `:`).

## Language Configuration

- **File:** `language-configuration.json`
- **Contents:** Bracket pairs `()`, `[]`, `{}`; line and block comments `//`, `/* */`; optional word patterns for navigation.

## Installation (development)

- Open `tools/vscode-knox` in VS Code; run "Extension: Run Extension" or package vsix and install from VSIX.
- Root README must document: open `tools/vscode-knox`, run `npm install`, then install from VSIX or run from workspace.

## LSP (optional / future)

- Scaffold: client (VS Code) + server (e.g. `knox_cli lsp` or separate binary). MVP can ship highlighting only; LSP for go-to-def, diagnostics, hover can follow.
