# Knox Targets

## wasm-wasi

- **Purpose:** Server and CLI execution.
- **Runtime:** Wasmtime (AOT). Required for `knox run`.
- **Capabilities:** WASI snapshot (e.g. stdout, stdin, env, filesystem as per WASI). `print` maps to WASI fd_write to stdout.
- **Output:** Single `.wasm` module; no JS.

## wasm-web

- **Purpose:** Browser execution.
- **Runtime:** Native WebAssembly + small JS glue (instantiate, linear memory, exports).
- **Capabilities:** MVP has no DOM or web APIs; glue is a placeholder for future `console.log` or host bindings.
- **Output:** `.wasm` + minimal `.js` loader.

## Capability gating (future)

- Targets may restrict which APIs are available (e.g. no filesystem in wasm-web).
- Stdlib and builtins can be gated by target in the compiler.

## Blockchain (future)

- Deterministic subset; Move-inspired resources. Out of scope for MVP.
