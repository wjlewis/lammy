mod interner;

use self::interner::Interner;
use super::tokens::{Token, TokenKind as Tk};
use crate::source::Span;
use std::collections::VecDeque;
use std::rc::Rc;
use std::str::Chars;

/// Produces tokens from an input string slice on demand. Interns token text,
/// and permits arbitrary lookaheads.
pub struct Lexer<'a> {
    /// The source string
    source: &'a str,
    chars: Chars<'a>,
    interner: Interner<'a>,
    /// A collection of already peeked tokens.
    peeked: VecDeque<Token>,
}

impl<'a> From<&'a str> for Lexer<'a> {
    fn from(source: &'a str) -> Self {
        Self {
            source,
            chars: source.chars(),
            interner: Interner::default(),
            peeked: VecDeque::new(),
        }
    }
}

impl<'a> Lexer<'a> {
    /// Returns the next token from the source text. Note that this token may
    /// have already been peeked.
    pub fn pop(&mut self) -> Token {
        match self.peeked.pop_front() {
            Some(next) => next,
            None => self.read_next(),
        }
    }

    /// Returns a reference to the next token to be popped. `peek` is
    /// idempotent: subsequent calls to `peek` produce the same value
    /// as the first call.
    pub fn peek(&mut self) -> &Token {
        if self.peeked.is_empty() {
            let next = self.read_next();
            self.peeked.push_back(next);
        }

        self.peeked.get(0).unwrap()
    }

    /// Returns a reference to the `n`th token to be popped. Like `peek`,
    /// `peek_ahead` is idempotent.
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
        Tk::Var
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
            '(' | ')' | '{' | '}' | ',' | ';' | '=' | '\\' | '#' => false,
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
        self.interner.intern(&self.source[start..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Tk::*;

    impl<'a> Iterator for Lexer<'a> {
        type Item = Token;

        fn next(&mut self) -> Option<Self::Item> {
            let popped = self.pop();
            match popped.kind {
                Tk::Eof => None,
                _ => Some(popped),
            }
        }
    }

    impl<'a> Lexer<'a> {
        fn collect_kinds(self) -> Vec<Tk> {
            self.into_iter().map(|t| t.kind).collect()
        }
    }

    #[test]
    fn peek_is_idempotent() {
        let mut l = Lexer::from("test=>");

        let peek_kind = l.peek().kind;
        assert_eq!(peek_kind, Var);
        assert_eq!(l.peek().kind, peek_kind);
    }

    #[test]
    fn peek_ahead_is_idempotent() {
        let mut l = Lexer::from("first second third");

        let second_peek_kind = l.peek_ahead(2).kind;
        assert_eq!(second_peek_kind, Var);
        assert_eq!(l.peek_ahead(2).kind, second_peek_kind);
    }

    #[test]
    fn correctly_assigns_text_and_spans() {
        let mut l = Lexer::from("var Alias\t=>");
        //                       0123456789 012

        let next = l.pop();
        assert_eq!(*next.text, "var");
        assert_eq!(next.span, Span::new(0, 3));

        let next = l.pop();
        assert_eq!(*next.text, " ");
        assert_eq!(next.span, Span::new(3, 4));

        let next = l.pop();
        assert_eq!(*next.text, "Alias");
        assert_eq!(next.span, Span::new(4, 9));

        let next = l.pop();
        assert_eq!(*next.text, "\t");
        assert_eq!(next.span, Span::new(9, 10));

        let next = l.pop();
        assert_eq!(*next.text, "=>");
        assert_eq!(next.span, Span::new(10, 12));
    }

    #[test]
    fn correctly_assigns_non_ascii_spans() {
        let mut l = Lexer::from("τϵστ");
        //                       02468

        let next = l.pop();
        assert_eq!(*next.text, "τϵστ");
        assert_eq!(next.span, Span::new(0, 8));
    }

    #[test]
    fn correctly_distinguishes_equals_from_arrow() {
        let l = Lexer::from("=var=>Alias");

        assert_eq!(l.collect_kinds(), vec![Equals, Var, Arrow, Alias]);
    }

    #[test]
    fn reads_unterminated_strings() {
        let l = Lexer::from(
            r#""unterminated string
var Alias"#,
        );

        assert_eq!(
            l.collect_kinds(),
            vec![UnterminatedString, Whitespace, Var, Whitespace, Alias]
        );
    }

    #[test]
    fn reads_unknown_tokens() {
        let l = Lexer::from("**-^^%<>:: unknown");

        assert_eq!(l.collect_kinds(), vec![Unknown, Whitespace, Var]);
    }

    #[test]
    fn passes_smoke_test_1() {
        let l = Lexer::from("(x, y) => x");

        assert_eq!(
            l.collect_kinds(),
            vec![LParen, Var, Comma, Whitespace, Var, RParen, Whitespace, Arrow, Whitespace, Var,]
        );
    }

    #[test]
    fn passes_smoke_test_2() {
        let l = Lexer::from("Id = x => x;");

        assert_eq!(
            l.collect_kinds(),
            vec![
                Alias, Whitespace, Equals, Whitespace, Var, Whitespace, Arrow, Whitespace, Var,
                Semi
            ]
        );
    }

    #[test]
    fn passes_smoke_test_3() {
        let l = Lexer::from(
            r#"import {} from "./common";
# My first comment
Quux = foo bar;
"#,
        );

        assert_eq!(
            l.collect_kinds(),
            vec![
                Var, Whitespace, LBrace, RBrace, Whitespace, Var, Whitespace, String, Semi,
                Whitespace, Comment, Whitespace, Alias, Whitespace, Equals, Whitespace, Var,
                Whitespace, Var, Semi, Whitespace
            ]
        );
    }
}
