# Knox Vision

## Goals

Knox is a new programming language designed to be the best of all worlds:

- **Simple to read and teach** — Clear syntax, minimal magic, predictable semantics.
- **Type safe** — Static typing with explicit `dynamic` when escape hatches are needed.
- **Memory safe** — No null/undefined in the core language; ownership/borrowing direction defined for future.
- **Flexible when needed** — `Option<T>`, `Result<T,E>`, pattern matching, and explicit `dynamic` for interop.
- **Run everywhere** — Server (WASI), browser (WebAssembly + JS glue), and a future deterministic blockchain subset (Move-inspired resources).

**Compilation strategy:** Wasm-first with AOT on server via Wasmtime. Browser runs via native Wasm + JS glue. Blockchain is a future deterministic subset.

## Non-Goals (MVP and near-term)

- No full ownership/borrowing implementation in MVP (direction only).
- No registry-based package manager in MVP (local path deps only).
- No DOM or browser-specific APIs in MVP.
- No blockchain target in MVP.

## Philosophy

- **Deterministic and runnable locally** — No hidden dependencies; toolchain works offline.
- **Docs as source of truth** — Architecture and specs live in `.cursor/` and are referenced from the root README.
- **Vertical slice first** — MVP delivers a working Hello World end-to-end; then we expand stdlib, packages, and targets.
