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

## Implementation

- **knox_pkg** crate: parse `knox.toml` and `knox.lock` (serde + toml). Build graph: current package + local deps. Lockfile generation as stub (record resolved local deps).
