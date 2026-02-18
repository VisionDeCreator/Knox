# Knox

**Knox** is a programming language built to be simple to read and teach, type safe, memory safe, flexible when needed, and able to run everywhere (server, browser, and a future blockchain subset). It aims for the best of JS-style flexibility, Rust-style safety, and (future) Move-style resources, and compiles **Wasm-first**: server runs via **Wasmtime**, browser via native WebAssembly + JS glue, with a deterministic blockchain subset planned later.

## What is Knox?

- **Simple** — Clear syntax, minimal magic, one file = one module, `pub` and `import` for visibility.
- **Type safe** — Static types, no null/undefined; use `Option<T>` and `Result<T, E>`.
- **Memory safe** — No raw null; struct fields are private by default; accessors via `@pub(get, set)`.
- **Portable** — Compiles to WebAssembly; run on server (Wasmtime), in the browser (wasm-web), and later in a blockchain environment.

## Installation requirements

- **Rust** (stable) — [rustup](https://rustup.rs). The repo uses `rust-toolchain.toml` (stable).
- **Wasmtime** — Required to run Knox programs. [Install from wasmtime.dev](https://wasmtime.dev).
- **Node + VS Code** — Optional; only for developing or packaging the [VS Code extension](tools/vscode-knox/) (syntax highlighting).

## How to build Knox

```bash
cargo build -p knox_cli
```

The `knox` binary is at `target/debug/knox` (or `target/release/knox` for release).

## How to run Hello World

```bash
./target/debug/knox run examples/hello_world/hello.kx
```

Expected output: **Hello, Knox!**

## Project structure

| Area | Contents |
|------|----------|
| **Root** | `README.md`, `LICENSE`, `.gitignore`, `rust-toolchain.toml`, `Cargo.toml` (workspace), `Justfile` |
| **`.cursor/`** | Internal architecture and spec docs (source of truth for the compiler) |
| **`docs/`** | User-facing language and tooling documentation |
| **Crates** | `knox_cli`, `knox_compiler`, `knox_syntax`, `knox_codegen_wasm`, `knox_pkg`, `knox_runtime` |
| **Examples** | `examples/hello_world/`, `examples/import_demo/` (import syntax), `examples/transfer_parse/` |
| **VS Code** | `tools/vscode-knox/` — TextMate grammar and language config for `.kx` |

## How modules work

- **One file = one module.** The path under `src/` defines the module path: `src/user.kx` → `user`, `src/auth/token.kx` → `auth::token`.
- **Imports:** `import user`, `import user::User`, `import auth::token::{verify, sign}`, `import http as h`. First segment: if it’s a dependency name in `knox.toml` → external package; otherwise → internal module under `src/`.
- **Visibility:** Only `pub` items can be imported. Fields are never directly accessible across modules; use getters/setters.

See [docs/modules-and-imports.md](docs/modules-and-imports.md) for full detail.

## How structs and accessors work

- **Structs** have private fields by default. No direct external field access.
- **`@pub(get)`** — Generates a public getter: `pub fn field(self) -> Type`.
- **`@pub(set)`** — Generates a public setter with camelCase name: `age` → `setAge`, `user_id` → `setUserId`.
- **`@pub(get, set)`** — Generates both. All access is through methods; safe mutation only via setters.

See [docs/structs-and-accessors.md](docs/structs-and-accessors.md) for full detail.

## Targets

| Target | Use | Runtime | Output |
|--------|-----|---------|--------|
| **wasm-wasi** | Server / CLI | Wasmtime | Single `.wasm` |
| **wasm-web** | Browser | Native Wasm + JS glue | `.wasm` + small `.js` loader |
| **Blockchain** | (Future) Deterministic subset | TBD | TBD |

See [docs/targets.md](docs/targets.md) for more.

## Commands

| Command | Description |
|--------|-------------|
| `knox new <name>` | Create a new Knox project (directory, `knox.toml`, stub `main.kx`) |
| `knox build --target wasm-wasi <path>` | Build a `.kx` file or package to Wasm |
| `knox run <file.kx>` | Compile and run with Wasmtime (wasm-wasi) |
| `knox fmt [path]` | Stub: formatter not implemented |
| `knox add <name> --path <path>` | Stub: add local path dependency |

## Documentation

- **User-facing:** [docs/](docs/) — introduction, getting started, language basics, structs, modules, type system, error handling, targets, package manager, compiler overview.
- **Internal / specs:** [.cursor/](.cursor/) — vision, language spec, type system, memory model, targets, compiler architecture, package manager, CLI, editor tools, roadmap.

## Running tests

```bash
cargo test
```

## VS Code extension (development)

1. Open `tools/vscode-knox`.
2. Run `npm install` (optional, for packaging).
3. Run `npm run package` to produce a `.vsix` file.
4. In VS Code: **Extensions: Install from VSIX...** and select the file.

`.kx` files get syntax highlighting (keywords, strings, numbers, comments, structs, imports, `@pub`, etc.).

## License

Apache-2.0. See [LICENSE](LICENSE).
