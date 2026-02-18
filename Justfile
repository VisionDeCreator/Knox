# Knox monorepo commands
# Install: https://github.com/casey/just

default:
  just --list

# Build the full workspace
build:
  cargo build

# Build release
build-release:
  cargo build --release

# Run tests
test:
  cargo test

# Format code
fmt:
  cargo fmt

# Lint
clippy:
  cargo clippy -- -D warnings

# Check (no build artifacts)
check:
  cargo check

# Build CLI and run Hello World (requires wasmtime)
run-hello:
  cargo build -p knox_cli
  ./target/debug/knox run examples/hello_world/hello.kx

# Build wasm-wasi for hello world
build-hello-wasi:
  cargo build -p knox_cli
  ./target/debug/knox build --target wasm-wasi examples/hello_world/hello.kx

# CI: format, clippy, test
ci: fmt clippy test

# Package VS Code extension (from repo root)
vscode-package:
  cd tools/vscode-knox && npm run package
