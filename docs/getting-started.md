# Getting Started

## Install requirements

1. **Rust** (stable) — [rustup](https://rustup.rs). The repo uses `rust-toolchain.toml` (stable).
2. **Wasmtime** — Required to run compiled Knox programs. [Install Wasmtime](https://wasmtime.dev).
3. **Node and VS Code** — Optional; only needed if you develop or package the Knox VS Code extension.

## Build Knox

From the repo root:

```bash
cargo build -p knox_cli
```

The `knox` binary is produced at `target/debug/knox` (or `target/release/knox` for release builds).

## Run Hello World

```bash
./target/debug/knox run examples/hello_world/hello.kx
```

You should see: `Hello, Knox!`

## Create a new project

```bash
knox new myapp
cd myapp
```

This creates a directory with `knox.toml` and a `main.kx` stub. You can then run:

```bash
knox run main.kx
```

## Build to WebAssembly

To build a single file or package to a `.wasm` file:

```bash
knox build --target wasm-wasi examples/hello_world/hello.kx
```

Output is written under `dist/` (or next to the input for a single file).

## Next steps

- [Language basics](language-basics.md) — Functions, bindings, types, control flow.
- [Structs and accessors](structs-and-accessors.md) — Structs and `@pub(get, set)`.
- [Modules and imports](modules-and-imports.md) — File-as-module and `import`/`pub`.
