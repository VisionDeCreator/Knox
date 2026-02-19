# Structs and accessors

## Structs

Structs define named aggregates with typed fields. All fields are **private** by default; external code cannot access them directly. **Struct fields are separated by commas** (a trailing comma before `}` is allowed). Statements elsewhere in Knox still require semicolons.

```kx
struct User {
  name: string,
  age: int,
  email: string,
}
```

## Field accessors: `@pub(get, set)`

To expose safe read or write access, annotate a field with `@pub(get)`, `@pub(set)`, or `@pub(get, set)`. The compiler generates exported getter and/or setter methods.

### `@pub(get)`

Generates an exported getter with the same name as the field:

```kx
struct User {
  name: string,
  age: int @pub(get),
}
```

Generated: `fn age(self) -> int` (returns the value of `age`).

### `@pub(set)`

Generates an exported setter with a snake_case name prefixed by `set_`:

- `age` → `set_age`
- `user_id` → `set_user_id`

```kx
struct User {
  name: string,
  age: int @pub(set),
}
```

Generated: `fn set_age(mut self, v: int) -> ()`.

### `@pub(get, set)`

Generates both getter and setter:

```kx
struct User {
  name: string,
  age: int @pub(get, set),
  email: string @pub(get),
}
```

## Rules

- **All fields remain private.** No direct external field access; only the generated (or manually defined) methods are visible.
- **Setter names** are always snake_case with a `set_` prefix (e.g. `set_age`, `set_user_id`).
- If you define a method that conflicts with a generated one (same name and signature), the compiler reports an error.
- Generated methods are exported (visible across modules when the struct is exported).

## How it works

The compiler runs a **desugaring** pass after parsing: for each struct field with `@pub(get)` or `@pub(set)`, it adds the corresponding function declarations to the module. So you get a single, consistent way to expose data (methods) instead of public fields.

## Safety

Accessors keep the struct representation encapsulated: callers use methods instead of raw field access, so you can change layout or add validation later without breaking the public API.
