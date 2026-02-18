//! Knox compiler: lexer → parser → typecheck → Wasm codegen.

pub mod desugar;
pub mod lexer;
pub mod modules;
pub mod parser;
pub mod typecheck;

use knox_syntax::diagnostics::{format_diagnostic, Diagnostic};

pub use knox_syntax::span::FileId;
use std::path::Path;

/// Compile Knox source to a Wasm module (bytes). Returns diagnostics on parse/typecheck failure.
pub fn compile(source: &str, file_id: FileId) -> Result<Vec<u8>, Vec<Diagnostic>> {
    let tokens = lexer::Lexer::new(source, file_id).collect_tokens();
    let mut parser = parser::Parser::new(tokens, file_id);
    let mut root = parser
        .parse_root()
        .map_err(|e| vec![Diagnostic::error(e, None)])?;
    desugar::desugar_root(&mut root);
    let mut typechecker = typecheck::TypeChecker::new(file_id);
    typechecker
        .check_root(&root)
        .map_err(|_| typechecker.diagnostics)?;
    let mut wasm = Vec::new();
    knox_codegen_wasm::emit_wasm(&root, &mut wasm).map_err(|e| vec![Diagnostic::error(e, None)])?;
    Ok(wasm)
}

/// Compile a file at path. Resolves same-directory imports and merges their pub items so
/// qualified calls (e.g. greet::greet()) typecheck and codegen.
pub fn compile_file(path: &Path) -> Result<Vec<u8>, Vec<Diagnostic>> {
    let source = std::fs::read_to_string(path).map_err(|e| {
        vec![Diagnostic::error(
            format!("Failed to read file: {}", e),
            None,
        )]
    })?;
    let tokens = lexer::Lexer::new(&source, FileId::new(0)).collect_tokens();
    let mut parser = parser::Parser::new(tokens, FileId::new(0));
    let mut root = parser
        .parse_root()
        .map_err(|e| vec![Diagnostic::error(e, None)])?;
    desugar::desugar_root(&mut root);

    let dir = path.parent().unwrap_or_else(|| path.as_ref());
    let mut merged_items = root.items.clone();
    for item in &root.items {
        if let knox_syntax::ast::Item::Import(imp) = item {
            if imp.path.len() == 1 {
                let mod_name = &imp.path[0];
                let dep_path = dir.join(format!("{}.kx", mod_name));
                if dep_path.exists() {
                    let dep_source = std::fs::read_to_string(&dep_path).map_err(|e| {
                        vec![Diagnostic::error(
                            format!("Failed to read {}: {}", dep_path.display(), e),
                            None,
                        )]
                    })?;
                    let dep_tokens = lexer::Lexer::new(&dep_source, FileId::new(0)).collect_tokens();
                    let mut dep_parser = parser::Parser::new(dep_tokens, FileId::new(0));
                    let mut dep_root = dep_parser
                        .parse_root()
                        .map_err(|e| vec![Diagnostic::error(format!("{}: {}", dep_path.display(), e), None)])?;
                    desugar::desugar_root(&mut dep_root);
                    for dep_item in &dep_root.items {
                        if let knox_syntax::ast::Item::Fn(f) = dep_item {
                            if f.pub_vis {
                                let mut q = f.clone();
                                q.name = format!("{}::{}", mod_name, f.name);
                                merged_items.push(knox_syntax::ast::Item::Fn(q));
                            }
                        }
                    }
                }
            }
        }
    }
    let merged_root = knox_syntax::ast::Root {
        items: merged_items,
        span: root.span,
    };

    let mut typechecker = typecheck::TypeChecker::new(FileId::new(0));
    typechecker
        .check_root(&merged_root)
        .map_err(|_| typechecker.diagnostics)?;
    let mut wasm = Vec::new();
    knox_codegen_wasm::emit_wasm(&merged_root, &mut wasm).map_err(|e| vec![Diagnostic::error(e, None)])?;
    Ok(wasm)
}

/// Print diagnostics to stderr with source context.
pub fn print_diagnostics(source: &str, file_id: FileId, diagnostics: &[Diagnostic]) {
    for d in diagnostics {
        eprintln!("{}", format_diagnostic(source, file_id, d));
    }
}
