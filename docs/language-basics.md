# Language basics

## Files and extensions

Knox source files use the `.kx` extension. One file is one module; the module path is derived from the path under `src/` (see [Modules and imports](modules-and-imports.md)).

## Functions

Define a function with `fn`, parameters with types, and a required return type:

```kx
fn add(a: int, b: int) -> int {
  return a + b
}

fn main() -> () {
  print("Hi")
}
```

Use `()` for the unit type when the function returns no value. Mark a function as importable from other modules with `pub`:

```kx
pub fn greet(name: string) -> string {
  "Hello, " + name
}
```

## Bindings

- `let x = expr` — immutable binding.
- `let mut x = expr` — mutable binding (mutation allowed).

## Types

- **Primitives:** `int`, `u64`, `string`, `bool`, `()` (unit).
- **Nominal:** `Option<T>`, `Result<T, E>`, and user-defined structs.
- **Escape hatch:** `dynamic` (explicit; use for interop or when you need to defer typing).

There is no `null` or `undefined`; use `Option<T>` and `Result<T, E>`.

## Control flow

- **if:** `if condition { block }` or `if condition { block } else { block }`.
- **match:** `match expr { pattern => expr, _ => expr }`.
- **return:** `return expr;` or `return;` for unit.

## Operators

- `?` — Propagate `Result` (early return on `Err`).
- Comparisons: `<`, `>`, `<=`, `>=`, `==`, `!=`.

## Comments

- Line: `// ...`
- Block: `/* ... */`

## Safety

Knox is designed to be type safe and memory safe: types are checked at compile time, and the core language avoids null and implicit dynamic typing.
