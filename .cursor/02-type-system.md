# Knox Type System (MVP)

## Core Types

- **Primitives:** `u64`, `int`, `string`, `bool`, `()` (unit).
- **Nominal types:** `Option<T>`, `Result<T, E>` — type names only in MVP; full generics later.

## No null/undefined

- There is no `null` or `undefined` in the type system.
- Optional values use `Option<T>`; fallible operations use `Result<T, E>`.

## Type Rules (MVP)

- **Literals:** Integer literals → `int` (or `u64` when context demands). String literals → `string`. Booleans → `bool`.
- **Function signatures:** Parameter and return types are checked. Call sites must match parameter types; return type is used for flow.
- **Builtins:** `print(s: string) -> ()` is the only required builtin for Hello World.
- **Option/Result:** Treated as nominal types. Constructor forms: `Ok(expr)`, `Err(expr)`, `Some(expr)`, `None` (with type context). Destructuring via `match` or `?` (Result only).

## dynamic quarantining

- `dynamic` is an explicit type. Values of type `dynamic` can only be:
  - Assigned from/to other `dynamic` or from JSON/interop.
  - Pattern-matched with record patterns (e.g. `{ name: string, age: int }`) to extract typed values.
- Typed code cannot implicitly receive `dynamic` without an explicit annotation or pattern match. This keeps dynamic use localized.

## MVP Scope

- No inference beyond literal types and simple call matching.
- No generics implementation; `Option<T>` and `Result<T,E>` are special-cased where needed for typecheck.
