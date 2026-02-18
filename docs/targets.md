# Targets

Knox compiles to **WebAssembly (Wasm)** first. The same language and type system target different runtimes depending on where you run the code.

## wasm-wasi

- **Use case:** Servers, CLI tools, and any environment where you run WebAssembly with a WASI runtime.
- **Runtime:** [Wasmtime](https://wasmtime.dev) (AOT). Required for `knox run`.
- **Capabilities:** WASI APIs (e.g. stdout, stdin, filesystem, environment). The builtin `print` maps to WASI (e.g. writing to stdout).
- **Output:** A single `.wasm` module. No JavaScript.

This is the default target for `knox run` and the main way to run Knox programs today.

## wasm-web

- **Use case:** Browsers and other WebAssembly-on-the-web environments.
- **Runtime:** Native WebAssembly in the browser, plus a small JS glue layer to load and call the module.
- **Capabilities:** MVP does not add DOM or web-specific APIs; the glue is a minimal loader. Future work can add console logging or other host bindings.
- **Output:** `.wasm` plus a small `.js` loader.

## Future: blockchain subset

- A **deterministic subset** of Knox is planned for blockchain or other deterministic environments, inspired by Move’s resource model.
- That would be a separate target with restricted APIs and execution guarantees.

## Capability gating (future)

Targets may restrict which APIs are available (e.g. no filesystem in wasm-web). The compiler and standard library can be extended to gate APIs by target so that only allowed operations are used for each environment.
