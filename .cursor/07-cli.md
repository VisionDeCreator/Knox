# Knox CLI

## Binary

- Name: `knox` (from crate `knox_cli`).

## Commands

- **knox --help** — Show top-level help and list of commands.
- **knox new \<name\>** — Scaffold a new Knox project (directory, `knox.toml`, optional `src/main.kx` or single entry file). MVP: create directory and minimal manifest + hello stub.
- **knox build --target wasm-wasi \<path\>** — Build a Knox package/file at \<path\> for wasm-wasi. Output: `dist/<name>.wasm` (or alongside source; doc the default).
- **knox build --target wasm-web \<path\>** — Build for wasm-web; output Wasm + JS glue placeholder.
- **knox run \<file.kx\>** — Default target wasm-wasi. Compile the file (and its package if present), then run the resulting Wasm with Wasmtime. If Wasmtime is not installed, print a friendly error with install instructions.
- **knox fmt \<path\>** — Stub: print "TODO: formatter not implemented" or equivalent; command exists.

## Flags

- `--target wasm-wasi | wasm-web` for `build`.
- Global: `--help`, `-h`.

## Expected Outputs

- **build:** Writes `.wasm` (and for wasm-web, a small `.js`). Path: e.g. `dist/<project_name>.wasm` when building a package, or derived from input file name.
- **run:** Compilation output path (e.g. to temp or `dist/`), then `wasmtime run <wasm>`; stdout/stderr from the Wasm module.
- **new:** Creates directory and files; prints "Created project <name>".

## Wasmtime

- Must be on `PATH` for `knox run` (wasm-wasi). Detect via `which wasmtime` or similar; on failure show: "Wasmtime is required to run Knox programs. Install from https://wasmtime.dev"
