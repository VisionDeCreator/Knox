# Package manager

Knox uses a package and dependency system so you can split code into multiple modules and reuse other packages.

## Manifest: knox.toml

At the root of a project (e.g. `examples/hello_world/knox.toml`), you define the package and its dependencies:

```toml
[package]
name = "hello_world"
version = "0.1.0"

[dependencies]
# Local path dependency (MVP)
# http = { path = "../http" }
```

- **name** — Package name (used in import paths for external deps).
- **version** — Version string (e.g. semver).
- **[dependencies]** — Map of dependency name to spec. MVP supports only path deps: `name = { path = "../path" }`.

## Lockfile: knox.lock

A lockfile (e.g. `knox.lock`) records the exact resolved dependencies (paths, versions) so builds are reproducible. MVP may use a stub format; the important part is that resolution is deterministic.

## Internal modules

Modules that live in the same package are **internal**. They live under `src/`:

- `src/main.kx` → module (e.g. main entry).
- `src/auth/token.kx` → module `auth::token`.

No dependency entry is needed; the compiler finds them by path.

## External dependencies

When the **first segment** of an import matches a dependency name in `knox.toml`, the compiler treats it as an **external** package:

- In `knox.toml`: `http = { path = "../http" }`
- In code: `import http::Client` → resolve inside `../http` (e.g. `../http/src/lib.kx` or the appropriate file for `Client`).

The resolver loads the dependency’s manifest and source under its `path` and then resolves the rest of the import path (e.g. `Client`) inside that package.

## Commands (MVP)

- **knox build** — Build the current package (and use its dependencies for import resolution). Reads `knox.toml` and, if present, the lockfile.
- **knox add &lt;name&gt; --path &lt;path&gt;** — Stub in MVP; intended to add a path dependency to `knox.toml` and update the lockfile.

Future work may add a registry and version solving; for now the focus is local path deps and reproducible builds.
