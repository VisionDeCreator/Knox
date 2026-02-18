//! Lexer: source text → tokens.

use knox_syntax::span::{FileId, Span};
use knox_syntax::token::{Token, TokenKind};
use std::iter::Peekable;
use std::str::Chars;

#[allow(dead_code)]
pub struct Lexer<'a> {
    source: &'a str,
    chars: Peekable<Chars<'a>>,
    offset: u32,
    file: FileId,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str, file: FileId) -> Self {
        Self {
            source,
            chars: source.chars().peekable(),
            offset: 0,
            file,
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    fn next(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        self.offset += c.len_utf8() as u32;
        Some(c)
    }

    fn start_offset(&self) -> u32 {
        self.offset
    }

    fn span_from(&self, start: u32) -> Span {
        Span::new(start, self.offset)
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace() && c != '\n') {
            self.next();
        }
    }

    fn skip_line_comment(&mut self) {
        while matches!(self.peek(), Some(c) if c != '\n') {
            self.next();
        }
    }

    fn read_ident_or_keyword(&mut self) -> (String, Span) {
        let start = self.start_offset();
        let mut s = String::new();
        while matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == '_') {
            s.push(self.next().unwrap());
        }
        (s, self.span_from(start))
    }

    fn read_string(&mut self) -> Result<(String, Span), String> {
        let start = self.start_offset();
        self.next(); // "
        let mut s = String::new();
        loop {
            match self.next() {
                None => return Err("Unterminated string".into()),
                Some('"') => break,
                Some('\\') => match self.next() {
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some('"') => s.push('"'),
                    Some('\\') => s.push('\\'),
                    _ => return Err("Invalid escape in string".into()),
                },
                Some(c) => s.push(c),
            }
        }
        Ok((s, self.span_from(start)))
    }

    fn read_number(&mut self) -> (i64, Span) {
        let start = self.start_offset();
        let mut s = String::new();
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            s.push(self.next().unwrap());
        }
        let n = s.parse().unwrap_or(0);
        (n, self.span_from(start))
    }

    pub fn next_token(&mut self) -> Token {
        loop {
            self.skip_whitespace();
            let start = self.start_offset();

            let c = match self.peek() {
                Some(c) => c,
                None => {
                    return Token::new(TokenKind::Eof, self.span_from(start));
                }
            };

            if c == '/' {
                self.next();
                if self.peek() == Some('/') {
                    self.next();
                    self.skip_line_comment();
                    continue;
                }
                // single / not part of // — treat as invalid or skip; MVP: skip
                continue;
            }

            if c.is_ascii_alphabetic() || c == '_' {
                let (s, span) = self.read_ident_or_keyword();
                let kind = match s.as_str() {
                    "fn" => TokenKind::Fn,
                    "let" => TokenKind::Let,
                    "mut" => TokenKind::Mut,
                    "if" => TokenKind::If,
                    "else" => TokenKind::Else,
                    "match" => TokenKind::Match,
                    "return" => TokenKind::Return,
                    "Ok" => TokenKind::Ok,
                    "Err" => TokenKind::Err,
                    "Option" => TokenKind::Option,
                    "Result" => TokenKind::Result,
                    "dynamic" => TokenKind::Dynamic,
                    "Some" => TokenKind::Some,
                    "None" => TokenKind::None,
                    "true" => TokenKind::True,
                    "false" => TokenKind::False,
                    _ => TokenKind::Ident(s),
                };
                return Token::new(kind, span);
            }

            if c == '"' {
                match self.read_string() {
                    Ok((s, span)) => return Token::new(TokenKind::StringLiteral(s), span),
                    Err(_) => {
                        return Token::new(
                            TokenKind::StringLiteral(String::new()),
                            self.span_from(start),
                        )
                    }
                }
            }

            if c.is_ascii_digit() {
                let (n, span) = self.read_number();
                return Token::new(TokenKind::IntLiteral(n), span);
            }

            self.next();
            let span = self.span_from(start);

            let kind = match c {
                '(' => TokenKind::LParen,
                ')' => TokenKind::RParen,
                '{' => TokenKind::LBrace,
                '}' => TokenKind::RBrace,
                '[' => TokenKind::LBracket,
                ']' => TokenKind::RBracket,
                ':' => TokenKind::Colon,
                ',' => TokenKind::Comma,
                '.' => TokenKind::Dot,
                '?' => TokenKind::Question,
                '|' => TokenKind::Pipe,
                '_' => TokenKind::Underscore,
                '<' => TokenKind::Lt,
                '>' => TokenKind::Gt,
                '=' => {
                    if self.peek() == Some('=') {
                        self.next();
                        TokenKind::Eq
                    } else if self.peek() == Some('>') {
                        self.next();
                        TokenKind::FatArrow
                    } else {
                        TokenKind::Assign
                    }
                }
                '!' => {
                    if self.peek() == Some('=') {
                        self.next();
                        TokenKind::Ne
                    } else {
                        TokenKind::Ident("!".into())
                    }
                }
                '-' => {
                    if self.peek() == Some('>') {
                        self.next();
                        TokenKind::Arrow
                    } else {
                        TokenKind::Ident("-".into())
                    }
                }
                _ => continue,
            };
            return Token::new(kind, span);
        }
    }

    /// Lex entire source into token stream (for parser).
    pub fn collect_tokens(mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let t = self.next_token();
            let is_eof = t.is_eof();
            tokens.push(t);
            if is_eof {
                break;
            }
        }
        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_ident_and_keywords() {
        let src = "fn let mut if else match return";
        let tokens = Lexer::new(src, FileId::new(0)).collect_tokens();
        assert!(matches!(tokens[0].kind, TokenKind::Fn));
        assert!(matches!(tokens[1].kind, TokenKind::Let));
        assert!(matches!(tokens[2].kind, TokenKind::Mut));
    }

    #[test]
    fn lex_string_literal() {
        let src = r#""hello""#;
        let tokens = Lexer::new(src, FileId::new(0)).collect_tokens();
        match &tokens[0].kind {
            TokenKind::StringLiteral(s) => assert_eq!(s, "hello"),
            _ => panic!("expected string literal"),
        }
    }

    #[test]
    fn lex_int_literal() {
        let src = "42 0";
        let tokens = Lexer::new(src, FileId::new(0)).collect_tokens();
        match tokens[0].kind {
            TokenKind::IntLiteral(n) => assert_eq!(n, 42),
            _ => panic!("expected int"),
        }
        match tokens[1].kind {
            TokenKind::IntLiteral(n) => assert_eq!(n, 0),
            _ => panic!("expected int"),
        }
    }

    #[test]
    fn lex_arrow_and_fat_arrow() {
        let src = "-> =>";
        let tokens = Lexer::new(src, FileId::new(0)).collect_tokens();
        assert!(matches!(tokens[0].kind, TokenKind::Arrow));
        assert!(matches!(tokens[1].kind, TokenKind::FatArrow));
    }
}
