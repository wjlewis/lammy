use crate::source::Span;
use std::rc::Rc;

#[derive(Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub text: Rc<String>,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, text: Rc<String>, span: Span) -> Self {
        Token { kind, text, span }
    }

    pub fn is_trivial(&self) -> bool {
        self.kind.is_trivial()
    }

    pub fn is_nontrivial(&self) -> bool {
        self.kind.is_nontrivial()
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum TokenKind {
    LParen,             // (
    RParen,             // )
    LBrace,             // {
    RBrace,             // }
    Comma,              // ,
    Semi,               // ;
    Equals,             // =
    Arrow,              // =>
    Name,               // [a-z][a-zA-Z0-9*+']*
    Alias,              // [A-Z][a-zA-Z0-9*+']*
    String,             // ".."
    UnterminatedString, // "..
    Comment,            // # ..
    Whitespace,         // ' ' | \t | \n | \r | \r\n
    Eof,                //
    Unknown,            //
}

impl TokenKind {
    pub fn is_trivial(&self) -> bool {
        match self {
            Self::Whitespace | Self::Comment | Self::Unknown => true,
            _ => false,
        }
    }

    pub fn is_nontrivial(&self) -> bool {
        !self.is_trivial()
    }
}
