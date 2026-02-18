//! Lexer tokens for Knox.

use crate::span::Span;

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    // Literals
    IntLiteral(i64),
    StringLiteral(String),
    True,
    False,

    // Identifiers and keywords
    Ident(String),
    Fn,
    Let,
    Mut,
    If,
    Else,
    Match,
    Return,
    Struct,
    Import,
    Pub,
    As,
    Ok,
    Err,
    Option,
    Result,
    Dynamic,
    Some,
    None,

    // Symbols
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Colon,
    Comma,
    Arrow,    // ->
    FatArrow, // =>
    Dot,
    Question,
    Pipe, // |
    Underscore,
    Assign, // =
    At,       // @
    ColonColon, // ::
    Semicolon,  // ;

    // Operators
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,

    Eof,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn is_eof(&self) -> bool {
        matches!(self.kind, TokenKind::Eof)
    }
}
