# Knox

**Knox** is a programming language built to be simple to read and teach, type safe, memory safe, flexible when needed, and able to run everywhere (server, browser, and a future blockchain subset). It aims for the best of JS-style flexibility, Rust-style safety, and (future) Move-style resources, and compiles **Wasm-first**: server runs via **Wasmtime**, browser via native WebAssembly + JS glue, with a deterministic blockchain subset planned later.

## What is Knox?

- **Simple** — Clear syntax, minimal magic, one file = one module, `export` and `import` for visibility.
- **Type safe** — Static types, no null/undefined; use `Option<T>` and `Result<T, E>`.
- **Memory safe** — No raw null; struct fields are private by default; accessors via `@pub(get, set)`.
- **Portable** — Compiles to WebAssembly; run on server (Wasmtime), in the browser (wasm-web), and later in a blockchain environment.

## Why Knox?

Knox is designed to combine simplicity, safety, and portability — without sacrificing performance.

### Safe by default: struct accessors

Fields are private by default. You generate exported getters and setters using `@pub(get, set)`; no boilerplate required.

```kx
export struct User {
  name: string,
  age: int @pub(get, set),
}

fn main() -> () {
  let mut user = User { name: "John", age: 20 };
  print(user.age());
  user.set_age(30);
  print(user.age());
}
```

- Fields remain private; no direct field access from outside the module.
- `@pub(get, set)` generates safe exported getter and setter methods.
- Setters require a `mut` binding, enforcing controlled mutation.
- No inheritance, no magic — just explicit structure.

### Simple module system

Modules are file-based and explicit. Visibility is controlled using `export`.

```kx
// product.kx
export struct Product {
  id: int @pub(get),
}

// main.kx
import product;

fn main() -> () {
  let p = product::Product { id: 1 };
  print(p.id());
}
```

- Each file is a module; the path under `src/` defines the module path.
- `export` controls cross-module visibility.
- `import product;` keeps dependencies explicit and predictable.
- `product::Product` and `p.id()` give clear, readable access.

### Wasm-first execution

Knox compiles ahead-of-time to WebAssembly:

- Consistent execution across platforms
- Server execution via Wasmtime
- Browser compatibility (via wasm-web target)
- Deterministic behavior

### Design philosophy

Knox aims to be:

- **Simple** enough to teach in universities.
- **Type safe** and **memory safe** by default.
- **Explicit** — no hidden behavior, no implicit semicolons.
- **Portable** through WebAssembly.

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

## Example runs

All commands below assume you are in the **root of the monorepo**.

### 1. Hello World

```bash
./target/debug/knox run examples/hello_world/hello.kx
```

Demonstrates: basic program structure, `fn main()`, and printing. Minimal Knox program.

### 2. Import demo

```bash
./target/debug/knox run examples/import_demo/src/main.kx
```

Demonstrates: file-based modules, `import` syntax, `export` visibility rules, and cross-module type usage.

### 3. Getters and setters

```bash
./target/debug/knox run examples/get_set/src/main.kx
```

Demonstrates: struct literals, automatic accessor generation via `@pub(get, set)`, safe mutation with `mut`, and method calls on structs.

## Project structure

| Area | Contents |
|------|----------|
| **Root** | `README.md`, `LICENSE`, `.gitignore`, `rust-toolchain.toml`, `Cargo.toml` (workspace), `Justfile` |
| **`.cursor/`** | Internal architecture and spec docs (source of truth for the compiler) |
| **`docs/`** | User-facing language and tooling documentation |
| **Crates** | `knox_cli`, `knox_compiler`, `knox_syntax`, `knox_codegen_wasm`, `knox_pkg`, `knox_runtime` |
| **Examples** | `examples/hello_world/`, `examples/import_demo/`, `examples/vars_ops/`, `examples/match/`, `examples/borrowing/` (borrowing parses/typechecks; codegen TODO), `examples/transfer_parse/` |
| **VS Code** | `tools/vscode-knox/` — TextMate grammar and language config for `.kx` |

## How modules work

- **One file = one module.** The path under `src/` defines the module path: `src/user.kx` → `user`, `src/auth/token.kx` → `auth::token`.
- **Imports:** `import user`, `import user::User`, `import auth::token::{verify, sign}`, `import http as h`. First segment: if it’s a dependency name in `knox.toml` → external package; otherwise → internal module under `src/`.
- **Visibility:** Only `export`ed items can be imported. Use `export struct`, `export fn`. Fields are never directly accessible across modules; use getters/setters.

See [docs/modules-and-imports.md](docs/modules-and-imports.md) for full detail.

## How structs and accessors work

- **Structs** have private fields by default. No direct external field access.
- **`@pub(get)`** — Generates an exported getter: `fn field(self) -> Type`.
- **`@pub(set)`** — Generates an exported setter in snake_case: `age` → `set_age`, `user_id` → `set_user_id`.
- **`@pub(get, set)`** — Generates both. All access is through methods; safe mutation only via setters.

See [docs/structs-and-accessors.md](docs/structs-and-accessors.md) for full detail.

## Language basics

- **Variables:** `let x = 1;`, `let mut y = 2;`, `y = y + 1;`. **Statements** require semicolons.
- **Struct fields** are separated by commas (trailing comma allowed); semicolons are not used inside struct bodies.
- **Operators:** Arithmetic (`+`, `-`, `*`, `/`, `%`), comparison (`<`, `<=`, `>`, `>=`, `==`, `!=`), logical (`&&`, `||`, `!`). `+` for int, u64, or string concat.
- **Match:** `match x { 0 => 10, 1 => 20, _ => 30 }`; literal and `_` patterns; exhaustive.
- **Borrowing:** `&T`, `&mut T`, `*ref`; `fn inc(x: &mut int) { *x = *x + 1; }`. Codegen for refs is not yet implemented (parse/typecheck work).

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

## Roadmap

Knox is under active development. The goal is to build a stable, predictable, Wasm-first language with strong fundamentals before expanding the surface area.

**Phase 1 — Core language** (in progress)

- Structs with private-by-default fields
- Automatic accessor generation via `@pub(get, set)`
- File-based module system (`import` / `export`)
- Strict semicolon enforcement
- Wasm (WASI) execution via Wasmtime
- Basic package structure
- CLI tool (`knox run`, `knox build`)

Focus: correctness, determinism, and architectural clarity.

**Phase 2 — Language expansion**

- Improved borrow semantics
- Pattern matching enhancements
- Async functions and concurrency primitives
- Standard library (collections, strings, utilities)
- Better error diagnostics
- Improved developer tooling (LSP, autocomplete, inline diagnostics)
- Stable package manager workflow

Focus: developer experience and ergonomics.

**Phase 3 — Multi-target runtime**

- wasm-web target (browser execution)
- Optimized server runtime
- Deterministic blockchain-safe subset
- Stable ABI for external integrations

Focus: portability and ecosystem growth.

**Long-term vision**

Knox aims to become:

- A language that is easy to teach
- A language that is safe by default
- A language that runs everywhere WebAssembly runs
- A language that avoids complexity unless it is absolutely necessary

The priority is stability over speed of feature addition.

## Running tests

```bash
cargo test
```

**Running tests (cargo test flags):**

- `--ignored` — run **only** tests that are marked `#[ignore]`. Do **not** combine with `--include-ignored`.
- `--include-ignored` — run **all** tests, including ignored ones. Do **not** combine with `--ignored`.

To run the end-to-end test that compiles `examples/accessors_generic` and runs it with Wasmtime (requires `wasmtime` on `PATH`):

```bash
cargo test -p knox_cli -- --ignored accessors_generic_run
```

To run all tests including ignored ones:

```bash
cargo test -p knox_cli -- --include-ignored
```

**Running examples from monorepo root:** All commands are intended to work when run from the monorepo root (the directory containing the workspace `Cargo.toml`). For example:

```bash
./target/debug/knox run examples/accessors_generic/src/main.kx
```

- **Package root** for a `.kx` file is the nearest ancestor directory containing `knox.toml`, or (for monorepo examples) the directory `examples/<name>/` when the file is under `examples/<name>/src/`.
- **Module root** is `<package_root>/src`; imports like `import product;` resolve to `<package_root>/src/product.kx`.
- Each example under `examples/<name>/` is a Knox package and should have its own `knox.toml` (e.g. `examples/accessors_generic/knox.toml`).

**Debugging path resolution:** Set `KNOX_DEBUG=1` to print cwd, monorepo root, entry path, package root, module root, wasm path, run cwd, and wasmtime location to stderr. Ensures Wasmtime is installed. The generated Wasm exports `_start` (WASI entry) and `memory`; the test `accessors_generic_wasm_has_start_and_memory_exports` verifies this.

## VS Code extension (development)

1. Open `tools/vscode-knox`.
2. Run `npm install` (optional, for packaging).
3. Run `npm run package` to produce a `.vsix` file.
4. In VS Code: **Extensions: Install from VSIX...** and select the file.

`.kx` files get syntax highlighting (keywords, strings, numbers, comments, structs, imports, `@pub`, etc.).

## Installing the Cursor Extension

You can install the extension manually using the `.vsix` package.

### Method 1 — Install from the Cursor UI

1. Open Cursor.
2. Open the Extensions panel:
   - **macOS:** `Cmd + Shift + X`
   - **Windows/Linux:** `Ctrl + Shift + X`
3. Click the **three dots** menu in the top right of the Extensions panel.
4. Click **Install from VSIX**.
5. Select the `.vsix` file and install.
6. Restart Cursor if the extension does not appear immediately.

### Method 2 — Install via CLI

First install the Cursor CLI command:

1. Open Cursor → **Command Palette** → run: **Shell Command: Install 'cursor' command in PATH**

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
