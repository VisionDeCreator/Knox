//! AST nodes for Knox.

use crate::span::Span;

/// Root of a Knox source file.
#[derive(Clone, Debug)]
pub struct Root {
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum Item {
    Fn(FnDecl),
    Struct(StructDecl),
    Import(ImportDecl),
}

#[derive(Clone, Debug)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Type,
    pub body: Block,
    pub span: Span,
    /// Whether this function is public (importable from other modules).
    pub pub_vis: bool,
}

#[derive(Clone, Debug)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<StructField>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
    pub attrs: Option<FieldAttrs>,
    pub span: Span,
}

/// Attribute for field accessor generation: @pub(get), @pub(set), @pub(get, set).
#[derive(Clone, Debug, Default)]
pub struct FieldAttrs {
    pub get: bool,
    pub set: bool,
}

#[derive(Clone, Debug)]
pub struct ImportDecl {
    /// Module path segments, e.g. ["auth", "token"] for auth::token.
    pub path: Vec<String>,
    /// Alias for the whole module, e.g. `import http as h` -> Some("h").
    pub alias: Option<String>,
    /// If Some, import only these names; if None, import the whole module.
    pub items: Option<Vec<String>>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Let {
        name: String,
        mutability: bool,
        init: Expr,
        span: Span,
    },
    Expr(Expr),
    Return(Option<Expr>, Span),
}

#[derive(Clone, Debug)]
pub enum Expr {
    Literal(Literal, Span),
    Ident(String, Span),
    /// Receiver.field (for getters and general field access).
    FieldAccess {
        receiver: Box<Expr>,
        field: String,
        span: Span,
    },
    Call {
        callee: String,
        args: Vec<Expr>,
        span: Span,
    },
    If {
        cond: Box<Expr>,
        then_block: Block,
        else_block: Option<Block>,
        span: Span,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    Block(Block, Span),
}

#[derive(Clone, Debug)]
pub enum Literal {
    Int(i64),
    String(String),
    Bool(bool),
    Unit,
}

#[derive(Clone, Debug)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub body: Expr,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum MatchPattern {
    Wildcard(Span),
    Literal(Literal, Span),
    RecordDestruct {
        fields: Vec<(String, Type)>,
        span: Span,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Unit,
    U64,
    Int,
    String,
    Bool,
    Dynamic,
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Named(String), // for User, Account, Address, Error etc.
}

impl Type {
    pub fn unit() -> Self {
        Type::Unit
    }
    pub fn string() -> Self {
        Type::String
    }
}
