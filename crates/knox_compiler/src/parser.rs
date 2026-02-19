//! Parser: tokens → AST.

use knox_syntax::ast::{MatchPattern, *};
use knox_syntax::span::{FileId, Span};
use knox_syntax::token::{Token, TokenKind};
use knox_syntax::Diagnostic;
use std::iter::Peekable;
use std::vec::IntoIter;

pub fn parse(tokens: Vec<Token>, file_id: FileId) -> Result<Root, Vec<Diagnostic>> {
    let mut p = Parser {
        tokens: tokens.into_iter().peekable(),
        file_id,
        diags: Vec::new(),
    };
    let root = p.parse_root();
    if p.diags.is_empty() {
        Ok(root)
    } else {
        Err(p.diags)
    }
}

struct Parser {
    tokens: Peekable<IntoIter<Token>>,
    file_id: FileId,
    diags: Vec<Diagnostic>,
}

impl Parser {
    fn peek(&mut self) -> Option<&TokenKind> {
        self.tokens.peek().map(|t| &t.kind)
    }

    fn advance(&mut self) -> Option<Token> {
        self.tokens.next()
    }

    fn loc(&self, span: Span) -> knox_syntax::span::Location {
        knox_syntax::span::Location::new(self.file_id, span)
    }

    fn error(&mut self, msg: impl Into<String>, span: Span) {
        self.diags
            .push(Diagnostic::error(msg, Some(self.loc(span))));
    }

    fn expect(&mut self, kind: TokenKind) -> Option<Span> {
        let t = self.advance()?;
        if std::mem::discriminant(&t.kind) == std::mem::discriminant(&kind) {
            Some(t.span)
        } else {
            self.error(format!("expected {:?}", kind), t.span);
            None
        }
    }

    fn expect_semicolon(&mut self, after: Span) {
        match self.peek() {
            Some(TokenKind::Semicolon) => {
                self.advance();
            }
            Some(_) => {
                let t = self.advance().unwrap();
                self.error("missing semicolon", t.span);
            }
            None => {
                self.error("missing semicolon", after);
            }
        }
    }

    fn parse_root(&mut self) -> Root {
        let mut items = Vec::new();
        while self
            .peek()
            .map(|k| !matches!(k, TokenKind::Eof))
            .unwrap_or(false)
        {
            if let Some(item) = self.parse_item() {
                items.push(item);
            }
        }
        Root { items }
    }

    fn parse_item(&mut self) -> Option<Item> {
        let vis = if matches!(self.peek(), Some(TokenKind::Export) | Some(TokenKind::Pub)) {
            self.advance();
            Visibility::Exported
        } else {
            Visibility::Private
        };
        let t = self.advance()?;
        let start = t.span.start;
        let item = match t.kind {
            TokenKind::Struct => self.parse_struct(vis, start)?,
            TokenKind::Fn => self.parse_fn(vis, start)?,
            TokenKind::Import => self.parse_import(start)?,
            _ => {
                self.error("expected struct, fn, or import", t.span);
                return None;
            }
        };
        Some(item)
    }

    fn parse_import(&mut self, start: u32) -> Option<Item> {
        let path = self.parse_path_segments()?;
        let alias = if matches!(self.peek(), Some(TokenKind::As)) {
            self.advance();
            match self.advance()?.kind {
                TokenKind::Ident(s) => Some(s),
                _ => {
                    let t = self.advance().unwrap();
                    self.error("expected identifier after as", t.span);
                    None
                }
            }
        } else {
            None
        };
        let end = if matches!(self.peek(), Some(TokenKind::Semicolon)) {
            let t = self.advance().unwrap();
            t.span.end
        } else {
            self.error("missing semicolon", Span::new(start, start + 1));
            start
        };
        Some(Item::Import(ImportDecl {
            span: Span::new(start, end),
            path,
            alias,
        }))
    }

    fn parse_path_segments(&mut self) -> Option<Vec<String>> {
        let mut segs = Vec::new();
        match self.advance()?.kind {
            TokenKind::Ident(s) => segs.push(s),
            _ => return None,
        }
        while matches!(self.peek(), Some(TokenKind::ColonColon)) {
            self.advance();
            match self.advance()?.kind {
                TokenKind::Ident(s) => segs.push(s),
                _ => return None,
            }
        }
        Some(segs)
    }

    fn parse_struct(&mut self, vis: Visibility, start: u32) -> Option<Item> {
        let name = match self.advance()?.kind {
            TokenKind::Ident(s) => s,
            _ => return None,
        };
        self.expect(TokenKind::LBrace);
        let mut fields = Vec::new();
        while !matches!(self.peek(), Some(TokenKind::RBrace) | None) {
            let f = self.parse_struct_field()?;
            fields.push(f);
            match self.peek() {
                Some(TokenKind::Semicolon) => {
                    let t = self.advance().unwrap();
                    self.error(
                        "Struct fields must be separated by commas, not semicolons",
                        t.span,
                    );
                    return None;
                }
                Some(TokenKind::Comma) => {
                    self.advance();
                }
                Some(TokenKind::RBrace) | None => break,
                _ => break,
            }
        }
        let end = self.advance().map(|t| t.span.end).unwrap_or(start);
        Some(Item::Struct(StructDecl {
            span: Span::new(start, end),
            vis,
            name,
            fields,
        }))
    }

    fn parse_struct_field(&mut self) -> Option<StructField> {
        let t = self.advance()?;
        let start = t.span.start;
        let name = match t.kind {
            TokenKind::Ident(s) => s,
            _ => {
                self.error("expected field name", t.span);
                return None;
            }
        };
        self.expect(TokenKind::Colon);
        let ty = self.parse_type()?;
        let attrs = if matches!(self.peek(), Some(TokenKind::At)) {
            self.parse_field_attrs()?
        } else {
            FieldAttrs::default()
        };
        // Struct fields are comma-delimited; end span at next token (we don't consume delimiter here)
        let end = self.tokens.peek().map(|t| t.span.start).unwrap_or(start);
        Some(StructField {
            span: Span::new(start, end),
            name,
            ty,
            attrs,
        })
    }

    fn parse_field_attrs(&mut self) -> Option<FieldAttrs> {
        self.advance(); // @
        if !matches!(self.peek(), Some(TokenKind::Pub)) {
            return Some(FieldAttrs::default());
        }
        self.advance(); // pub
        self.expect(TokenKind::LParen);
        let mut get = false;
        let mut set = false;
        loop {
            match self.advance()?.kind {
                TokenKind::Ident(s) if s == "get" => get = true,
                TokenKind::Ident(s) if s == "set" => set = true,
                TokenKind::Comma => {}
                TokenKind::RParen => break,
                _ => {}
            }
        }
        Some(FieldAttrs { get, set })
    }

    fn parse_type(&mut self) -> Option<Type> {
        let t = self.advance()?;
        let ty = match &t.kind {
            TokenKind::Ident(s) => {
                let mut path = vec![s.clone()];
                while matches!(self.peek(), Some(TokenKind::ColonColon)) {
                    self.advance();
                    if let Some(TokenKind::Ident(n)) = self.advance().map(|t| t.kind) {
                        path.push(n);
                    }
                }
                if path.len() == 1 {
                    match path[0].as_str() {
                        "int" => Type::Int,
                        "string" => Type::String,
                        "bool" => Type::Bool,
                        _ => Type::Path(path),
                    }
                } else {
                    Type::Path(path)
                }
            }
            TokenKind::LParen => {
                if matches!(self.peek(), Some(TokenKind::RParen)) {
                    self.advance();
                    Type::Unit
                } else {
                    self.error("expected () for unit type", t.span);
                    return None;
                }
            }
            TokenKind::Amp => {
                let mut_ = matches!(self.peek(), Some(TokenKind::Mut));
                if mut_ {
                    self.advance();
                }
                let inner = self.parse_type()?;
                Type::Ref(mut_, Box::new(inner))
            }
            _ => {
                self.error("expected type", t.span);
                return None;
            }
        };
        Some(ty)
    }

    fn parse_fn(&mut self, vis: Visibility, start: u32) -> Option<Item> {
        let name = match self.advance()?.kind {
            TokenKind::Ident(s) => s,
            _ => return None,
        };
        self.expect(TokenKind::LParen);
        let mut params = Vec::new();
        while !matches!(self.peek(), Some(TokenKind::RParen) | None) {
            let mut_ = matches!(self.peek(), Some(TokenKind::Mut));
            if mut_ {
                self.advance();
            }
            let pname = match self.advance()?.kind {
                TokenKind::Ident(s) => s,
                _ => return None,
            };
            self.expect(TokenKind::Colon);
            let pty = self.parse_type()?;
            params.push(Param {
                name: pname,
                ty: pty,
                mut_,
            });
            if matches!(self.peek(), Some(TokenKind::Comma)) {
                self.advance();
            }
        }
        self.expect(TokenKind::RParen);
        self.expect(TokenKind::Arrow);
        let return_ty = self.parse_type()?;
        let body = self.parse_block()?;
        let end = body.span.end;
        Some(Item::Fn(FnDecl {
            span: Span::new(start, end),
            vis,
            name,
            params,
            return_ty,
            body,
        }))
    }

    fn parse_block(&mut self) -> Option<Block> {
        let start = self.advance().map(|t| t.span.start).unwrap_or(0);
        let mut stmts = Vec::new();
        while !matches!(self.peek(), Some(TokenKind::RBrace) | None) {
            let s = self.parse_stmt()?;
            stmts.push(s);
        }
        let end = self.advance().map(|t| t.span.end).unwrap_or(start);
        Some(Block {
            span: Span::new(start, end),
            stmts,
        })
    }

    fn parse_stmt(&mut self) -> Option<Stmt> {
        let stmt = if matches!(self.peek(), Some(TokenKind::Let)) {
            let t = self.advance().unwrap();
            let start = t.span.start;
            let mut_ = matches!(self.peek(), Some(TokenKind::Mut));
            if mut_ {
                self.advance();
            }
            let name = match self.advance()?.kind {
                TokenKind::Ident(s) => s,
                _ => return None,
            };
            let ty = if matches!(self.peek(), Some(TokenKind::Colon)) {
                self.advance();
                Some(self.parse_type()?)
            } else {
                None
            };
            self.expect(TokenKind::Assign);
            let init = self.parse_expr()?;
            self.expect_semicolon(init.span());
            Stmt::Let {
                span: Span::new(start, init.span().end),
                mut_,
                name,
                ty,
                init,
            }
        } else if matches!(self.peek(), Some(TokenKind::Return)) {
            let t = self.advance().unwrap();
            let start = t.span.start;
            let value = if !matches!(self.peek(), Some(TokenKind::Semicolon) | None) {
                Some(self.parse_expr()?)
            } else {
                None
            };
            let end = value.as_ref().map(|e| e.span().end).unwrap_or(start + 6);
            self.expect_semicolon(Span::new(start, end));
            Stmt::Return {
                span: Span::new(start, end),
                value,
            }
        } else {
            let expr = self.parse_expr()?;
            self.expect_semicolon(expr.span());
            Stmt::Expr {
                span: expr.span(),
                expr,
            }
        };
        Some(stmt)
    }

    fn parse_expr(&mut self) -> Option<Expr> {
        let lhs = self.parse_expr_add()?;
        if matches!(self.peek(), Some(TokenKind::Assign)) {
            let start = lhs.span().start;
            self.advance();
            let rhs = self.parse_expr_add()?;
            return Some(Expr::Assign {
                span: Span::new(start, rhs.span().end),
                target: Box::new(lhs),
                value: Box::new(rhs),
            });
        }
        Some(lhs)
    }

    fn parse_expr_add(&mut self) -> Option<Expr> {
        let mut base = self.parse_expr_unary()?;
        while matches!(self.peek(), Some(TokenKind::Plus)) {
            let start = base.span().start;
            self.advance();
            let rhs = self.parse_expr_unary()?;
            base = Expr::Add {
                span: Span::new(start, rhs.span().end),
                lhs: Box::new(base),
                rhs: Box::new(rhs),
            };
        }
        Some(base)
    }

    fn parse_expr_unary(&mut self) -> Option<Expr> {
        if matches!(self.peek(), Some(TokenKind::Star)) {
            let t = self.advance().unwrap();
            let start = t.span.start;
            let operand = self.parse_expr_unary()?;
            return Some(Expr::Deref {
                span: Span::new(start, operand.span().end),
                expr: Box::new(operand),
            });
        }
        if matches!(self.peek(), Some(TokenKind::Amp)) {
            let t = self.advance().unwrap();
            let start = t.span.start;
            let mut_ = matches!(self.peek(), Some(TokenKind::Mut));
            if mut_ {
                self.advance();
            }
            let operand = self.parse_expr_unary()?;
            return Some(Expr::Ref {
                span: Span::new(start, operand.span().end),
                mut_,
                expr: Box::new(operand),
            });
        }
        self.parse_expr_call()
    }

    fn parse_expr_call(&mut self) -> Option<Expr> {
        let mut base = self.parse_expr_primary()?;
        loop {
            if matches!(self.peek(), Some(TokenKind::Dot)) {
                self.advance();
                let name = match self.advance()?.kind {
                    TokenKind::Ident(s) => s,
                    _ => return None,
                };
                self.expect(TokenKind::LParen);
                let mut args = Vec::new();
                while !matches!(self.peek(), Some(TokenKind::RParen) | None) {
                    args.push(self.parse_expr()?);
                    if matches!(self.peek(), Some(TokenKind::Comma)) {
                        self.advance();
                    }
                }
                let end = self
                    .advance()
                    .map(|t| t.span.end)
                    .unwrap_or(base.span().end);
                base = Expr::Call {
                    span: Span::new(base.span().start, end),
                    receiver: Some(Box::new(base)),
                    name,
                    args,
                };
            } else if matches!(self.peek(), Some(TokenKind::LParen)) {
                let name = match &base {
                    Expr::Ident { name, .. } => name.clone(),
                    _ => return Some(base),
                };
                self.advance();
                let mut args = Vec::new();
                while !matches!(self.peek(), Some(TokenKind::RParen) | None) {
                    args.push(self.parse_expr()?);
                    if matches!(self.peek(), Some(TokenKind::Comma)) {
                        self.advance();
                    }
                }
                let end = self
                    .advance()
                    .map(|t| t.span.end)
                    .unwrap_or(base.span().end);
                base = Expr::Call {
                    span: Span::new(base.span().start, end),
                    receiver: None,
                    name,
                    args,
                };
            } else {
                break;
            }
        }
        Some(base)
    }

    fn parse_expr_primary(&mut self) -> Option<Expr> {
        let t = self.advance()?;
        let start = t.span.start;
        let expr = match &t.kind {
            TokenKind::IntLiteral(n) => Expr::IntLiteral {
                span: t.span,
                value: *n,
            },
            TokenKind::StringLiteral(s) => Expr::StringLiteral {
                span: t.span,
                value: s.clone(),
            },
            TokenKind::True => Expr::BoolLiteral {
                span: t.span,
                value: true,
            },
            TokenKind::False => Expr::BoolLiteral {
                span: t.span,
                value: false,
            },
            TokenKind::Match => {
                let value = Box::new(self.parse_expr()?);
                self.expect(TokenKind::LBrace);
                let mut arms = Vec::new();
                while !matches!(self.peek(), Some(TokenKind::RBrace) | None) {
                    let pat = self.parse_match_pattern()?;
                    self.expect(TokenKind::FatArrow);
                    let arm_expr = self.parse_expr()?;
                    arms.push((pat, arm_expr));
                    if matches!(self.peek(), Some(TokenKind::Comma)) {
                        self.advance();
                    }
                }
                let end = self.advance().map(|t| t.span.end).unwrap_or(start);
                Expr::Match {
                    span: Span::new(start, end),
                    value,
                    arms,
                }
            }
            TokenKind::Ident(name) => {
                let mut segments = vec![name.clone()];
                while matches!(self.peek(), Some(TokenKind::ColonColon)) {
                    self.advance();
                    if let Some(TokenKind::Ident(s)) = self.advance().map(|t| t.kind) {
                        segments.push(s);
                    }
                }
                if segments.len() == 1 {
                    Expr::Ident {
                        span: t.span,
                        name: name.clone(),
                    }
                } else if matches!(self.peek(), Some(TokenKind::LBrace)) {
                    self.advance();
                    let mut fields = Vec::new();
                    while !matches!(self.peek(), Some(TokenKind::RBrace) | None) {
                        let fname = match self.advance()?.kind {
                            TokenKind::Ident(s) => s,
                            _ => return None,
                        };
                        self.expect(TokenKind::Colon);
                        let val = self.parse_expr()?;
                        fields.push((fname, val));
                        if matches!(self.peek(), Some(TokenKind::Comma)) {
                            self.advance();
                        }
                    }
                    let end = self.advance().map(|t| t.span.end).unwrap_or(start);
                    Expr::StructLiteral {
                        span: Span::new(start, end),
                        path: segments,
                        fields,
                    }
                } else {
                    Expr::Path {
                        span: t.span,
                        segments,
                    }
                }
            }
            _ => {
                self.error("expected expression", t.span);
                return None;
            }
        };
        Some(expr)
    }

    fn parse_match_pattern(&mut self) -> Option<MatchPattern> {
        let t = self.advance()?;
        let pat = match &t.kind {
            TokenKind::IntLiteral(n) => MatchPattern::Int(*n),
            TokenKind::True => MatchPattern::Bool(true),
            TokenKind::False => MatchPattern::Bool(false),
            TokenKind::StringLiteral(s) => MatchPattern::String(s.clone()),
            TokenKind::Underscore => MatchPattern::Underscore,
            _ => {
                self.error("expected match pattern (literal or _)", t.span);
                return None;
            }
        };
        Some(pat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use knox_syntax::ast::{Item, Visibility};

    #[test]
    fn parse_export_struct() {
        let src = "export struct User { name: string, age: int @pub(get, set), }";
        let tokens = Lexer::new(src, FileId::new(0)).collect_tokens();
        let root = parse(tokens, FileId::new(0)).expect("parse should succeed");
        assert_eq!(root.items.len(), 1);
        match &root.items[0] {
            Item::Struct(s) => {
                assert_eq!(s.vis, Visibility::Exported);
                assert_eq!(s.name, "User");
                assert_eq!(s.fields.len(), 2);
                assert_eq!(s.fields[0].name, "name");
                assert!(!s.fields[0].attrs.get && !s.fields[0].attrs.set);
                assert_eq!(s.fields[1].name, "age");
                assert!(s.fields[1].attrs.get && s.fields[1].attrs.set);
            }
            _ => panic!("expected struct"),
        }
    }

    #[test]
    fn parse_struct_fields_comma_delimited() {
        let src = "struct Point { x: int, y: int }";
        let tokens = Lexer::new(src, FileId::new(0)).collect_tokens();
        let root = parse(tokens, FileId::new(0)).expect("parse should succeed");
        match &root.items[0] {
            Item::Struct(s) => {
                assert_eq!(s.name, "Point");
                assert_eq!(s.fields.len(), 2);
                assert_eq!(s.fields[0].name, "x");
                assert_eq!(s.fields[1].name, "y");
            }
            _ => panic!("expected struct"),
        }
    }

    #[test]
    fn parse_struct_trailing_comma_allowed() {
        let src = "struct Empty { }\nstruct One { a: int, }";
        let tokens = Lexer::new(src, FileId::new(0)).collect_tokens();
        let root = parse(tokens, FileId::new(0)).expect("parse should succeed");
        assert_eq!(root.items.len(), 2);
        match &root.items[1] {
            Item::Struct(s) => {
                assert_eq!(s.name, "One");
                assert_eq!(s.fields.len(), 1);
                assert_eq!(s.fields[0].name, "a");
            }
            _ => panic!("expected struct"),
        }
    }

    #[test]
    fn parse_struct_semicolon_in_field_list_errors() {
        let src = "struct User { name: string; age: int }";
        let tokens = Lexer::new(src, FileId::new(0)).collect_tokens();
        let result = parse(tokens, FileId::new(0));
        assert!(result.is_err());
        let diags = result.unwrap_err();
        assert!(!diags.is_empty(), "expected at least one diagnostic");
        assert!(
            diags[0].message.contains("comma") && diags[0].message.contains("semicolon"),
            "expected diagnostic about commas not semicolons, got: {}",
            diags[0].message
        );
    }

    #[test]
    fn parse_import_user() {
        let src = "import user;";
        let tokens = Lexer::new(src, FileId::new(0)).collect_tokens();
        let root = parse(tokens, FileId::new(0)).expect("parse should succeed");
        assert_eq!(root.items.len(), 1);
        match &root.items[0] {
            Item::Import(imp) => {
                assert_eq!(imp.path, &["user"]);
                assert!(imp.alias.is_none());
            }
            _ => panic!("expected import"),
        }
    }

    #[test]
    fn parse_match_expr() {
        let src = r#"
fn main() -> () {
  let x = 2;
  let y = match x {
    0 => 10,
    1 => 20,
    _ => 30
  };
  print("ok");
}
"#;
        let tokens = Lexer::new(src, FileId::new(0)).collect_tokens();
        let root = parse(tokens, FileId::new(0)).expect("parse should succeed");
        assert_eq!(root.items.len(), 1);
        match &root.items[0] {
            Item::Fn(f) => {
                assert_eq!(f.name, "main");
                assert_eq!(f.body.stmts.len(), 3); // let x, let y, print
                if let knox_syntax::ast::Stmt::Let {
                    init: knox_syntax::ast::Expr::Match { arms, .. },
                    ..
                } = &f.body.stmts[1]
                {
                    assert_eq!(arms.len(), 3);
                    return;
                }
                panic!("expected let y = match ... with 3 arms");
            }
            _ => panic!("expected fn main"),
        }
    }
}
