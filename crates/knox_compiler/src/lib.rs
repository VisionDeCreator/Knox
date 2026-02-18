//! Knox compiler: lexer → parser → typecheck → Wasm codegen.

pub mod lexer;
pub mod parser;
pub mod typecheck;

use knox_syntax::diagnostics::{format_diagnostic, Diagnostic};

pub use knox_syntax::span::FileId;
use std::path::Path;

/// Compile Knox source to a Wasm module (bytes). Returns diagnostics on parse/typecheck failure.
pub fn compile(source: &str, file_id: FileId) -> Result<Vec<u8>, Vec<Diagnostic>> {
    let tokens = lexer::Lexer::new(source, file_id).collect_tokens();
    let mut parser = parser::Parser::new(tokens, file_id);
    let root = parser
        .parse_root()
        .map_err(|e| vec![Diagnostic::error(e, None)])?;
    let mut typechecker = typecheck::TypeChecker::new(file_id);
    typechecker
        .check_root(&root)
        .map_err(|_| typechecker.diagnostics)?;
    let mut wasm = Vec::new();
    knox_codegen_wasm::emit_wasm(&root, &mut wasm).map_err(|e| vec![Diagnostic::error(e, None)])?;
    Ok(wasm)
}

/// Compile a file at path. Reads source and returns Wasm bytes or diagnostics.
pub fn compile_file(path: &Path) -> Result<Vec<u8>, Vec<Diagnostic>> {
    let source = std::fs::read_to_string(path).map_err(|e| {
        vec![Diagnostic::error(
            format!("Failed to read file: {}", e),
            None,
        )]
    })?;
    compile(&source, FileId::new(0))
}

/// Print diagnostics to stderr with source context.
pub fn print_diagnostics(source: &str, file_id: FileId, diagnostics: &[Diagnostic]) {
    for d in diagnostics {
        eprintln!("{}", format_diagnostic(source, file_id, d));
    }
}
