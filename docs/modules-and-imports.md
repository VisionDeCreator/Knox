# Modules and imports

## One file = one module

Each `.kx` file is one module. The module path is determined by its path under the **source root** (`src/`):

- `src/user.kx` → module `user`
- `src/auth/token.kx` → module `auth::token`

So the file path and the module path match.

## Import syntax

Import a whole module, specific items, or use an alias:

```kx
import user
import user::User
import auth::token::verify
import auth::token::{verify, sign}
import auth::token as token
import http as h
```

- **Whole module:** `import user` — brings the module `user` into scope (how you refer to it depends on the implementation; e.g. qualified `user::item` or alias).
- **Single item:** `import user::User` — imports the type (or value) `User` from module `user`.
- **Multiple items:** `import auth::token::{verify, sign}` — imports `verify` and `sign` from `auth::token`.
- **Alias:** `import auth::token as token` or `import http as h` — imports the module under a different name.

## Internal vs external

The **first segment** of the import path decides whether the module is internal or external:

- **Internal** — Not a dependency name in `knox.toml`. Resolved under the current package’s `src/`:
  - `auth::token` → `src/auth/token.kx`
- **External** — First segment is a dependency name in `knox.toml`. Resolved to that dependency’s root (e.g. its `src/lib.kx` or a path inside it):
  - `http` in deps → resolve `import http::Client` from the `http` package (e.g. `path = "../http"` → `../http/src/...`).

If the resolved file is missing, the compiler reports “module not found”.

## Visibility: `pub`

Only **public** items can be imported from another module:

- `pub fn` — function is importable.
- Structs and their generated accessors (from `@pub(get, set)`) are public when the struct is public (or when the accessors are generated as `pub`, which they are by default).
- Non-`pub` items are only visible in the same module.

Fields are never directly visible across modules; use accessors.

## Cycles

The MVP allows cycles in the module graph (e.g. A imports B, B imports A). The implementation must avoid infinite resolution loops when resolving imports.

## Summary

- File path under `src/` = module path.
- Use `import` to bring in other modules or specific items; use `as` to alias.
- Internal = under `src/`; external = dependency from `knox.toml`.
- Only `pub` items can be imported; fields are never directly accessible across modules.
