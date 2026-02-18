# Knox Language Spec (MVP)

## File Extension

- Source files use the `.kx` extension.

## Modules

- One file = one module.
- Source root = `src/`.
- File path defines module path: `src/user.kx` → module `user`; `src/auth/token.kx` → module `auth::token`.
- Import syntax: `import user`, `import user::User`, `import auth::token::{verify, sign}`, `import auth::token as token`.
- First segment of import: if it matches a dependency name in `knox.toml` → external package; otherwise → internal module.
- Only `pub` items can be imported across modules.

## Lexical Rules

- **Identifiers:** `[a-zA-Z_][a-zA-Z0-9_]*`
- **Keywords:** `fn`, `let`, `mut`, `if`, `match`, `return`, `struct`, `import`, `pub`, `as`, `Ok`, `Err`, `Option`, `Result`, `dynamic`, `true`, `false`
- **Symbols:** `( ) { } [ ] : , ; -> => . ? | _ @ ::`
- **Comments:** `//` line comments, `/* */` block comments

## Functions

```text
[pub] fn name(param: Type, ...) -> ReturnType { body }
```

- Optional `pub` for visibility across modules.
- Return type is required (use `()` for unit).

## Structs

```text
struct Name {
  field: Type [@pub(get)] [@pub(set)] [@pub(get, set)]
}
```

- Fields are private by default.
- `@pub(get)` generates a public getter: `pub fn field(self) -> Type`.
- `@pub(set)` generates a public setter: `pub fn setFieldName(mut self, v: Type) -> ()` (camelCase: `age` → `setAge`, `user_id` → `setUserId`).
- `@pub(get, set)` generates both.
- Direct external field access is forbidden. Conflicting manual method → compiler error.

## Statements and Semicolons

- Every statement must end with `;`. No implicit semicolons.
- Applies to: `let` declarations, assignments, expression statements, `return` statements, function calls used as statements.
- Inside `{ }`, each statement must end with `;`.
- **match**: arms do not take a semicolon after the arm expression (e.g. `0 => 10,`). The whole `match` statement must end with `;` when used as a statement.

## Variables and Mutability

- `let name = expr;` — immutable; `let mut name = expr;` — mutable. Optional type: `let name: Type = expr;`.
- Assignment: `name = expr;` only to `mut` variables. Through reference: `*ref = expr;`.

## Operators and Precedence

- **Unary:** `!` (bool), `-` (negation for int/u64).
- **Multiplicative:** `*`, `/`, `%`.
- **Additive:** `+`, `-` (+ supports string concat).
- **Comparison:** `<`, `<=`, `>`, `>=`, `==`, `!=` (return bool).
- **Logical:** `&&`, `||` (short-circuit). Precedence: `||` < `&&` < equality < comparison < additive < multiplicative < unary.

## Borrowing (MVP)

- Types: `&T`, `&mut T`. Create: `&x`, `&mut x` (only from mut for `&mut`). Deref: `*ref`.
- `*x = expr;` for assign-through-reference. MVP: local variables only; borrow checker is flow-insensitive, conservative.

## Bindings and Control Flow

- **if** / **match** / **return** as before. Match: literal and `_` patterns; must be exhaustive.

## Core Types

- Primitives: `u64`, `int`, `string`, `bool`, `()`
- Nominal: `Option<T>`, `Result<T, E>`, and user structs.
- Escape hatch: `dynamic`.

## No null/undefined

- Use `Option<T>` and `Result<T, E>`.

- `?` — propagate `Result`.

## Pattern Matching

- Literal patterns, `_`, record destructuring for `dynamic`: `{ name: string, age: int }`.
