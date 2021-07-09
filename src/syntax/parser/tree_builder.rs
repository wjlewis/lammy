use super::untyped_tree::{SyntaxKind as Sk, UntypedTree};
use crate::errors::SimpleError;
use crate::source::{Source, SourceInfo};
use crate::syntax::lexer::{Lexer, Token, TokenKind as Tk};
use std::rc::Rc;

pub struct TreeBuilder<'a> {
    source: Rc<Source>,
    tokens: Lexer<'a>,
    wip: Vec<Entry>,
    errors: Vec<SimpleError>,
    pos: usize,
}

#[derive(Debug)]
pub struct ParseResult {
    pub tree: UntypedTree,
    pub errors: Vec<SimpleError>,
}

impl<'a> TreeBuilder<'a> {
    pub fn new(source: &'a Rc<Source>) -> Self {
        TreeBuilder {
            source: Rc::clone(source),
            tokens: Lexer::new(source),
            wip: Vec::new(),
            errors: Vec::new(),
            pos: 0,
        }
    }

    pub fn take(mut self) -> ParseResult {
        match self.wip.pop() {
            None => panic!("no tree to take"),
            Some(entry) => match entry {
                Entry::InProgress { kind, .. } => panic!("unmatched `open` ({:?})", kind),
                Entry::Complete(tree) => {
                    if self.wip.is_empty() {
                        ParseResult {
                            tree,
                            errors: self.errors,
                        }
                    } else {
                        panic!("multiple toplevel trees")
                    }
                }
            },
        }
    }

    pub fn parse_repl_input(&mut self) {
        self.open(Sk::ReplInput);
        self.skip_trivia();
        let peek = self.tokens.peek();
        let kind = peek.kind;
        let info = peek.info.clone();
        match kind {
            Tk::Alias | Tk::Name if self.starts_def() => self.parse_def(),
            Tk::Equals => self.parse_def(),
            Tk::Name | Tk::Alias | Tk::LParen | Tk::Comma | Tk::Arrow => self.parse_tms(),
            _ => self.error("expected a definition or term before this", info),
        }

        self.skip_trivia();
        let start_info = self.tokens.peek().info.clone();
        let end_info = loop {
            let peek = self.tokens.peek();
            match peek.kind {
                Tk::Eof => break peek.info.clone(),
                _ => self.pop_leaf(),
            }
        };

        if start_info.span != end_info.span {
            self.error("extraneous input", start_info.combine_with(end_info));
        }

        self.close(Sk::ReplInput);
    }

    pub fn parse_module(&mut self) {
        self.open(Sk::Module);
        loop {
            self.skip_trivia();
            let peek = self.tokens.peek();
            let kind = peek.kind;
            let info = peek.info.clone();
            match kind {
                Tk::Eof => break,
                Tk::Name if *peek.text == "use" => self.parse_use(),
                Tk::LBrace | Tk::RBrace | Tk::String | Tk::UnterminatedString => self.parse_use(),
                Tk::Alias | Tk::Name if self.starts_def() => self.parse_def(),
                Tk::Equals => self.parse_def(),
                Tk::Semi => self.error("extraneous ';'", info),
                _ => {
                    let info = self.skip_to_decl_separator();
                    self.error("expected definition or use declaration here", info);
                }
            }

            self.skip_trivia();
            let peek = self.tokens.peek();
            match peek.kind {
                Tk::Semi => self.pop_leaf(),
                Tk::Eof => {
                    let info = peek.info.clone();
                    self.error("missing a ';'", info);
                    break;
                }
                _ => {
                    let info = self.skip_to_decl_separator();
                    self.error("extraneous input", info);

                    debug_assert!(match self.tokens.peek().kind {
                        Tk::Semi | Tk::Eof => true,
                        _ => false,
                    });
                    self.pop_leaf();
                }
            }
        }
        self.close(Sk::Module);
    }

    fn skip_to_decl_separator(&mut self) -> SourceInfo {
        let start_info = self.tokens.peek().info.clone();
        let end_info = loop {
            let peek = self.tokens.peek();
            match peek.kind {
                Tk::Semi | Tk::Eof => break peek.info.clone(),
                _ => self.pop_leaf(),
            }
        };
        start_info.combine_with(end_info)
    }

    fn parse_def(&mut self) {
        debug_assert!(match self.tokens.peek().kind {
            Tk::Alias | Tk::Name | Tk::Equals => true,
            _ => false,
        });

        self.open(Sk::Def);

        let peek = self.tokens.peek();
        match peek.kind {
            Tk::Alias => {
                self.open(Sk::Name);
                self.pop_leaf();
                self.close(Sk::Name);
            }
            Tk::Name => {
                let info = peek.info.clone();
                self.error("expected an alias, not a var", info);
                self.open(Sk::BadName);
                self.pop_leaf();
                self.close(Sk::BadName);
            }
            Tk::Equals => {
                let info = peek.info.clone();
                self.error("expected an alias name before this", info);
                self.dummy();
            }
            _ => unreachable!(),
        }

        self.skip_trivia();
        let peek = self.tokens.peek();
        match peek.kind {
            Tk::Equals => self.pop_leaf(),
            Tk::Name | Tk::Alias | Tk::LParen | Tk::Comma | Tk::Arrow => {
                let info = peek.info.clone();
                self.error("expected an '=' before this", info);
            }
            _ => {
                let info = peek.info.clone();
                self.error("expected an '=', followed by a term before this", info);
                self.dummy();
                self.close(Sk::Def);
                return;
            }
        }

        self.skip_trivia();
        self.parse_tms();
        self.close(Sk::Def);
    }

    fn parse_use(&mut self) {
        debug_assert!(match self.tokens.peek().kind {
            Tk::Name | Tk::LBrace | Tk::RBrace | Tk::String | Tk::UnterminatedString => true,
            _ => false,
        });

        self.open(Sk::Use);

        let peek = self.tokens.peek();
        match peek.kind {
            Tk::Name if *peek.text == "use" => self.pop_leaf(),
            Tk::LBrace
            | Tk::Alias
            | Tk::Name
            | Tk::Comma
            | Tk::RBrace
            | Tk::String
            | Tk::UnterminatedString => {
                let info = peek.info.clone();
                self.error("expected 'use' before this", info);
            }
            _ => unreachable!(),
        }

        self.skip_trivia();
        self.parse_use_aliases();

        self.skip_trivia();
        let peek = self.tokens.peek();
        match peek.kind {
            Tk::Name if *peek.text == "from" => self.pop_leaf(),
            Tk::String | Tk::UnterminatedString => {
                let info = peek.info.clone();
                self.error("expected 'from' before this", info);
            }
            _ => {
                let info = peek.info.clone();
                self.error("expected 'from', followed by a filepath before this", info);
                self.dummy();
                self.close(Sk::Use);
                return;
            }
        }

        self.skip_trivia();
        let peek = self.tokens.peek();
        match peek.kind {
            Tk::String => {
                self.open(Sk::UseFilepath);
                self.pop_leaf();
                self.close(Sk::UseFilepath);
            }
            Tk::UnterminatedString => {
                let info = peek.info.clone();
                self.error("unterminated filepath", info);
                self.open(Sk::UseFilepath);
                self.pop_leaf();
                self.close(Sk::UseFilepath);
            }
            _ => {
                let info = peek.info.clone();
                self.error("expected a filepath before this", info);
                self.dummy();
                self.close(Sk::Use);
                return;
            }
        }

        self.close(Sk::Use);
    }

    fn parse_use_aliases(&mut self) {
        debug_assert!(self.tokens.peek().is_nontrivial());

        let peek = self.tokens.peek();
        let info = peek.info.clone();
        match peek.kind {
            Tk::LBrace => {
                self.open(Sk::UseAliases);
                self.pop_leaf();
            }
            Tk::Alias | Tk::Name | Tk::Comma | Tk::RBrace => {
                self.open(Sk::UseAliases);
                self.error("expected a '{' before this", info);
            }
            _ => {
                self.error(
                    "expected a list of aliases enclosed in '{ .. }' before this",
                    info,
                );
                self.dummy();
                return;
            }
        }

        loop {
            self.skip_trivia();
            let peek = self.tokens.peek();
            match peek.kind {
                Tk::Alias => {
                    self.open(Sk::Name);
                    self.pop_leaf();
                    self.close(Sk::Name);
                }
                Tk::Name => {
                    let info = peek.info.clone();
                    self.error("expected an alias here, not a name", info);
                    self.open(Sk::BadName);
                    self.pop_leaf();
                    self.close(Sk::BadName);
                }
                Tk::RBrace => {
                    self.pop_leaf();
                    break;
                }
                Tk::Comma => {
                    let info = peek.info.clone();
                    self.error("extraneous ','", info);
                }
                _ => {
                    let info = peek.info.clone();
                    self.error("expected a '}' before this", info);
                    break;
                }
            }

            self.skip_trivia();
            let peek = self.tokens.peek();
            match peek.kind {
                Tk::Comma => self.pop_leaf(),
                Tk::RBrace => {
                    self.pop_leaf();
                    break;
                }
                Tk::Alias | Tk::Name => {
                    let info = peek.info.clone();
                    self.error("expected a ',' before this", info);
                }
                _ => {
                    let info = peek.info.clone();
                    self.error("expected a '}' before this", info);
                    break;
                }
            }
        }

        self.close(Sk::UseAliases);
    }

    fn parse_tms(&mut self) {
        debug_assert!(self.tokens.peek().is_nontrivial());
        self.open(Sk::Tms);
        self.parse_tm();

        loop {
            self.skip_trivia();
            let peek = self.tokens.peek();
            match peek.kind {
                Tk::Name | Tk::Alias | Tk::LParen | Tk::Comma | Tk::Arrow => self.parse_tm(),
                _ => break,
            }
        }

        self.close(Sk::Tms);
    }

    fn parse_tm(&mut self) {
        debug_assert!(self.tokens.peek().is_nontrivial());
        let peek = self.tokens.peek();
        let info = peek.info.clone();
        match peek.kind.clone() {
            Tk::Name if self.starts_single_abs() => self.parse_single_abs(),
            Tk::Name => self.parse_name(),
            Tk::Alias => self.parse_alias(),
            Tk::LParen if self.starts_abs_names() => self.parse_multi_abs(),
            Tk::LParen => self.parse_parend(),
            Tk::Comma => self.parse_multi_abs(),
            Tk::Arrow => self.parse_abs_from_arrow(),
            _ => self.error("expected a term before this", info),
        }
    }

    fn parse_single_abs(&mut self) {
        debug_assert!(self.tokens.peek().kind == Tk::Name);
        self.open(Sk::Abs);
        self.open(Sk::AbsVars);
        self.open(Sk::Name);
        self.pop_leaf();
        self.close(Sk::Name);
        self.close(Sk::AbsVars);

        self.skip_trivia();
        self.parse_abs_after_names();

        self.close(Sk::Abs);
    }

    fn parse_multi_abs(&mut self) {
        debug_assert!(match self.tokens.peek().kind {
            Tk::LParen | Tk::Comma => true,
            _ => false,
        });

        self.open(Sk::Abs);
        self.parse_abs_names();

        self.skip_trivia();
        self.parse_abs_after_names();

        self.close(Sk::Abs);
    }

    fn parse_abs_from_arrow(&mut self) {
        debug_assert!(self.tokens.peek().kind == Tk::Arrow);

        self.open(Sk::Abs);
        self.dummy();

        let arrow_info = self.tokens.peek().info.clone();
        self.error("expected abstraction name(s) before this", arrow_info);

        self.skip_trivia();
        self.parse_abs_after_names();

        self.close(Sk::Abs);
    }

    fn parse_abs_after_names(&mut self) {
        debug_assert!(self.tokens.peek().is_nontrivial());
        let peek = self.tokens.peek();
        match peek.kind {
            Tk::Arrow => self.pop_leaf(),
            Tk::Name | Tk::Alias | Tk::LParen | Tk::Comma => {
                let info = peek.info.clone();
                self.error("expected an '=>' before this", info);
            }
            _ => {
                let info = peek.info.clone();
                self.error("expected an '=>', followed by a term before this", info);
                self.dummy();
                return;
            }
        }

        self.skip_trivia();
        self.parse_tms();
    }

    fn parse_abs_names(&mut self) {
        debug_assert!(match self.tokens.peek().kind {
            Tk::LParen | Tk::Comma => true,
            _ => false,
        });

        self.open(Sk::AbsVars);
        let peek = self.tokens.peek();
        match peek.kind {
            Tk::LParen => self.pop_leaf(),
            Tk::Comma => {
                let info = peek.info.clone();
                self.error("expected a '(' before this", info);
            }
            _ => unreachable!(),
        }

        loop {
            self.skip_trivia();
            let peek = self.tokens.peek();
            match peek.kind {
                Tk::Name => {
                    self.open(Sk::Name);
                    self.pop_leaf();
                    self.close(Sk::Name);
                }
                Tk::Alias => {
                    let info = peek.info.clone();
                    self.error("expected a var here, not an alias", info);
                    self.open(Sk::BadName);
                    self.pop_leaf();
                    self.close(Sk::BadName);
                }
                Tk::RParen => {
                    self.pop_leaf();
                    break;
                }
                Tk::Comma => {
                    let info = peek.info.clone();
                    self.error("extraneous ','", info);
                }
                _ => {
                    let info = peek.info.clone();
                    self.error("expected a ')' before this", info);
                    break;
                }
            }

            self.skip_trivia();
            let peek = self.tokens.peek();
            match peek.kind {
                Tk::Comma => self.pop_leaf(),
                Tk::RParen => {
                    self.pop_leaf();
                    break;
                }
                Tk::Name | Tk::Alias => {
                    let info = peek.info.clone();
                    self.error("expected a ',' before this", info);
                }
                _ => {
                    let info = peek.info.clone();
                    self.error("expected a ')' before this", info);
                    break;
                }
            }
        }

        self.close(Sk::AbsVars);
    }

    fn parse_name(&mut self) {
        debug_assert!(self.tokens.peek().kind == Tk::Name);
        self.open(Sk::Var);
        self.pop_leaf();
        self.close(Sk::Var);
    }

    fn parse_alias(&mut self) {
        debug_assert!(self.tokens.peek().kind == Tk::Alias);
        self.open(Sk::Alias);
        self.pop_leaf();
        self.close(Sk::Alias);
    }

    fn parse_parend(&mut self) {
        debug_assert!(self.tokens.peek().kind == Tk::LParen);
        let lparen = self.tokens.pop();
        let lparen_info = lparen.info.clone();
        self.leaf(lparen);

        self.skip_trivia();
        self.parse_tms();

        self.skip_trivia();
        match self.tokens.peek().kind {
            Tk::RParen => self.pop_leaf(),
            _ => self.error("unmatched '('", lparen_info),
        }
    }

    fn starts_single_abs(&mut self) -> bool {
        debug_assert!(self.tokens.peek().kind == Tk::Name);

        let mut peek_cursor = 1;
        loop {
            let peek = self.tokens.peek_ahead(peek_cursor);
            match peek.kind {
                _ if peek.is_trivial() => {}
                Tk::Arrow => break true,
                _ => break false,
            }
            peek_cursor += 1;
        }
    }

    fn starts_abs_names(&mut self) -> bool {
        debug_assert!(self.tokens.peek().kind == Tk::LParen);

        let mut peek_cursor = 1;
        loop {
            let peek = self.tokens.peek_ahead(peek_cursor);
            match peek.kind {
                _ if peek.is_trivial() => {}
                Tk::Name | Tk::Alias => {}
                Tk::Comma => break true,
                Tk::RParen => {
                    peek_cursor += 1;
                    loop {
                        let peek = self.tokens.peek_ahead(peek_cursor);
                        match peek.kind {
                            _ if peek.is_trivial() => {}
                            Tk::Arrow => return true,
                            _ => return false,
                        }
                        peek_cursor += 1;
                    }
                }
                _ => break false,
            }
            peek_cursor += 1;
        }
    }

    fn starts_def(&mut self) -> bool {
        debug_assert!(match self.tokens.peek().kind {
            Tk::Alias | Tk::Name => true,
            _ => false,
        });

        let mut peek_cursor = 1;
        loop {
            let peek = self.tokens.peek_ahead(peek_cursor);
            match peek.kind {
                _ if peek.is_trivial() => {}
                Tk::Equals => break true,
                _ => break false,
            }
            peek_cursor += 1;
        }
    }

    fn skip_trivia(&mut self) {
        loop {
            let peek = self.tokens.peek();
            match peek.kind {
                Tk::Whitespace | Tk::Comment => self.pop_leaf(),
                Tk::Unknown => {
                    let info = peek.info.clone();
                    self.error("unknown token", info);
                    self.pop_leaf();
                }
                _ => break,
            }
        }
    }

    fn pop_leaf(&mut self) {
        let next = self.tokens.pop();
        self.leaf(next);
    }

    fn leaf(&mut self, token: Token) {
        self.pos = token.info.span.end;
        self.wip.push(Entry::Complete(UntypedTree::Leaf(token)))
    }

    fn open(&mut self, kind: Sk) {
        self.wip.push(Entry::InProgress {
            kind,
            start: self.pos,
        });
    }

    fn close(&mut self, kind: Sk) {
        let mut children = Vec::new();
        while let Some(entry) = self.wip.pop() {
            match entry {
                Entry::InProgress {
                    kind: open_kind,
                    start,
                } => {
                    if open_kind != kind {
                        panic!(
                            "`open` and `close` kinds don't match ({:?} != {:?})",
                            open_kind, kind
                        );
                    }

                    children.reverse();
                    self.wip.push(Entry::Complete(UntypedTree::Inner {
                        kind,
                        info: SourceInfo::new(Rc::clone(&self.source), start, self.pos),
                        children,
                    }));
                    return;
                }
                Entry::Complete(child) => {
                    children.push(child);
                }
            }
        }
    }

    fn error(&mut self, message: impl Into<String>, info: SourceInfo) {
        self.errors.push(SimpleError::new(message, info));
    }

    fn dummy(&mut self) {
        self.open(Sk::Dummy);
        self.close(Sk::Dummy);
    }
}

enum Entry {
    InProgress { kind: Sk, start: usize },
    Complete(UntypedTree),
}
