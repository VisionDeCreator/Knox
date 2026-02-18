//! Minimal type checker for MVP: literals, function signatures, print, Option/Result.

use knox_syntax::ast::*;
use knox_syntax::diagnostics::{Diagnostic, Level};
use knox_syntax::span::{FileId, Location, Span};
use std::collections::HashMap;

pub struct TypeChecker {
    pub diagnostics: Vec<Diagnostic>,
    file: FileId,
    functions: HashMap<String, FnDecl>,
    structs: HashMap<String, StructDecl>,
}

impl TypeChecker {
    pub fn new(file: FileId) -> Self {
        Self {
            diagnostics: Vec::new(),
            file,
            functions: HashMap::new(),
            structs: HashMap::new(),
        }
    }

    #[allow(clippy::result_unit_err)]
    pub fn check_root(&mut self, root: &Root) -> Result<(), ()> {
        for item in &root.items {
            match item {
                Item::Fn(f) => {
                    self.functions.insert(f.name.clone(), f.clone());
                }
                Item::Struct(s) => {
                    self.structs.insert(s.name.clone(), s.clone());
                }
                Item::Import(_) => {}
            }
        }
        for item in &root.items {
            match item {
                Item::Fn(f) => self.check_fn(f)?,
                Item::Struct(_) | Item::Import(_) => {}
            }
        }
        if !self.functions.contains_key("main") {
            self.diagnostics.push(Diagnostic::error(
                "No main function found. Expected fn main() -> () { ... }",
                Some(Location::new(self.file, root.span)),
            ));
            return Err(());
        }
        if self.diagnostics.iter().any(|d| d.level == Level::Error) {
            return Err(());
        }
        Ok(())
    }

    fn check_fn(&mut self, f: &FnDecl) -> Result<(), ()> {
        let mut env: HashMap<String, Type> = HashMap::new();
        for p in &f.params {
            env.insert(p.name.clone(), p.ty.clone());
        }
        self.check_block(&f.body, &env)?;
        if f.name == "main" {
            self.check_type_equals(&f.return_type, &Type::Unit, f.span)?;
        }
        Ok(())
    }

    fn check_block(&mut self, block: &Block, env: &HashMap<String, Type>) -> Result<(), ()> {
        for stmt in &block.stmts {
            self.check_stmt(stmt, env)?;
        }
        Ok(())
    }

    fn check_stmt(&mut self, stmt: &Stmt, env: &HashMap<String, Type>) -> Result<(), ()> {
        match stmt {
            Stmt::Let { name, init, .. } => {
                let ty = self.check_expr(init, env)?;
                // could store in env for subsequent stmts; MVP we don't use it
                let _ = (name, ty);
            }
            Stmt::Expr(e) => {
                self.check_expr(e, env)?;
            }
            Stmt::Return(Some(e), _) => {
                self.check_expr(e, env)?;
            }
            Stmt::Return(None, _) => {}
        }
        Ok(())
    }

    fn check_expr(&mut self, expr: &Expr, env: &HashMap<String, Type>) -> Result<Type, ()> {
        match expr {
            Expr::Literal(lit, span) => Ok(self.type_of_literal(lit, *span)),
            Expr::Ident(name, span) => {
                if let Some(ty) = env.get(name) {
                    Ok(ty.clone())
                } else {
                    self.diagnostics.push(Diagnostic::error(
                        format!("Unknown variable: {}", name),
                        Some(Location::new(self.file, *span)),
                    ));
                    Err(())
                }
            }
            Expr::FieldAccess {
                receiver,
                field,
                span,
            } => {
                let rec_ty = self.check_expr(receiver, env)?;
                let Type::Named(struct_name) = rec_ty else {
                    self.diagnostics.push(Diagnostic::error(
                        "Field access only on struct types",
                        Some(Location::new(self.file, *span)),
                    ));
                    return Err(());
                };
                let Some(s) = self.structs.get(&struct_name) else {
                    self.diagnostics.push(Diagnostic::error(
                        format!("Unknown type: {}", struct_name),
                        Some(Location::new(self.file, *span)),
                    ));
                    return Err(());
                };
                let Some(f) = s.fields.iter().find(|fld| fld.name == *field) else {
                    self.diagnostics.push(Diagnostic::error(
                        format!("Unknown field: {} on {}", field, struct_name),
                        Some(Location::new(self.file, *span)),
                    ));
                    return Err(());
                };
                Ok(f.ty.clone())
            }
            Expr::Call { callee, args, span } => {
                if callee == "print" {
                    if args.len() != 1 {
                        self.diagnostics.push(Diagnostic::error(
                            "print expects exactly one argument (string)",
                            Some(Location::new(self.file, *span)),
                        ));
                        return Err(());
                    }
                    let arg_ty = self.check_expr(&args[0], env)?;
                    if arg_ty != Type::String {
                        self.diagnostics.push(Diagnostic::error(
                            "print argument must be string",
                            Some(Location::new(self.file, *span)),
                        ));
                        return Err(());
                    }
                    return Ok(Type::Unit);
                }
                if let Some(f) = self.functions.get(callee).cloned() {
                    if f.params.len() != args.len() {
                        self.diagnostics.push(Diagnostic::error(
                            format!("Expected {} arguments, got {}", f.params.len(), args.len()),
                            Some(Location::new(self.file, *span)),
                        ));
                        return Err(());
                    }
                    for (p, a) in f.params.iter().zip(args.iter()) {
                        let aty = self.check_expr(a, env)?;
                        self.check_type_equals(&p.ty, &aty, a.span())?;
                    }
                    return Ok(f.return_type.clone());
                }
                self.diagnostics.push(Diagnostic::error(
                    format!("Unknown function: {}", callee),
                    Some(Location::new(self.file, *span)),
                ));
                Err(())
            }
            Expr::If {
                cond,
                then_block,
                else_block,
                ..
            } => {
                self.check_expr(cond, env)?;
                self.check_block(then_block, env)?;
                if let Some(eb) = else_block {
                    self.check_block(eb, env)?;
                }
                Ok(Type::Unit)
            }
            Expr::Match {
                scrutinee, arms, ..
            } => {
                self.check_expr(scrutinee, env)?;
                for arm in arms {
                    self.check_expr(&arm.body, env)?;
                }
                Ok(Type::Unit)
            }
            Expr::Block(block, _) => {
                self.check_block(block, env)?;
                Ok(Type::Unit)
            }
        }
    }

    fn type_of_literal(&self, lit: &Literal, _span: Span) -> Type {
        match lit {
            Literal::Int(_) => Type::Int,
            Literal::String(_) => Type::String,
            Literal::Bool(_) => Type::Bool,
            Literal::Unit => Type::Unit,
        }
    }

    fn check_type_equals(&mut self, expected: &Type, actual: &Type, span: Span) -> Result<(), ()> {
        if expected != actual {
            self.diagnostics.push(Diagnostic::error(
                format!("Type mismatch: expected {:?}, got {:?}", expected, actual),
                Some(Location::new(self.file, span)),
            ));
            return Err(());
        }
        Ok(())
    }
}

trait ExprSpan {
    fn span(&self) -> Span;
}

impl ExprSpan for Expr {
    fn span(&self) -> Span {
        match self {
            Expr::Literal(_, s) => *s,
            Expr::Ident(_, s) => *s,
            Expr::FieldAccess { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::If { span, .. } => *span,
            Expr::Match { span, .. } => *span,
            Expr::Block(_, s) => *s,
        }
    }
}
