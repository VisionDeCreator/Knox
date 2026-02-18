//! Parser: tokens → AST.

use knox_syntax::ast::*;
use knox_syntax::span::{FileId, Span};
use knox_syntax::token::{Token, TokenKind};
use std::iter::Peekable;
use std::vec::IntoIter;

#[allow(dead_code)]
pub struct Parser {
    tokens: Peekable<IntoIter<Token>>,
    file: FileId,
    last_span: Span,
}

impl Parser {
    pub fn new(tokens: Vec<Token>, file: FileId) -> Self {
        Self {
            tokens: tokens.into_iter().peekable(),
            file,
            last_span: Span::default(),
        }
    }

    fn peek(&mut self) -> &TokenKind {
        static EOF: TokenKind = TokenKind::Eof;
        self.tokens.peek().map(|t| &t.kind).unwrap_or(&EOF)
    }

    fn next(&mut self) -> Option<Token> {
        let t = self.tokens.next()?;
        self.last_span = t.span;
        Some(t)
    }

    fn expect(&mut self, kind: TokenKind) -> Result<(), String> {
        let t = self
            .next()
            .ok_or_else(|| "Unexpected end of file".to_string())?;
        if std::mem::discriminant(&t.kind) != std::mem::discriminant(&kind) {
            return Err(format!("Expected {:?}, got {:?}", kind, t.kind));
        }
        Ok(())
    }

    fn span_from(&self, start: u32) -> Span {
        Span::new(start, self.last_span.end)
    }

    pub fn parse_root(&mut self) -> Result<Root, String> {
        let start = 0;
        let mut items = Vec::new();
        while !matches!(self.peek(), TokenKind::Eof) {
            if matches!(self.peek(), TokenKind::Import) {
                items.push(Item::Import(self.parse_import()?));
            } else if matches!(self.peek(), TokenKind::Struct) {
                items.push(Item::Struct(self.parse_struct()?));
            } else if matches!(self.peek(), TokenKind::Pub) | matches!(self.peek(), TokenKind::Fn) {
                items.push(Item::Fn(self.parse_fn()?));
            } else {
                let t = self.next().unwrap();
                return Err(format!("Unexpected token at root: {:?}", t.kind));
            }
        }
        let end = self.last_span.end;
        Ok(Root {
            items,
            span: Span::new(start, end),
        })
    }

    fn parse_fn(&mut self) -> Result<FnDecl, String> {
        let pub_vis = matches!(self.peek(), TokenKind::Pub);
        if pub_vis {
            self.next(); // consume pub
        }
        self.expect(TokenKind::Fn)?;
        let start = self.last_span.start;
        let name_t = self.next().ok_or("Expected function name")?;
        let name = match &name_t.kind {
            TokenKind::Ident(s) => s.clone(),
            _ => return Err("Expected function name".into()),
        };
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        while !matches!(self.peek(), TokenKind::RParen) {
            let param_name = self.next().ok_or("Expected param name")?;
            let param_name = match &param_name.kind {
                TokenKind::Ident(s) => s.clone(),
                _ => return Err("Expected param name".into()),
            };
            self.expect(TokenKind::Colon)?;
            let ty = self.parse_type()?;
            params.push(Param {
                name: param_name,
                ty,
                span: self.last_span,
            });
            if !matches!(self.peek(), TokenKind::RParen) {
                self.expect(TokenKind::Comma)?;
            }
        }
        self.expect(TokenKind::RParen)?;
        self.expect(TokenKind::Arrow)?;
        let return_type = self.parse_type()?;
        self.expect(TokenKind::LBrace)?;
        let body = self.parse_block()?;
        self.expect(TokenKind::RBrace)?;
        Ok(FnDecl {
            name,
            params,
            return_type,
            body,
            span: self.span_from(start),
            pub_vis,
        })
    }

    fn parse_import(&mut self) -> Result<ImportDecl, String> {
        self.expect(TokenKind::Import)?;
        let start = self.last_span.start;
        let mut path = Vec::new();
        let first = self.next().ok_or("Expected module path")?;
        let name = match &first.kind {
            TokenKind::Ident(s) => s.clone(),
            _ => return Err("Expected module path".into()),
        };
        path.push(name);
        while matches!(self.peek(), TokenKind::ColonColon) {
            self.next(); // ::
            if matches!(self.peek(), TokenKind::LBrace) {
                break; // path done, then { a, b }
            }
            let t = self.next().ok_or("Expected path segment or item")?;
            let s = match &t.kind {
                TokenKind::Ident(x) => x.clone(),
                _ => return Err("Expected path segment".into()),
            };
            if matches!(self.peek(), TokenKind::ColonColon) {
                path.push(s);
            } else {
                // single item: import auth::token::verify
                let alias = if matches!(self.peek(), TokenKind::As) {
                    self.next();
                    let at = self.next().ok_or("Expected alias")?;
                    match &at.kind {
                        TokenKind::Ident(a) => Some(a.clone()),
                        _ => None,
                    }
                } else {
                    None
                };
                return Ok(ImportDecl {
                    path,
                    alias,
                    items: Some(vec![s]),
                    span: self.span_from(start),
                });
            }
        }
        let items = if matches!(self.peek(), TokenKind::LBrace) {
            self.next(); // {
            let mut names = Vec::new();
            while !matches!(self.peek(), TokenKind::RBrace) {
                let t = self.next().ok_or("Expected item name")?;
                let s = match &t.kind {
                    TokenKind::Ident(x) => x.clone(),
                    _ => return Err("Expected item name".into()),
                };
                names.push(s);
                if !matches!(self.peek(), TokenKind::RBrace) {
                    self.expect(TokenKind::Comma)?;
                }
            }
            self.expect(TokenKind::RBrace)?;
            Some(names)
        } else {
            None
        };
        let alias = if matches!(self.peek(), TokenKind::As) {
            self.next();
            let t = self.next().ok_or("Expected alias name")?;
            match &t.kind {
                TokenKind::Ident(a) => Some(a.clone()),
                _ => return Err("Expected alias name".into()),
            }
        } else {
            None
        };
        Ok(ImportDecl {
            path,
            alias,
            items,
            span: self.span_from(start),
        })
    }

    fn parse_struct(&mut self) -> Result<StructDecl, String> {
        self.expect(TokenKind::Struct)?;
        let start = self.last_span.start;
        let name_t = self.next().ok_or("Expected struct name")?;
        let name = match &name_t.kind {
            TokenKind::Ident(s) => s.clone(),
            _ => return Err("Expected struct name".into()),
        };
        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while !matches!(self.peek(), TokenKind::RBrace) {
            fields.push(self.parse_struct_field()?);
            if !matches!(self.peek(), TokenKind::RBrace) {
                // optional comma
                if matches!(self.peek(), TokenKind::Comma) {
                    self.next();
                }
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok(StructDecl {
            name,
            fields,
            span: self.span_from(start),
        })
    }

    fn parse_struct_field(&mut self) -> Result<StructField, String> {
        let start = self.last_span.start;
        let name_t = self.next().ok_or("Expected field name")?;
        let name = match &name_t.kind {
            TokenKind::Ident(s) => s.clone(),
            _ => return Err("Expected field name".into()),
        };
        self.expect(TokenKind::Colon)?;
        let ty = self.parse_type()?;
        let attrs = if matches!(self.peek(), TokenKind::At) {
            Some(self.parse_field_attrs()?)
        } else {
            None
        };
        Ok(StructField {
            name,
            ty,
            attrs,
            span: self.span_from(start),
        })
    }

    fn parse_field_attrs(&mut self) -> Result<FieldAttrs, String> {
        self.expect(TokenKind::At)?;
        self.expect(TokenKind::Pub)?;
        self.expect(TokenKind::LParen)?;
        let mut get = false;
        let mut set = false;
        loop {
            let t = self.next().ok_or("Expected get or set")?;
            match &t.kind {
                TokenKind::Ident(s) => {
                    if s == "get" {
                        get = true;
                    } else if s == "set" {
                        set = true;
                    } else {
                        return Err(format!("Expected get or set, got {}", s));
                    }
                }
                _ => return Err("Expected get or set".into()),
            }
            if !matches!(self.peek(), TokenKind::Comma) {
                break;
            }
            self.next(); // comma
        }
        self.expect(TokenKind::RParen)?;
        Ok(FieldAttrs { get, set })
    }

    fn parse_type(&mut self) -> Result<Type, String> {
        let t = self.next().ok_or("Expected type")?;
        match &t.kind {
            TokenKind::Ident(s) => {
                let ty = match s.as_str() {
                    "u64" => Type::U64,
                    "int" => Type::Int,
                    "string" => Type::String,
                    "bool" => Type::Bool,
                    "dynamic" => Type::Dynamic,
                    "Option" => {
                        self.expect(TokenKind::LBracket)?;
                        let inner = self.parse_type()?;
                        self.expect(TokenKind::RBracket)?;
                        Type::Option(Box::new(inner))
                    }
                    "Result" => {
                        self.expect(TokenKind::LBracket)?;
                        let ok = self.parse_type()?;
                        self.expect(TokenKind::Comma)?;
                        let err = self.parse_type()?;
                        self.expect(TokenKind::RBracket)?;
                        Type::Result(Box::new(ok), Box::new(err))
                    }
                    _ => Type::Named(s.clone()),
                };
                Ok(ty)
            }
            TokenKind::LParen => {
                self.expect(TokenKind::RParen)?;
                Ok(Type::Unit)
            }
            _ => Err(format!("Expected type, got {:?}", t.kind)),
        }
    }

    fn parse_block(&mut self) -> Result<Block, String> {
        let start = self.last_span.start;
        let mut stmts = Vec::new();
        while !matches!(self.peek(), TokenKind::RBrace | TokenKind::Eof) {
            stmts.push(self.parse_stmt()?);
        }
        Ok(Block {
            stmts,
            span: self.span_from(start),
        })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        let start = self.last_span.start;
        if matches!(self.peek(), TokenKind::Let) {
            self.next(); // let
            let mutability = matches!(self.peek(), TokenKind::Mut);
            if mutability {
                self.next();
            }
            let name_t = self.next().ok_or("Expected binding name")?;
            let name = match &name_t.kind {
                TokenKind::Ident(s) => s.clone(),
                _ => return Err("Expected binding name".into()),
            };
            self.expect(TokenKind::Assign)?;
            let init = self.parse_expr()?;
            self.expect_semicolon_after_stmt()?;
            return Ok(Stmt::Let {
                name,
                mutability,
                init,
                span: self.span_from(start),
            });
        }
        if matches!(self.peek(), TokenKind::Return) {
            self.next();
            let value = if matches!(self.peek(), TokenKind::RBrace) || matches!(self.peek(), TokenKind::Semicolon) {
                None
            } else {
                Some(self.parse_expr()?)
            };
            self.expect_semicolon_after_stmt()?;
            return Ok(Stmt::Return(value, self.span_from(start)));
        }
        let expr = self.parse_expr()?;
        self.expect_semicolon_after_stmt()?;
        Ok(Stmt::Expr(expr))
    }

    fn expect_semicolon_after_stmt(&mut self) -> Result<(), String> {
        if matches!(self.peek(), TokenKind::Semicolon) {
            self.next();
            Ok(())
        } else {
            Err("Expected ';' after statement".into())
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_expr_bp(0)
    }

    fn parse_expr_bp(&mut self, _min_bp: u8) -> Result<Expr, String> {
        let start = self.tokens.peek().map(|t| t.span.start).unwrap_or(0);
        let expr = if matches!(self.peek(), TokenKind::If) {
            self.next();
            let cond = Box::new(self.parse_expr()?);
            self.expect(TokenKind::LBrace)?;
            let then_block = self.parse_block()?;
            self.expect(TokenKind::RBrace)?;
            let else_block = if matches!(self.peek(), TokenKind::Else) {
                self.next();
                self.expect(TokenKind::LBrace)?;
                let b = self.parse_block()?;
                self.expect(TokenKind::RBrace)?;
                Some(b)
            } else {
                None
            };
            Expr::If {
                cond,
                then_block,
                else_block,
                span: self.span_from(start),
            }
        } else if matches!(self.peek(), TokenKind::Match) {
            self.next();
            let scrutinee = Box::new(self.parse_expr()?);
            self.expect(TokenKind::LBrace)?;
            let mut arms = Vec::new();
            while !matches!(self.peek(), TokenKind::RBrace) {
                let arm_start = self.tokens.peek().map(|t| t.span.start).unwrap_or(0);
                let pattern = self.parse_match_pattern()?;
                self.expect(TokenKind::FatArrow)?;
                let body = self.parse_expr()?;
                arms.push(MatchArm {
                    pattern,
                    body,
                    span: self.span_from(arm_start),
                });
                if matches!(self.peek(), TokenKind::Comma) {
                    self.next();
                }
            }
            self.expect(TokenKind::RBrace)?;
            Expr::Match {
                scrutinee,
                arms,
                span: self.span_from(start),
            }
        } else if matches!(self.peek(), TokenKind::LBrace) {
            self.next();
            let block = self.parse_block()?;
            self.expect(TokenKind::RBrace)?;
            Expr::Block(block, self.span_from(start))
        } else {
            self.parse_primary()?
        };
        Ok(expr)
    }

    fn parse_match_pattern(&mut self) -> Result<MatchPattern, String> {
        let start = self.tokens.peek().map(|t| t.span.start).unwrap_or(0);
        if matches!(self.peek(), TokenKind::Underscore) {
            self.next();
            return Ok(MatchPattern::Wildcard(self.span_from(start)));
        }
        if matches!(self.peek(), TokenKind::LBrace) {
            self.next();
            let mut fields = Vec::new();
            while !matches!(self.peek(), TokenKind::RBrace) {
                let name_t = self.next().ok_or("Expected field name")?;
                let name = match &name_t.kind {
                    TokenKind::Ident(s) => s.clone(),
                    _ => return Err("Expected field name in pattern".into()),
                };
                self.expect(TokenKind::Colon)?;
                let ty = self.parse_type()?;
                fields.push((name, ty));
                if !matches!(self.peek(), TokenKind::RBrace) {
                    self.expect(TokenKind::Comma)?;
                }
            }
            self.expect(TokenKind::RBrace)?;
            return Ok(MatchPattern::RecordDestruct {
                fields,
                span: self.span_from(start),
            });
        }
        if matches!(
            self.peek(),
            TokenKind::IntLiteral(_)
                | TokenKind::StringLiteral(_)
                | TokenKind::True
                | TokenKind::False
        ) {
            let lit = self.parse_literal()?;
            return Ok(MatchPattern::Literal(lit, self.span_from(start)));
        }
        Err("Expected match pattern".into())
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        let start = self.tokens.peek().map(|t| t.span.start).unwrap_or(0);
        let t = self.next().ok_or("Expected expression")?;
        match &t.kind {
            TokenKind::IntLiteral(n) => Ok(Expr::Literal(Literal::Int(*n), t.span)),
            TokenKind::StringLiteral(s) => Ok(Expr::Literal(Literal::String(s.clone()), t.span)),
            TokenKind::True => Ok(Expr::Literal(Literal::Bool(true), t.span)),
            TokenKind::False => Ok(Expr::Literal(Literal::Bool(false), t.span)),
            TokenKind::LParen => {
                if matches!(self.peek(), TokenKind::RParen) {
                    self.next();
                    return Ok(Expr::Literal(Literal::Unit, self.span_from(start)));
                }
                let inner = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                Ok(inner)
            }
            TokenKind::Ident(callee) => {
                let mut name = callee.clone();
                while matches!(self.peek(), TokenKind::ColonColon) {
                    self.next(); // ::
                    let next_t = self.next().ok_or("Expected identifier after ::")?;
                    let seg = match &next_t.kind {
                        TokenKind::Ident(s) => s.clone(),
                        _ => return Err("Expected identifier after ::".into()),
                    };
                    name.push_str("::");
                    name.push_str(&seg);
                }
                if matches!(self.peek(), TokenKind::LParen) {
                    self.next(); // (
                    let mut args = Vec::new();
                    while !matches!(self.peek(), TokenKind::RParen) {
                        args.push(self.parse_expr()?);
                        if !matches!(self.peek(), TokenKind::RParen) {
                            self.expect(TokenKind::Comma)?;
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    Ok(Expr::Call {
                        callee: name,
                        args,
                        span: self.span_from(start),
                    })
                } else {
                    Ok(Expr::Ident(name, t.span))
                }
            }
            TokenKind::Ok | TokenKind::Err => {
                let constructor = match &t.kind {
                    TokenKind::Ok => "Ok",
                    TokenKind::Err => "Err",
                    _ => unreachable!(),
                };
                self.expect(TokenKind::LParen)?;
                let arg = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                Ok(Expr::Call {
                    callee: constructor.to_string(),
                    args: vec![arg],
                    span: self.span_from(start),
                })
            }
            _ => Err(format!("Expected expression, got {:?}", t.kind)),
        }
    }

    fn parse_literal(&mut self) -> Result<Literal, String> {
        let t = self.next().ok_or("Expected literal")?;
        match &t.kind {
            TokenKind::IntLiteral(n) => Ok(Literal::Int(*n)),
            TokenKind::StringLiteral(s) => Ok(Literal::String(s.clone())),
            TokenKind::True => Ok(Literal::Bool(true)),
            TokenKind::False => Ok(Literal::Bool(false)),
            _ => Err("Expected literal".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;
    use knox_syntax::ast::Item;

    #[test]
    fn parse_hello_main() {
        let src = r#"fn main() -> () { print("Hello, Knox!"); }"#;
        let tokens = lexer::Lexer::new(src, FileId::new(0)).collect_tokens();
        let mut parser = Parser::new(tokens, FileId::new(0));
        let root = parser.parse_root().unwrap();
        assert_eq!(root.items.len(), 1);
        let Item::Fn(f) = &root.items[0] else { panic!("expected Fn") };
        assert_eq!(f.name, "main");
        assert_eq!(f.params.len(), 0);
    }

    #[test]
    fn semicolon_required_after_statement() {
        let src = r#"fn main() -> () { let x = 1 }"#;
        let tokens = lexer::Lexer::new(src, FileId::new(0)).collect_tokens();
        let mut parser = Parser::new(tokens, FileId::new(0));
        let res = parser.parse_root();
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Expected ';' after statement"));
    }

    #[test]
    fn semicolon_required_after_expr_statement() {
        let src = r#"fn main() -> () { print("hi") }"#;
        let tokens = lexer::Lexer::new(src, FileId::new(0)).collect_tokens();
        let mut parser = Parser::new(tokens, FileId::new(0));
        let res = parser.parse_root();
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Expected ';' after statement"));
    }

    #[test]
    fn valid_semicolons_parse() {
        let src = r#"
fn main() -> () {
  let x = 1;
  let mut y = 2;
  return x;
}
"#;
        let tokens = lexer::Lexer::new(src, FileId::new(0)).collect_tokens();
        let mut parser = Parser::new(tokens, FileId::new(0));
        let root = parser.parse_root().unwrap();
        assert_eq!(root.items.len(), 1);
    }

    #[test]
    fn parse_struct_with_pub_accessors() {
        let src = r#"
struct User {
  name: string
  age: int @pub(get, set)
  email: string @pub(get)
}
"#;
        let tokens = lexer::Lexer::new(src, FileId::new(0)).collect_tokens();
        let mut parser = Parser::new(tokens, FileId::new(0));
        let root = parser.parse_root().unwrap();
        assert_eq!(root.items.len(), 1);
        let Item::Struct(s) = &root.items[0] else { panic!("expected Struct") };
        assert_eq!(s.name, "User");
        assert_eq!(s.fields.len(), 3);
        assert!(s.fields[1].attrs.as_ref().unwrap().get);
        assert!(s.fields[1].attrs.as_ref().unwrap().set);
    }

    #[test]
    fn parse_import() {
        let src = "import auth::token::{verify, sign}";
        let tokens = lexer::Lexer::new(src, FileId::new(0)).collect_tokens();
        let mut parser = Parser::new(tokens, FileId::new(0));
        let root = parser.parse_root().unwrap();
        assert_eq!(root.items.len(), 1);
        let Item::Import(imp) = &root.items[0] else { panic!("expected Import") };
        assert_eq!(imp.path, ["auth", "token"]);
        assert_eq!(imp.items.as_ref().unwrap(), &["verify", "sign"]);
    }
}
