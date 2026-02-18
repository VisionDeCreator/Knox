//! Knox CLI: build, run, new, fmt (stub).

use clap::{Parser, Subcommand};
use knox_compiler::print_diagnostics;
use knox_syntax::span::FileId;
use std::path::{Path, PathBuf};
use std::process::Command;

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

fn cmd_build(_target: &str, path: &Path) -> Result<(), String> {
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    let (compile_path, out_path) = if path.is_dir() {
        let main_path = path.join("main.kx");
        let main_path = if main_path.exists() {
            main_path
        } else {
            path.join("src").join("main.kx")
        };
        if !main_path.exists() {
            return Err("No main.kx or src/main.kx found in directory".into());
        }
        (
            main_path,
            path.join("dist")
                .join(
                    path.file_name()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or("out"),
                )
                .with_extension("wasm"),
        )
    } else if path.extension().map(|e| e == "kx").unwrap_or(false) {
        let out_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
        let out_path = path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("dist")
            .join(out_name)
            .with_extension("wasm");
        (path.to_path_buf(), out_path)
    } else {
        return Err("Expected .kx file or project directory".into());
    };

    let source_for_diags = std::fs::read_to_string(&compile_path).unwrap_or_default();
    let wasm = knox_compiler::compile_file(&compile_path).map_err(|diags| {
        print_diagnostics(&source_for_diags, FileId::new(0), &diags);
        "Compilation failed".to_string()
    })?;

    let out_dir = out_path.parent().unwrap();
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    std::fs::write(&out_path, wasm).map_err(|e| e.to_string())?;
    println!("Wrote {}", out_path.display());
    Ok(())
}

fn cmd_run(path: &Path) -> Result<(), String> {
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    let compile_path = if path.is_dir() {
        let main_path = path.join("main.kx");
        let main_path = if main_path.exists() {
            main_path
        } else {
            path.join("src").join("main.kx")
        };
        if !main_path.exists() {
            return Err("No main.kx or src/main.kx found in directory".into());
        }
        main_path
    } else {
        path.to_path_buf()
    };
    let source = std::fs::read_to_string(&compile_path).map_err(|e| e.to_string())?;
    let wasm = knox_compiler::compile_file(&compile_path).map_err(|diags| {
        print_diagnostics(&source, FileId::new(0), &diags);
        "Compilation failed".to_string()
    })?;

    let wasm_path = compile_path.parent().unwrap().join(".knox_run.wasm");
    std::fs::write(&wasm_path, &wasm).map_err(|e| e.to_string())?;

    let wasmtime = which::which("wasmtime").map_err(|_| {
        "Wasmtime is required to run Knox programs. Install from https://wasmtime.dev".to_string()
    })?;
    let status = Command::new(wasmtime)
        .arg("run")
        .arg(&wasm_path)
        .status()
        .map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&wasm_path);
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
