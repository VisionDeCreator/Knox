# Knox Language Spec (MVP)

## File Extension

- Source files use the `.kx` extension.

## Lexical Rules

- **Identifiers:** `[a-zA-Z_][a-zA-Z0-9_]*`
- **Keywords:** `fn`, `let`, `mut`, `if`, `match`, `return`, `Ok`, `Err`, `Option`, `Result`, `dynamic`, `true`, `false`
- **Literals:** string (`"..."`), integer (decimal), boolean (`true`/`false`)
- **Symbols:** `( ) { } [ ] : , -> => . ? | _`
- **Operators:** `<`, `>`, `==`, `!=`, etc. (subset for MVP)
- **Comments:** `//` line comments, `/* */` block comments (optional in MVP)

## Functions

```text
fn name(param: Type, ...) -> ReturnType { body }
```

- Parameters are comma-separated with type annotations.
- Return type is required (use `()` for unit).
- Body is a block `{ ... }` of statements/expressions.

## Bindings

- `let name = expr;` — immutable binding.
- `let mut name = expr;` — mutable binding (explicit).

## Control Flow

- **if expression:** `if condition { block }` or `if condition { block } else { block }`
- **match expression:** `match expr { pattern => expr, _ => expr }`
- **return:** `return expr;` or `return;` (unit).

## Core Types (MVP)

- Primitives: `u64`, `int`, `string`, `bool`, `()`
- Generic/nominal: `Option<T>`, `Result<T, E>`
- Escape hatch: `dynamic` (explicit; no implicit dynamic)

## No null/undefined

- The core language has no null or undefined. Use `Option<T>` and `Result<T,E>`.

## Operators

- `?` — Propagate `Result` (early return on `Err`).
- Comparison: `<`, `>`, `<=`, `>=`, `==`, `!=`

## Pattern Matching (MVP minimal)

- Literal patterns, `_` (wildcard), and record destructuring for `dynamic` checks: `{ name: string, age: int }`.

## Sample (reference; do not change)

See `examples/transfer_parse/transfer_parse.kx` for the canonical sample of `transfer`, `parseUser`, `match` on `dynamic`, and `Result`/`?`.
