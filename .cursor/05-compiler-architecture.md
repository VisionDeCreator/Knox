# Knox Compiler Architecture

## Pipeline (MVP)

1. **Lexer** — Source text → tokens (identifiers, keywords, literals, symbols). Handles comments.
2. **Parser** — Tokens → AST (functions, let, if, match, calls, literals). Diagnostics with file/line spans.
3. **AST** — Defined in `knox_syntax`: nodes, spans, diagnostics helpers.
4. **Type checker** — Resolve types for literals, function signatures, `print`, and nominal `Option`/`Result`. Emit errors with spans.
5. **Lowering** — (Minimal in MVP.) Map typed AST to a simple IR or directly to codegen inputs.
6. **Wasm codegen** — Emit Wasm module (e.g. wat/wasm) for `main` and builtins like `print` (WASI fd_write).

## Crates

- **knox_syntax** — Tokens, AST nodes, spans, diagnostic types. No I/O.
- **knox_compiler** — Lexer, parser, type checker, orchestration. Depends on knox_syntax, knox_codegen_wasm.
- **knox_codegen_wasm** — Wasm emitter (memory, funcs, exports). Uses wasm-encoder or similar.
- **knox_runtime** — Tiny runtime shims (WASI print stub; web placeholder). Linked or imported in generated Wasm.

## Diagnostics

- All phases report errors with file path and line/column (or span). No panics for user input; collect and print diagnostics.

## Hello World path

- `hello.kx` → lex → parse → typecheck (`main() -> ()`, `print(string)`) → codegen (export `main`, import `print` from runtime/WASI) → `dist/hello_world.wasm` → Wasmtime runs it.
