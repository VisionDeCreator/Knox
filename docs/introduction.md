# Introduction to Knox

Knox is a programming language built to be **simple to read and teach**, **type safe**, **memory safe**, and **portable**. It aims to combine the flexibility of dynamic languages with the safety of Rust and the resource model of Move, while compiling to WebAssembly so you can run the same code on servers, in the browser, and (in the future) on blockchain.

## Vision

- **Simple** — Clear syntax, minimal magic, predictable behavior.
- **Safe** — Static typing, no null/undefined in the core language, explicit `dynamic` when you need an escape hatch.
- **Portable** — Wasm-first: run on the server (Wasmtime), in the browser (WebAssembly + JS), and eventually a deterministic blockchain subset.
- **Best of many worlds** — The ergonomics of JS, the safety of Rust, and (future) Move-style resources for blockchain.

## What Knox looks like

```kx
fn main() -> () {
  print("Hello, Knox!")
}
```

Functions, structs with private fields and generated accessors, pattern matching, and a module system with `import` and `pub` visibility. No null; use `Option<T>` and `Result<T, E>` instead.

## Who is Knox for?

- Learners who want a small, consistent language.
- Teams that want type safety and a single binary (Wasm) across environments.
- Future use in environments that need deterministic execution (e.g. blockchain).

This documentation covers the language basics, structs and accessors, modules and imports, the type system, error handling, targets, and the package manager.
