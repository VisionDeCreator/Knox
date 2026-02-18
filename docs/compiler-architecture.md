# Compiler architecture (user view)

This page gives a high-level, user-facing overview of how the Knox compiler works. For internal design details, see the `.cursor/` docs in the repo.

## Pipeline

1. **Lexer** — Reads source and produces a stream of tokens (keywords, identifiers, literals, symbols like `->`, `::`, `@`).
2. **Parser** — Builds an abstract syntax tree (AST): functions, structs, imports, expressions, statements.
3. **Desugar** — Expands struct fields annotated with `@pub(get, set)` into getter and setter function declarations. So by the time the rest of the compiler runs, those methods already exist in the AST.
4. **Module resolution** — (When building a package.) Resolves import paths to files: internal modules under `src/`, external ones from `knox.toml` dependencies. Ensures every imported module exists and can be loaded.
5. **Type checker** — Checks that types are correct everywhere: function calls, field access, return types, and that only `pub` items are imported. Reports errors with file and line.
6. **Wasm codegen** — Produces a WebAssembly module (e.g. for `main` and builtins like `print`). The current MVP focuses on getting a single entry point and simple calls working; struct layout and accessors can be lowered to functions and data in Wasm.

## What you can rely on

- **Determinism** — Same source and dependencies produce the same build.
- **Clear errors** — The compiler reports type and resolution errors with locations so you can fix them before running.
- **Wasm-first** — The main output is a `.wasm` file that you can run with Wasmtime (server) or load in the browser (wasm-web target).

For more on targets and runtimes, see [Targets](targets.md). For the language’s type and safety rules, see [Type system](type-system.md) and [Error handling](error-handling.md).
