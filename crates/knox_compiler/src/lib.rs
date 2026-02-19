//! Knox compiler: lexer, parser, desugar, type checker, pipeline, IR, lowering.

mod desugar;
mod ir;
mod lexer;
mod lower;
mod modules;
mod parser;

use knox_codegen_wasm;
use knox_syntax::diagnostics::{format_diagnostic, Diagnostic};
use knox_syntax::span::FileId;
use std::path::Path;

/// Print diagnostics to stderr with source context.
pub fn print_diagnostics(source: &str, file_id: FileId, diags: &[Diagnostic]) {
    for d in diags {
        eprintln!("{}", format_diagnostic(source, file_id, d));
    }
}

/// Compile a single file or package entry point to Wasm.
/// When path is inside a package (has knox.toml), resolves imports from src/.
/// Returns either Wasm bytes or a list of diagnostics.
pub fn compile_file(path: &Path) -> Result<Vec<u8>, Vec<Diagnostic>> {
    let path = path.canonicalize().map_err(|e| {
        vec![Diagnostic::error(
            format!("failed to canonicalize: {}", e),
            None,
        )]
    })?;
    let source = std::fs::read_to_string(&path).map_err(|e| {
        vec![Diagnostic::error(
            format!("failed to read file: {}", e),
            None,
        )]
    })?;
    let file_id = FileId::new(0);
    let tokens = lexer::Lexer::new(&source, file_id).collect_tokens();
    let root = match parser::parse(tokens, file_id) {
        Ok(r) => r,
        Err(diags) => return Err(diags),
    };

    // Package root: nearest ancestor with knox.toml, or if under examples/<name>/src/ use that directory (monorepo convention).
    let package_root = path.ancestors().find(|p| p.join("knox.toml").exists());
    let package_root = package_root.or_else(|| {
        let path_str = path.to_string_lossy();
        if path_str.contains("examples") && path_str.contains("src") {
            path.ancestors().find(|a| {
                let src_dir = a.join("src");
                path.starts_with(&src_dir)
            })
        } else {
            None
        }
    });
    let debug = std::env::var("KNOX_DEBUG").is_ok();
    if debug {
        if let Some(ref pkg) = package_root {
            eprintln!("[KNOX_DEBUG] compiler package_root: {}", pkg.display());
            eprintln!(
                "[KNOX_DEBUG] compiler module_root: {}",
                pkg.join("src").display()
            );
        } else {
            eprintln!("[KNOX_DEBUG] compiler package_root: (none)");
        }
    }
    let mut deps: Vec<(String, knox_syntax::ast::Root)> = Vec::new();
    for item in &root.items {
        if let knox_syntax::ast::Item::Import(imp) = item {
            if imp.path.len() == 1 && imp.alias.is_none() {
                let mod_name = &imp.path[0];
                if let Some(pkg) = package_root {
                    let mod_path = vec![mod_name.clone()];
                    let dep_path = modules::resolve_internal(pkg, &mod_path);
                    if let Some(ref dep_path) = dep_path {
                        if let Ok(dep_src) = std::fs::read_to_string(dep_path) {
                            let dep_tokens =
                                lexer::Lexer::new(&dep_src, FileId::new(1)).collect_tokens();
                            if let Ok(dep_root) = parser::parse(dep_tokens, FileId::new(1)) {
                                deps.push((mod_name.clone(), dep_root));
                            }
                        }
                    }
                }
            }
        }
    }

    let (layouts, accessors) = desugar::collect_struct_layouts_and_accessors(&deps);

    let program = match lower::lower_to_ir(&root, &deps, &layouts, &accessors) {
        Ok(p) => p,
        Err(e) => return Err(vec![Diagnostic::error(e, None)]),
    };
    if debug {
        eprintln!(
            "[KNOX_DEBUG] compiler: lowered to IR: {} functions, {} struct layouts, {} string data",
            program.functions.len(),
            program.struct_layouts.len(),
            program.string_data.len(),
        );
    }
    let wasm = knox_codegen_wasm::emit_from_ir(&program, debug);
    Ok(wasm)
}
