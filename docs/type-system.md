# Type system

Knox uses a static type system: every expression has a type, and the compiler checks that types line up (function arguments, returns, and field access).

## Primitive types

- **`int`** — Signed integer (e.g. literals `0`, `42`).
- **`u64`** — 64-bit unsigned integer.
- **`string`** — String (e.g. `"hello"`).
- **`bool`** — Boolean (`true`, `false`).
- **`()`** — Unit type (no value; used for “no return” or “nothing here”).

## Nominal types

- **`Option<T>`** — Optional value: `Some(expr)` or `None`. No `null`; use `Option` instead.
- **`Result<T, E>`** — Success or error: `Ok(expr)` or `Err(expr)`. Used for fallible operations; use `?` to propagate errors.

User-defined **structs** are also nominal types: once you define `struct User { ... }`, the type `User` is a distinct type.

## No null or undefined

The core language has no `null` or `undefined`. Use:

- `Option<T>` when a value might be absent.
- `Result<T, E>` when an operation can fail.

This avoids a whole class of null-reference bugs and makes “no value” explicit in the type.

## Dynamic

The type **`dynamic`** is an explicit escape hatch: a value whose type is not statically known. Use it only where needed (e.g. parsing JSON or interop). The type system “quarantines” dynamic: you must pattern-match or cast to a known type before using it as something else. There is no implicit dynamic; you must write `dynamic` in the type.

## Struct types and field access

Struct types are referred to by name (e.g. `User`). Field access (e.g. `receiver.field`) is type-checked: the receiver must be a struct type, and the field must exist on that struct. Getter/setter methods generated from `@pub(get, set)` are checked like any other function.

## Type checking

The compiler checks that:

- Function arguments match the parameter types.
- Return expressions match the function’s return type.
- Only valid operations are performed on each type (e.g. no field access on non-structs).
- Imported items exist and are public.

Errors are reported with file and line so you can fix them before running the program.
