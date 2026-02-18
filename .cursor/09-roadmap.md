# Knox Roadmap

## Phase 1: MVP Hello World (current)

- [x] Monorepo layout, docs, workspace.
- [x] Lexer, parser, AST, minimal type checker, Wasm codegen for `main` + `print`.
- [x] CLI: `knox new`, `knox build`, `knox run`, `knox fmt` (stub).
- [x] Minimal stdlib: `print` (WASI).
- [x] Package manager: manifest + lockfile parsing, local path deps, stub lockfile generation; `knox add` stub.
- [x] VS Code: TextMate grammar + language config.
- [x] `knox run examples/hello_world/hello.kx` works with Wasmtime.

## Phase 2: Stdlib and expressions

- Expand stdlib (more types, basic I/O).
- Full expression coverage: match, if, let, operators.
- Option/Result in codegen and runtime.

## Phase 3: Packages

- Multi-package builds using lockfile.
- `knox add` implementation.
- Optional registry design.

## Phase 4: Targets and capability gating

- wasm-web: real JS glue, optional console.log.
- Target-gated APIs in compiler.

## Phase 5: Blockchain subset (future)

- Deterministic subset; Move-inspired resources.
- New target and runtime constraints.
