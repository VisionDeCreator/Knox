# Knox Compiler Architecture

## Pipeline

1. **Lexer** — Source → tokens (identifiers, keywords, literals, symbols including `@`, `::`, `import`, `struct`, `pub`).
2. **Parser** — Tokens → AST (functions, structs, imports, let, if, match, field access, etc.).
3. **Desugar** — Expand `@pub(get, set)` on struct fields into generated getter/setter `FnDecl` items. Runs after parse, before typecheck.
4. **AST** — Defined in `knox_syntax`: `Root`, `Item` (Fn, Struct, Import), `FnDecl` (with `pub_vis`), `StructDecl`, `StructField`, `FieldAttrs`, `ImportDecl`, `Expr::FieldAccess`, etc.
5. **Module loader** — (Optional.) For packages: resolve internal modules (`src/a/b.kx` → `a::b`) and external deps from `knox.toml`; build module graph.
6. **Type checker** — Resolve types, check function/struct/field access, enforce visibility (only `pub` importable). Requires struct and function maps.
7. **Wasm codegen** — Emit Wasm for `main` and builtins (e.g. `print` via WASI). Struct/accessor codegen can be minimal in MVP.

## Crates

- **knox_syntax** — Tokens, AST, spans, diagnostics.
- **knox_compiler** — Lexer, parser, desugar, modules, type checker, orchestration.
- **knox_codegen_wasm** — Wasm emitter.
- **knox_pkg** — Manifest and lockfile parsing.
- **knox_runtime** — Tiny runtime shims.

## Diagnostics

- All phases report errors with file/line/span. No panics for user input.

## Hello World path

- Single file → lex → parse → desugar → typecheck → codegen → Wasm → Wasmtime.
