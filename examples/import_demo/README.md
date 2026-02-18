# import_demo

This example shows how to use the **import** feature in Knox.

## Layout

- `src/main.kx` — Entry file that imports `greet` and defines `main`.
- `src/greet.kx` — Module `greet` with a public function `greet()`.

## Syntax

- **Whole module:** `import greet` brings the module `greet` into scope (path: `src/greet.kx`).
- **Single item:** `import greet::greet` would import just the `greet` function.
- **Multiple items:** `import greet::{greet, other}` for multiple names.
- **Alias:** `import greet as g` to use a different name.

## Running

Single-file run (compiles only `main.kx`; import is parsed but not resolved yet):

```bash
knox run src/main.kx
```

Output: `Hello, Knox! (import_demo)`

Full package build (loads `src/` and resolves imports) will be supported in a future CLI update.
