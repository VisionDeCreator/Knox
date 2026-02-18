# Knox

Knox is a new programming language designed to be the best of all worlds: simple to read and teach, type safe, memory safe, flexible when needed, and able to run everywhere (server, browser, blockchain). The compilation strategy is **Wasm-first** with **AOT on server via Wasmtime**. Browser runs via native Wasm + JS glue. Blockchain is a future deterministic subset inspired by Move resources.

**Authoritative docs** for architecture and specs live in [`.cursor/`](.cursor/) and are the source of truth:

| Doc | Description |
|-----|-------------|
| [00-vision.md](.cursor/00-vision.md) | Project goals, non-goals, philosophy |
| [01-language-spec.md](.cursor/01-language-spec.md) | MVP grammar and syntax |
| [02-type-system.md](.cursor/02-type-system.md) | Types, Option/Result, no null, dynamic |
| [03-memory-model.md](.cursor/03-memory-model.md) | Ownership/borrowing direction |
| [04-targets.md](.cursor/04-targets.md) | wasm-wasi, wasm-web, capability gating |
| [05-compiler-architecture.md](.cursor/05-compiler-architecture.md) | Pipeline: lexer → parser → AST → typecheck → Wasm |
| [06-package-manager.md](.cursor/06-package-manager.md) | Manifest, lockfile, local deps |
| [07-cli.md](.cursor/07-cli.md) | Commands and expected behavior |
| [08-editor-tools.md](.cursor/08-editor-tools.md) | Syntax highlighting, LSP plan |
| [09-roadmap.md](.cursor/09-roadmap.md) | Phases from MVP to blockchain subset |

## Install requirements

- **Rust** (stable): [rustup](https://rustup.rs). Use the repo’s `rust-toolchain.toml` (stable).
- **Wasmtime** (to run wasm-wasi): [wasmtime.dev](https://wasmtime.dev). Required for `knox run`.
- **Node** and **VS Code** only if you work on the [VS Code extension](tools/vscode-knox/) (syntax highlighting).

## Quickstart

```bash
# Build the CLI
cargo build -p knox_cli

# Run Hello World (compiles then runs via Wasmtime)
./target/debug/knox run examples/hello_world/hello.kx
```

Expected output: `Hello, Knox!`

## Commands

| Command | Description |
|--------|-------------|
| `knox new <name>` | Create a new Knox project (directory, `knox.toml`, `main.kx` stub) |
| `knox build --target wasm-wasi <path>` | Build a `.kx` file or package to Wasm |
| `knox run <file.kx>` | Compile and run with Wasmtime (wasm-wasi) |
| `knox fmt [path]` | Stub: formatter not implemented |
| `knox add <name> --path <path>` | Stub: add local path dependency |

## Project layout

- **Root**: `README.md`, `LICENSE`, `.gitignore`, `rust-toolchain.toml`, `Cargo.toml` (workspace), `Justfile`
- **`.cursor/`**: Authoritative markdown docs (see table above)
- **Crates**: `knox_cli`, `knox_compiler`, `knox_syntax`, `knox_codegen_wasm`, `knox_pkg`, `knox_runtime`
- **Examples**: `examples/hello_world/hello.kx`, `examples/transfer_parse/transfer_parse.kx` (sample from spec)
- **VS Code**: `tools/vscode-knox/` (TextMate grammar + language config)

## Running tests

```bash
cargo test
```

## VS Code extension (development)

1. Open `tools/vscode-knox`
2. Run `npm install` (optional, only for packaging)
3. Package: `npm run package` → produces `knox-lang-0.1.0.vsix`
4. In VS Code: **Extensions: Install from VSIX...** and select that file

`.kx` files will then get syntax highlighting (keywords, strings, numbers, comments, types, functions).

## Installing the Cursor Extension

You can install the extension manually using the `.vsix` package.

### Method 1 — Install from the Cursor UI

1. Open Cursor.
2. Open the Extensions panel:
   - **macOS:** `Cmd + Shift + X`
   - **Windows/Linux:** `Ctrl + Shift + X`
3. Click the three dots menu in the top right of the Extensions panel.
4. Click **Install from VSIX**.
5. Select the `.vsix` file and install.
6. Restart Cursor if the extension does not appear immediately.

### Method 2 — Install via CLI

First install the Cursor CLI command:

1. Open Cursor → **Command Palette** → run:
   ```
   Shell Command: Install 'cursor' command in PATH
   ```

Then install the extension from the terminal:

```bash
cursor --install-extension your-extension.vsix
```

Verify installation:

```bash
cursor --list-extensions
```

## License

Apache-2.0. See [LICENSE](LICENSE).
