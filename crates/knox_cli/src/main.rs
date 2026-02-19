//! Knox CLI: build, run, new, fmt (stub).

use clap::{Parser, Subcommand};
use knox_compiler::print_diagnostics;
use knox_syntax::span::FileId;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Monorepo root: directory containing Cargo.toml with [workspace]. Walk up from `start`.
fn find_monorepo_root(mut start: &Path) -> Option<PathBuf> {
    loop {
        let cargo = start.join("Cargo.toml");
        if cargo.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo) {
                if content.contains("[workspace]") {
                    return Some(start.to_path_buf());
                }
            }
        }
        start = start.parent()?;
    }
}

/// Package root for a .kx file: nearest ancestor with knox.toml, or examples/<name>/ when under examples/<name>/src/.
fn find_package_root(entry_path: &Path) -> Option<PathBuf> {
    entry_path
        .ancestors()
        .find(|p| p.join("knox.toml").exists())
        .map(PathBuf::from)
        .or_else(|| {
            let s = entry_path.to_string_lossy();
            if s.contains("examples") && s.contains("src") {
                entry_path.ancestors().find(|a| {
                    let src = a.join("src");
                    entry_path.starts_with(&src)
                }).map(PathBuf::from)
            } else {
                None
            }
        })
}

#[derive(Parser)]
#[command(name = "knox")]
#[command(about = "Knox programming language toolchain")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new Knox project
    New { name: String },
    /// Build a Knox file or package
    Build {
        #[arg(long, default_value = "wasm-wasi")]
        target: String,
        path: PathBuf,
    },
    /// Compile and run a Knox file (wasm-wasi via Wasmtime)
    Run { path: PathBuf },
    /// Format Knox source (TODO: not implemented)
    Fmt {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Add a dependency (TODO: stub)
    Add {
        name: String,
        #[arg(long)]
        path: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Commands::New { name } => cmd_new(&name),
        Commands::Build { target, path } => cmd_build(&target, &path),
        Commands::Run { path } => cmd_run(&path),
        Commands::Fmt { path } => cmd_fmt(&path),
        Commands::Add { name, path } => cmd_add(&name, path.as_deref()),
    }
}

fn cmd_new(name: &str) -> Result<(), String> {
    let dir = PathBuf::from(name);
    if dir.exists() {
        return Err(format!("Directory already exists: {}", name));
    }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let manifest = format!(
        r#"[package]
name = "{}"
version = "0.1.0"

[dependencies]
"#,
        name
    );
    std::fs::write(dir.join("knox.toml"), manifest).map_err(|e| e.to_string())?;
    let main_kx = r#"fn main() -> () {
  print("Hello, Knox!")
}
"#;
    std::fs::write(dir.join("main.kx"), main_kx).map_err(|e| e.to_string())?;
    println!("Created project {}", name);
    Ok(())
}

/// Resolve project root (directory containing dist/) and compile path (main.kx to compile).
/// Used by both build and run so output is always project_root/dist/main.wasm.
fn resolve_compile_and_project(path: &Path) -> Result<(PathBuf, PathBuf), String> {
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    if path.is_dir() {
        let main_path = path.join("main.kx");
        let main_path = if main_path.exists() {
            main_path
        } else {
            path.join("src").join("main.kx")
        };
        if !main_path.exists() {
            return Err("No main.kx or src/main.kx found in directory".into());
        }
        Ok((main_path.canonicalize().unwrap_or(main_path), path))
    } else if path.extension().map(|e| e == "kx").unwrap_or(false) {
        let project_root = find_package_root(&path)
            .unwrap_or_else(|| path.parent().unwrap_or(Path::new(".")).to_path_buf());
        Ok((path.clone(), project_root))
    } else {
        Err("Expected .kx file or project directory".into())
    }
}

fn cmd_build(_target: &str, path: &Path) -> Result<(), String> {
    let (compile_path, project_root) = resolve_compile_and_project(path)?;
    let out_path = project_root.join("dist").join("main.wasm");

    let source_for_diags = std::fs::read_to_string(&compile_path).unwrap_or_default();
    let wasm = knox_compiler::compile_file(&compile_path).map_err(|diags| {
        print_diagnostics(&source_for_diags, FileId::new(0), &diags);
        "Compilation failed".to_string()
    })?;

    std::fs::create_dir_all(out_path.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(&out_path, wasm).map_err(|e| e.to_string())?;
    println!("Wrote {}", out_path.display());
    Ok(())
}

fn cmd_run(path: &Path) -> Result<(), String> {
    let (compile_path, project_root) = resolve_compile_and_project(path)?;
    let wasm_path = project_root.join("dist").join("main.wasm");

    let debug = std::env::var("KNOX_DEBUG").is_ok();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let monorepo_root = find_monorepo_root(&cwd);
    if debug {
        eprintln!("[KNOX_DEBUG] cwd: {}", cwd.display());
        eprintln!("[KNOX_DEBUG] monorepo_root: {}", monorepo_root.as_deref().map(|p| p.display().to_string()).unwrap_or_else(|| "(none)".into()));
        eprintln!("[KNOX_DEBUG] compile_path (entry): {}", compile_path.display());
        eprintln!("[KNOX_DEBUG] project_root: {}", project_root.display());
        eprintln!("[KNOX_DEBUG] module_root: {}", project_root.join("src").display());
    }

    let source = std::fs::read_to_string(&compile_path).map_err(|e| e.to_string())?;
    let wasm = match knox_compiler::compile_file(&compile_path) {
        Ok(w) => w,
        Err(diags) => {
            print_diagnostics(&source, FileId::new(0), &diags);
            return Err("Compilation failed".to_string());
        }
    };

    std::fs::create_dir_all(wasm_path.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(&wasm_path, &wasm).map_err(|e| e.to_string())?;
    let wasm_path_abs = wasm_path.canonicalize().unwrap_or(wasm_path.clone());
    if debug {
        eprintln!("[KNOX_DEBUG] wasm_path: {}", wasm_path_abs.display());
        eprintln!("[KNOX_DEBUG] wasm size: {} bytes", wasm.len());
    }

    let wasmtime = which::which("wasmtime").map_err(|_| {
        "Wasmtime is required to run Knox programs. Install from https://wasmtime.dev".to_string()
    })?;
    if debug {
        eprintln!("[KNOX_DEBUG] wasmtime: {}", wasmtime.display());
        eprintln!("[KNOX_DEBUG] run_cwd: {}", project_root.display());
    }

    // Run wasmtime; capture stdout/stderr then write so program output is visible.
    let output = Command::new(&wasmtime)
        .arg("run")
        .arg(&wasm_path_abs)
        .current_dir(&project_root)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| e.to_string())?;
    let status = output.status;

    // Write guest program output to stderr so it appears in the terminal.
    if !output.stdout.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        let _ = std::io::Write::write_all(&mut std::io::stderr(), &output.stderr);
        let _ = std::io::Write::flush(&mut std::io::stderr());
    }

    if !status.success() {
        return Err(format!("wasmtime exited with {}", status));
    }
    Ok(())
}

fn cmd_fmt(_path: &Path) -> Result<(), String> {
    eprintln!("TODO: formatter not implemented");
    Ok(())
}

fn cmd_add(name: &str, _path: Option<&std::path::Path>) -> Result<(), String> {
    eprintln!(
        "TODO: knox add {} (package manager add not implemented)",
        name
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Workspace root (knox_cli is at crates/knox_cli).
    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn wasm_has_start_and_memory(wasm: &[u8]) -> bool {
        let mut has_start = false;
        let mut has_memory = false;
        for payload in wasmparser::Parser::new(0).parse_all(wasm) {
            let payload = payload.expect("parse wasm");
            if let wasmparser::Payload::ExportSection(reader) = payload {
                for export in reader {
                    let export = export.expect("export");
                    match export.name {
                        "_start" => has_start = true,
                        "memory" => has_memory = true,
                        _ => {}
                    }
                }
            }
        }
        has_start && has_memory
    }

    #[test]
    fn accessors_generic_wasm_has_start_and_memory_exports() {
        let ws = workspace_root();
        let main_kx = ws.join("examples/accessors_generic/src/main.kx");
        if !main_kx.exists() {
            eprintln!("skip: {} not found", main_kx.display());
            return;
        }
        let _source = std::fs::read_to_string(&main_kx).expect("read main.kx");
        let wasm = knox_compiler::compile_file(&main_kx).expect("compile");
        assert!(!wasm.is_empty(), "wasm must be non-empty");
        assert!(
            wasm_has_start_and_memory(&wasm),
            "wasm must export _start and memory for WASI"
        );
    }

    #[test]
    fn print_one_compiles_and_has_wasi_exports() {
        let tmp = std::env::temp_dir().join("knox_test_print_one");
        let _ = std::fs::create_dir(&tmp);
        let main_kx = tmp.join("main.kx");
        std::fs::write(
            &main_kx,
            "fn main() -> () { print(1); }",
        )
        .expect("write main.kx");
        let wasm = knox_compiler::compile_file(&main_kx).expect("compile");
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(!wasm.is_empty());
        assert!(
            wasm_has_start_and_memory(&wasm),
            "print(1) wasm must export _start and memory"
        );
    }

    #[test]
    fn hello_world_compiles_and_has_wasi_exports() {
        let tmp = std::env::temp_dir().join("knox_test_hello");
        let _ = std::fs::create_dir(&tmp);
        let main_kx = tmp.join("main.kx");
        std::fs::write(
            &main_kx,
            r#"fn main() -> () { print("Hello, World!"); }"#,
        )
        .expect("write main.kx");
        let wasm = knox_compiler::compile_file(&main_kx).expect("compile");
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(!wasm.is_empty());
        assert!(
            wasm_has_start_and_memory(&wasm),
            "Hello World wasm must export _start and memory"
        );
    }

    /// Runs the exact command the user runs: `knox run examples/accessors_generic/src/main.kx` from monorepo root.
    #[test]
    #[ignore = "requires wasmtime on PATH; run with: cargo test -p knox_cli -- --ignored accessors_generic_run"]
    fn accessors_generic_run_produces_expected_stdout() {
        let ws = workspace_root();
        let main_kx = ws.join("examples/accessors_generic/src/main.kx");
        if !main_kx.exists() {
            eprintln!("skip: {} not found", main_kx.display());
            return;
        }
        if which::which("wasmtime").is_err() {
            eprintln!("skip: wasmtime not on PATH");
            return;
        }
        // Run the knox binary exactly as the user does (same cwd and args).
        let bin = match std::env::var("CARGO_BIN_EXE_knox") {
            Ok(p) => p,
            Err(_) => {
                eprintln!("skip: CARGO_BIN_EXE_knox not set (run with: cargo test -p knox_cli -- --ignored)");
                return;
            }
        };
        let out = Command::new(&bin)
            .arg("run")
            .arg("examples/accessors_generic/src/main.kx")
            .current_dir(&ws)
            .output()
            .expect("run knox");
        let stdout = String::from_utf8_lossy(&out.stdout);
        if std::env::var("KNOX_DEBUG").is_ok() {
            eprintln!("[KNOX_DEBUG] test stdout length: {} bytes", stdout.len());
            eprintln!("[KNOX_DEBUG] test stdout (repr): {:?}", stdout);
        }
        assert!(
            out.status.success(),
            "knox run must exit 0; stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        // Regression: int print must emit ASCII digits, not raw bytes or NULs.
        assert!(
            !stdout.chars().all(|c| c == '\0'),
            "stdout must not be only NUL bytes; got {} bytes; stdout: {:?}",
            stdout.len(),
            stdout
        );
        assert!(
            stdout.contains("1"),
            "stdout should contain '1' (id); got: {:?}",
            stdout
        );
        assert!(
            stdout.contains("10"),
            "stdout should contain '10' (price); got: {:?}",
            stdout
        );
        assert!(
            stdout.contains("99"),
            "stdout should contain '99' (updated price); got: {:?}",
            stdout
        );
    }
}
