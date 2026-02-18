# Knox Package Manager (MVP)

## Manifest: knox.toml

- At project root (e.g. `examples/hello_world/knox.toml`).
- Schema (TOML):
  - `name` — package name (string).
  - `version` — semver string (e.g. `"0.1.0"`).
  - `[dependencies]` — table of name → dependency spec.
  - MVP: only local path deps: `mylib = { path = "../mylib" }`.

## Lockfile: knox.lock

- Generated/stub in MVP. Records resolved dependencies (name, path, version) so builds are reproducible.
- Format: TOML or custom; must list every direct and transitive dep with resolved path/version.

## Resolver (MVP)

- Only path dependencies. Resolution = expand paths, optionally normalize, and write lockfile.
- No registry, no version solving. Lockfile can be a stub that just records local paths.

## CLI

- `knox add <name> --path <path>` — Stub in MVP; intended to add a path dep to `knox.toml` and update lockfile.
- `knox build` — Reads manifest + lockfile; builds current package and deps (MVP: single package + local deps only).

## Module resolution

- **Internal:** Import path `auth::token` → file `src/auth/token.kx` under package root. Error if file missing.
- **External:** First segment = dependency name. Look up in `knox.toml` `[dependencies]`; path dep → resolve under `path` (e.g. `../http/src/lib.kx` or `../http/src/Client.kx` for `import http::Client`).
- **Visibility:** Only `pub` functions and structs (and generated accessors) can be imported. Fields are never directly accessible across modules.

## Implementation

- **knox_pkg** crate: parse `knox.toml` and `knox.lock` (serde + toml).
- **knox_compiler** `modules` module: `resolve_internal`, `resolve_external`, `resolve_import`, `file_to_mod_path`. Integrates with package dependency graph.
