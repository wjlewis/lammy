mod interner;
mod tokens;

pub use tokens::{Token, TokenKind};

use crate::source::Span;
use interner::Interner;
use std::collections::VecDeque;
use std::rc::Rc;
use std::str::Chars;
use TokenKind as Tk;

pub struct Lexer<'a> {
    source: &'a str,
    chars: Chars<'a>,
    interner: Interner<'a>,
    peeked: VecDeque<Token>,
}

impl<'a> Lexer<'a> {
    pub fn pop(&mut self) -> Token {
        match self.peeked.pop_front() {
            Some(next) => next,
            None => self.read_next(),
        }
    }

    pub fn peek(&mut self) -> &Token {
        if self.peeked.is_empty() {
            let next = self.read_next();
            self.peeked.push_back(next);
        }

        self.peeked.get(0).unwrap()
    }

    pub fn peek_ahead(&mut self, n: usize) -> &Token {
        if let Some(need_to_peek) = n.checked_sub(self.peeked.len()) {
            for _ in 0..=need_to_peek {
                let next = self.read_next();
                self.peeked.push_back(next);
            }
        }

        self.peeked.get(n).unwrap()
    }

    fn read_next(&mut self) -> Token {
        let start = self.current_pos();
        let next = self.chars.next();
        if next.is_none() {
            return Token::new(Tk::Eof, self.interner.intern(""), Span::new(start, start));
        }

        let kind = match next.unwrap() {
            '(' => Tk::LParen,
            ')' => Tk::RParen,
            '{' => Tk::LBrace,
            '}' => Tk::RBrace,
            ',' => Tk::Comma,
            ';' => Tk::Semi,
            '=' => self.read_equals_or_arrow(),
            '#' => self.read_comment(),
            '"' => self.read_string(),
            c if Self::is_name_start(c) => self.read_name(),
            c if Self::is_alias_start(c) => self.read_alias(),
            c if Self::is_whitespace(c) => self.read_whitespace(),
            _ => self.read_unknown(),
        };

        let end = self.current_pos();
        let text = self.extract_text(&kind, start, end);
        Token::new(kind, text, Span::new(start, end))
    }

    fn read_equals_or_arrow(&mut self) -> Tk {
        if let Some('>') = self.peek_char() {
            self.chars.next();
            Tk::Arrow
        } else {
            Tk::Equals
        }
    }

    fn read_comment(&mut self) -> Tk {
        self.eat_while(|c| match c {
            '\n' | '\r' => false,
            _ => true,
        });
        Tk::Comment
    }

    fn read_string(&mut self) -> Tk {
        let mut escape_next = false;
        while let Some(c) = self.peek_char() {
            match c {
                '"' if !escape_next => {
                    self.chars.next();
                    return Tk::String;
                }
                '\\' if !escape_next => {
                    escape_next = true;
                }
                '\n' | '\r' => {
                    break;
                }
                _ => {
                    escape_next = false;
                }
            }
            self.chars.next();
        }
        Tk::UnterminatedString
    }

    fn read_name(&mut self) -> Tk {
        self.eat_while(Self::is_name_continue);
        Tk::Name
    }

    fn read_alias(&mut self) -> Tk {
        self.eat_while(Self::is_alias_continue);
        Tk::Alias
    }

    fn read_whitespace(&mut self) -> Tk {
        self.eat_while(Self::is_whitespace);
        Tk::Whitespace
    }

    fn read_unknown(&mut self) -> Tk {
        self.eat_while(Self::is_unknown);
        Tk::Unknown
    }

    fn eat_while(&mut self, pred: impl Fn(char) -> bool) {
        while let Some(c) = self.peek_char() {
            if !pred(c) {
                break;
            }
            self.chars.next();
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.chars.clone().next()
    }

    fn current_pos(&self) -> usize {
        self.source.len() - self.chars.as_str().len()
    }

    fn is_name_start(c: char) -> bool {
        match c {
            'a'..='z' => true,
            _ => false,
        }
    }

    fn is_alias_start(c: char) -> bool {
        match c {
            'A'..='Z' => true,
            _ => false,
        }
    }

    fn is_name_continue(c: char) -> bool {
        match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '*' | '+' | '\'' | '?' => true,
            _ => false,
        }
    }

    fn is_alias_continue(c: char) -> bool {
        Self::is_name_continue(c)
    }

    fn is_whitespace(c: char) -> bool {
        match c {
            ' ' | '\t' | '\n' | '\r' => true,
            _ => false,
        }
    }

    fn is_unknown(c: char) -> bool {
        match c {
            '(' | ')' | '{' | '}' | ',' | ';' | '=' | '\\' | '-' | '#' => false,
            '\n' | '\r' => false,
            c if Self::is_name_start(c) => false,
            c if Self::is_alias_start(c) => false,
            c if Self::is_whitespace(c) => false,
            _ => true,
        }
    }

    fn extract_text(&mut self, kind: &Tk, start: usize, end: usize) -> Rc<String> {
        let start = match kind {
            Tk::String | Tk::UnterminatedString => start + 1,
            _ => start,
        };
        let end = match kind {
            Tk::String => end - 1,
            _ => end,
        };
        let text = &self.source[start..end];
        self.interner.intern(text)
    }
}

impl<'a> From<&'a str> for Lexer<'a> {
    fn from(source: &'a str) -> Self {
        Lexer {
            source,
            chars: source.chars(),
            interner: Interner::default(),
            peeked: VecDeque::new(),
        }
    }
}
