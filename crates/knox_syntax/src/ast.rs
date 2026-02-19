//! AST types for Knox (functions, structs, imports, expressions).

use crate::span::Span;

/// Visibility for cross-module access. Only exported items can be imported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    Private,
    Exported,
}

/// Root of a module: list of top-level items.
#[derive(Clone, Debug)]
pub struct Root {
    pub items: Vec<Item>,
}

/// Top-level item in a module.
#[derive(Clone, Debug)]
pub enum Item {
    Fn(FnDecl),
    Struct(StructDecl),
    Import(ImportDecl),
}

/// Function declaration.
#[derive(Clone, Debug)]
pub struct FnDecl {
    pub span: Span,
    pub vis: Visibility,
    pub name: String,
    pub params: Vec<Param>,
    pub return_ty: Type,
    pub body: Block,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub mut_: bool,
}

/// Struct declaration.
#[derive(Clone, Debug)]
pub struct StructDecl {
    pub span: Span,
    pub vis: Visibility,
    pub name: String,
    pub fields: Vec<StructField>,
}

#[derive(Clone, Debug)]
pub struct StructField {
    pub span: Span,
    pub name: String,
    pub ty: Type,
    pub attrs: FieldAttrs,
}

/// @pub(get), @pub(set), or @pub(get, set)
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FieldAttrs {
    pub get: bool,
    pub set: bool,
}

impl FieldAttrs {
    /// True if this field has `@pub(get)` or `@pub(get, set)`.
    pub fn has_pub_get(&self) -> bool {
        self.get
    }
    /// True if this field has `@pub(set)` or `@pub(get, set)`.
    pub fn has_pub_set(&self) -> bool {
        self.set
    }
}

/// Import declaration: `import user` or `import user as u`
#[derive(Clone, Debug)]
pub struct ImportDecl {
    pub span: Span,
    pub path: Vec<String>,
    pub alias: Option<String>,
}

/// Type reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Int,
    String,
    Bool,
    Unit,
    Path(Vec<String>),
    /// Reference: &T or &mut T
    Ref(bool, Box<Type>),
}

/// Block: `{ stmts }`
#[derive(Clone, Debug)]
pub struct Block {
    pub span: Span,
    pub stmts: Vec<Stmt>,
}

/// Match pattern (literal or _).
#[derive(Clone, Debug)]
pub enum MatchPattern {
    Int(i64),
    Bool(bool),
    String(String),
    Underscore,
}

/// Statement.
#[derive(Clone, Debug)]
pub enum Stmt {
    Let {
        span: Span,
        mut_: bool,
        name: String,
        ty: Option<Type>,
        init: Expr,
    },
    Expr {
        span: Span,
        expr: Expr,
    },
    Return {
        span: Span,
        value: Option<Expr>,
    },
}

/// Expression.
#[derive(Clone, Debug)]
pub enum Expr {
    IntLiteral {
        span: Span,
        value: i64,
    },
    StringLiteral {
        span: Span,
        value: String,
    },
    BoolLiteral {
        span: Span,
        value: bool,
    },
    Ident {
        span: Span,
        name: String,
    },
    /// Qualified path: user::User (type or module)
    Path {
        span: Span,
        segments: Vec<String>,
    },
    /// Struct literal: user::User { name: "John", age: 20 }
    StructLiteral {
        span: Span,
        path: Vec<String>,
        fields: Vec<(String, Expr)>,
    },
    /// Method or function call: user.name() or user.set_age(30) or print(x)
    Call {
        span: Span,
        receiver: Option<Box<Expr>>,
        name: String,
        args: Vec<Expr>,
    },
    /// Assignment: x = expr (receiver is the lvalue)
    Assign {
        span: Span,
        target: Box<Expr>,
        value: Box<Expr>,
    },
    /// match expr { pat => expr, ... }
    Match {
        span: Span,
        value: Box<Expr>,
        arms: Vec<(MatchPattern, Expr)>,
    },
    /// Dereference: *expr
    Deref {
        span: Span,
        expr: Box<Expr>,
    },
    /// Reference: &expr or &mut expr
    Ref {
        span: Span,
        mut_: bool,
        expr: Box<Expr>,
    },
    /// Binary add: lhs + rhs
    Add {
        span: Span,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
}

impl Expr {
    /// Span of this expression in source.
    pub fn span(&self) -> Span {
        match self {
            Expr::IntLiteral { span, .. }
            | Expr::StringLiteral { span, .. }
            | Expr::BoolLiteral { span, .. }
            | Expr::Ident { span, .. }
            | Expr::Path { span, .. }
            | Expr::StructLiteral { span, .. }
            | Expr::Call { span, .. }
            | Expr::Assign { span, .. }
            | Expr::Match { span, .. }
            | Expr::Deref { span, .. }
            | Expr::Ref { span, .. }
            | Expr::Add { span, .. } => *span,
        }
    }
}
