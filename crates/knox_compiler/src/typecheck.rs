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
        let mut env: HashMap<String, (Type, bool)> = HashMap::new();
        for p in &f.params {
            env.insert(p.name.clone(), (p.ty.clone(), false));
        }
        self.check_block(&f.body, &mut env)?;
        if f.name == "main" {
            self.check_type_equals(&f.return_type, &Type::Unit, f.span)?;
        }
        Ok(())
    }

    fn check_block(
        &mut self,
        block: &Block,
        env: &mut HashMap<String, (Type, bool)>,
    ) -> Result<(), ()> {
        for stmt in &block.stmts {
            self.check_stmt(stmt, env)?;
        }
        Ok(())
    }

    fn check_stmt(
        &mut self,
        stmt: &Stmt,
        env: &mut HashMap<String, (Type, bool)>,
    ) -> Result<(), ()> {
        match stmt {
            Stmt::Let {
                name,
                mutability,
                type_annot,
                init,
                span,
            } => {
                let init_ty = self.check_expr(init, env)?;
                let ty = match type_annot {
                    Some(annot) => {
                        self.check_type_equals(annot, &init_ty, *span)?;
                        annot.clone()
                    }
                    None => init_ty,
                };
                env.insert(name.clone(), (ty, *mutability));
            }
            Stmt::AssignDeref { name, expr, span } => {
                let (var_ty, _) = env.get(name).ok_or_else(|| {
                    self.diagnostics.push(Diagnostic::error(
                        format!("Unknown variable: {}", name),
                        Some(Location::new(self.file, *span)),
                    ));
                    ()
                })?;
                let inner = match var_ty {
                    Type::Ref(inner, true) => inner.as_ref().clone(),
                    _ => {
                        self.diagnostics.push(Diagnostic::error(
                            "Assign through * requires &mut reference".to_string(),
                            Some(Location::new(self.file, *span)),
                        ));
                        return Err(());
                    }
                };
                let expr_ty = self.check_expr(expr, env)?;
                self.check_type_equals(&inner, &expr_ty, expr.span())?;
            }
            Stmt::Assign { name, expr, span } => {
                let (var_ty, mutability) = env.get(name).cloned().ok_or_else(|| {
                    self.diagnostics.push(Diagnostic::error(
                        format!("Unknown variable: {}", name),
                        Some(Location::new(self.file, *span)),
                    ));
                    ()
                })?;
                if !mutability {
                    self.diagnostics.push(Diagnostic::error(
                        "Cannot assign to immutable variable".to_string(),
                        Some(Location::new(self.file, *span)),
                    ));
                    return Err(());
                }
                let expr_ty = self.check_expr(expr, env)?;
                self.check_type_equals(&var_ty, &expr_ty, expr.span())?;
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

    fn check_expr(
        &mut self,
        expr: &Expr,
        env: &mut HashMap<String, (Type, bool)>,
    ) -> Result<Type, ()> {
        match expr {
            Expr::Literal(lit, span) => Ok(self.type_of_literal(lit, *span)),
            Expr::Ident(name, span) => {
                if let Some((ty, _)) = env.get(name) {
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
                scrutinee,
                arms,
                span,
            } => {
                let scrut_ty = self.check_expr(scrutinee, env)?;
                let mut arm_ty = None::<Type>;
                for arm in arms {
                    let t = self.check_expr(&arm.body, env)?;
                    if let Some(ref expected) = arm_ty {
                        self.check_type_equals(expected, &t, arm.body.span())?;
                    } else {
                        arm_ty = Some(t);
                    }
                }
                let result_ty = arm_ty.unwrap_or(Type::Unit);
                self.check_match_exhaustive(scrut_ty, arms, *span)?;
                Ok(result_ty)
            }
            Expr::Block(block, _) => {
                let mut block_env = env.clone();
                self.check_block(block, &mut block_env)?;
                if let Some(Stmt::Expr(e)) = block.stmts.last() {
                    Ok(self.check_expr(e, &mut block_env)?)
                } else {
                    Ok(Type::Unit)
                }
            }
            Expr::BinaryOp {
                op,
                left,
                right,
                span,
            } => {
                use knox_syntax::ast::BinOp;
                let lt = self.check_expr(left, env)?;
                let rt = self.check_expr(right, env)?;
                match op {
                    BinOp::Add => {
                        if lt == Type::Int && rt == Type::Int {
                            Ok(Type::Int)
                        } else if lt == Type::U64 && rt == Type::U64 {
                            Ok(Type::U64)
                        } else if lt == Type::String && rt == Type::String {
                            Ok(Type::String)
                        } else {
                            self.diagnostics.push(Diagnostic::error(
                                "Operator + requires int+int, u64+u64, or string+string"
                                    .to_string(),
                                Some(Location::new(self.file, *span)),
                            ));
                            Err(())
                        }
                    }
                    BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
                        if lt == Type::Int && rt == Type::Int {
                            Ok(Type::Int)
                        } else if lt == Type::U64 && rt == Type::U64 {
                            Ok(Type::U64)
                        } else {
                            self.diagnostics.push(Diagnostic::error(
                                format!("Operator {:?} requires both operands int or both u64", op),
                                Some(Location::new(self.file, *span)),
                            ));
                            Err(())
                        }
                    }
                    BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                        self.check_type_equals(&lt, &rt, *span)?;
                        Ok(Type::Bool)
                    }
                    BinOp::And | BinOp::Or => {
                        self.check_type_equals(&Type::Bool, &lt, left.span())?;
                        self.check_type_equals(&Type::Bool, &rt, right.span())?;
                        Ok(Type::Bool)
                    }
                }
            }
            Expr::UnaryOp { op, expr, span } => {
                use knox_syntax::ast::UnOp;
                let t = self.check_expr(expr, env)?;
                match op {
                    UnOp::Neg => {
                        if t == Type::Int || t == Type::U64 {
                            Ok(t)
                        } else {
                            self.diagnostics.push(Diagnostic::error(
                                "Unary - requires int or u64".to_string(),
                                Some(Location::new(self.file, *span)),
                            ));
                            Err(())
                        }
                    }
                    UnOp::Not => {
                        self.check_type_equals(&Type::Bool, &t, expr.span())?;
                        Ok(Type::Bool)
                    }
                }
            }
            Expr::Ref {
                is_mut,
                target,
                span,
            } => {
                let (ty, mutability) = env.get(target).ok_or_else(|| {
                    self.diagnostics.push(Diagnostic::error(
                        format!("Unknown variable: {}", target),
                        Some(Location::new(self.file, *span)),
                    ));
                    ()
                })?;
                if *is_mut && !mutability {
                    self.diagnostics.push(Diagnostic::error(
                        "Cannot take &mut of immutable variable".to_string(),
                        Some(Location::new(self.file, *span)),
                    ));
                    return Err(());
                }
                Ok(Type::Ref(Box::new(ty.clone()), *is_mut))
            }
            Expr::Deref { expr, span } => {
                let ref_ty = self.check_expr(expr, env)?;
                match &ref_ty {
                    Type::Ref(inner, _) => Ok(inner.as_ref().clone()),
                    _ => {
                        self.diagnostics.push(Diagnostic::error(
                            "Deref * only applies to &T or &mut T".to_string(),
                            Some(Location::new(self.file, *span)),
                        ));
                        Err(())
                    }
                }
            }
        }
    }

    fn check_match_exhaustive(
        &mut self,
        scrut_ty: Type,
        arms: &[MatchArm],
        span: Span,
    ) -> Result<(), ()> {
        let has_wildcard = arms
            .iter()
            .any(|a| matches!(a.pattern, MatchPattern::Wildcard(_)));
        if has_wildcard {
            return Ok(());
        }
        if scrut_ty == Type::Bool {
            let has_true = arms
                .iter()
                .any(|a| matches!(&a.pattern, MatchPattern::Literal(Literal::Bool(true), _)));
            let has_false = arms
                .iter()
                .any(|a| matches!(&a.pattern, MatchPattern::Literal(Literal::Bool(false), _)));
            if has_true && has_false {
                return Ok(());
            }
        }
        self.diagnostics.push(Diagnostic::error(
            "Match must be exhaustive (add _ arm or cover all cases)".to_string(),
            Some(Location::new(self.file, span)),
        ));
        Err(())
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
            Expr::BinaryOp { span, .. } => *span,
            Expr::UnaryOp { span, .. } => *span,
            Expr::Ref { span, .. } => *span,
            Expr::Deref { span, .. } => *span,
            Expr::If { span, .. } => *span,
            Expr::Match { span, .. } => *span,
            Expr::Block(_, s) => *s,
        }
    }
}
