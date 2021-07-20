//! A parser that produces untyped, full-fidelity trees.

use super::untyped_tree::{SyntaxKind as Sk, UntypedTree};
use crate::errors::SimpleError;
use crate::source::Span;
use crate::syntax::lexer::Lexer;
use crate::syntax::tokens::{Token, TokenKind as Tk};

/// A stateful tree building device.
pub struct TreeBuilder<'a> {
    /// The source of tokens used to construct a tree.
    tokens: Lexer<'a>,
    /// A stack of in-progress and completed tree nodes. In-progress nodes are
    /// pushed onto the stack when the appropriate tokens are encountered, and
    /// then later "completed".
    wip: Vec<Entry>,
    /// An "error sink", used to accumulate errors that occur during parsing.
    /// Note that all parsing errors may be represented as `SimpleError`s (i.e.
    /// an error with a single span).
    errors: Vec<SimpleError>,
    /// The end position of the `Span` of the last token that was popped. We
    /// keep track of this in order to construct spans for entire trees.
    pos: usize,
}

/// The result of parsing a construct.
/// Note that parsing always succeeds in producing _some_ tree; if the tree is
/// incomplete/incorrect, errors will be returned as well.
#[derive(Debug)]
pub struct ParseResult {
    pub tree: UntypedTree,
    pub errors: Vec<SimpleError>,
}

impl<'a> TreeBuilder<'a> {
    /// Parses input to the REPL (e.g. definitions, terms, special commands).
    pub fn parse_repl_input(source: &'a str) -> ParseResult {
        let mut builder = TreeBuilder::from(source);
        builder._parse_repl_input();
        builder.take()
    }

    /// Parses a module (file).
    pub fn parse_module(source: &'a str) -> ParseResult {
        let mut builder = TreeBuilder::from(source);
        builder._parse_module();
        builder.take()
    }

    fn _parse_repl_input(&mut self) {
        self.open(Sk::ReplInput);
        self.skip_trivia();
        let peek = self.tokens.peek();
        let kind = peek.kind;
        let span = peek.span.clone();
        match kind {
            Tk::Alias | Tk::Name if self.starts_def() => self.parse_def(),
            Tk::Equals => self.parse_def(),
            Tk::Name | Tk::Alias | Tk::LParen | Tk::Comma | Tk::Arrow => self.parse_tms(),
            _ => self.error("expected a definition or term before this", span),
        }

        self.skip_trivia();
        let start_span = self.tokens.peek().span.clone();
        let end_span = loop {
            let peek = self.tokens.peek();
            match peek.kind {
                Tk::Eof => break peek.span.clone(),
                _ => self.pop_leaf(),
            }
        };

        if start_span != end_span {
            self.error("extraneous input", start_span.combine_with(end_span));
        }

        self.close(Sk::ReplInput);
    }

    fn _parse_module(&mut self) {
        self.open(Sk::Module);
        loop {
            self.skip_trivia();
            let peek = self.tokens.peek();
            let kind = peek.kind;
            let span = peek.span.clone();
            match kind {
                Tk::Eof => break,
                Tk::Name if *peek.text == "import" => self.parse_import(),
                Tk::LBrace | Tk::RBrace | Tk::String | Tk::UnterminatedString => {
                    self.parse_import()
                }
                Tk::Alias | Tk::Name if self.starts_def() => self.parse_def(),
                Tk::Equals => self.parse_def(),
                Tk::Semi => self.error("extraneous ';'", span),
                _ => {
                    let span = self.skip_to_decl_separator();
                    self.error("expected definition or import declaration here", span);
                }
            }

            self.skip_trivia();
            let peek = self.tokens.peek();
            match peek.kind {
                Tk::Semi => self.pop_leaf(),
                Tk::Eof => {
                    let span = peek.span.clone();
                    self.error("missing a ';'", span);
                    break;
                }
                _ => {
                    let span = self.skip_to_decl_separator();
                    self.error("extraneous input", span);

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

    fn skip_to_decl_separator(&mut self) -> Span {
        let start_span = self.tokens.peek().span.clone();
        let end_span = loop {
            let peek = self.tokens.peek();
            match peek.kind {
                Tk::Semi | Tk::Eof => break peek.span.clone(),
                _ => self.pop_leaf(),
            }
        };
        start_span.combine_with(end_span)
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
                let span = peek.span.clone();
                self.error("expected an alias, not a var", span);
                self.open(Sk::BadName);
                self.pop_leaf();
                self.close(Sk::BadName);
            }
            Tk::Equals => {
                let span = peek.span.clone();
                self.error("expected an alias name before this", span);
                self.missing();
            }
            _ => unreachable!(),
        }

        self.skip_trivia();
        let peek = self.tokens.peek();
        match peek.kind {
            Tk::Equals => self.pop_leaf(),
            Tk::Name | Tk::Alias | Tk::LParen | Tk::Comma | Tk::Arrow => {
                let span = peek.span.clone();
                self.error("expected an '=' before this", span);
            }
            _ => {
                let span = peek.span.clone();
                self.error("expected an '=', followed by a term before this", span);
                self.missing();
                self.close(Sk::Def);
                return;
            }
        }

        self.skip_trivia();
        self.parse_tms();
        self.close(Sk::Def);
    }

    fn parse_import(&mut self) {
        debug_assert!(match self.tokens.peek().kind {
            Tk::Name | Tk::LBrace | Tk::RBrace | Tk::String | Tk::UnterminatedString => true,
            _ => false,
        });

        self.open(Sk::Import);

        let peek = self.tokens.peek();
        match peek.kind {
            Tk::Name if *peek.text == "import" => self.pop_leaf(),
            Tk::LBrace
            | Tk::Alias
            | Tk::Name
            | Tk::Comma
            | Tk::RBrace
            | Tk::String
            | Tk::UnterminatedString => {
                let span = peek.span.clone();
                self.error("expected 'import' before this", span);
            }
            _ => unreachable!(),
        }

        self.skip_trivia();
        self.parse_import_aliases();

        self.skip_trivia();
        let peek = self.tokens.peek();
        match peek.kind {
            Tk::Name if *peek.text == "from" => self.pop_leaf(),
            Tk::String | Tk::UnterminatedString => {
                let span = peek.span.clone();
                self.error("expected 'from' before this", span);
            }
            _ => {
                let span = peek.span.clone();
                self.error("expected 'from', followed by a filepath before this", span);
                self.missing();
                self.close(Sk::Import);
                return;
            }
        }

        self.skip_trivia();
        let peek = self.tokens.peek();
        match peek.kind {
            Tk::String => {
                self.open(Sk::ImportFilepath);
                self.pop_leaf();
                self.close(Sk::ImportFilepath);
            }
            Tk::UnterminatedString => {
                let span = peek.span.clone();
                self.error("unterminated filepath", span);
                self.open(Sk::ImportFilepath);
                self.pop_leaf();
                self.close(Sk::ImportFilepath);
            }
            _ => {
                let span = peek.span.clone();
                self.error("expected a filepath before this", span);
                self.missing();
                self.close(Sk::Import);
                return;
            }
        }

        self.close(Sk::Import);
    }

    fn parse_import_aliases(&mut self) {
        debug_assert!(self.tokens.peek().is_nontrivial());

        let peek = self.tokens.peek();
        let span = peek.span.clone();
        match peek.kind {
            Tk::LBrace => {
                self.open(Sk::ImportAliases);
                self.pop_leaf();
            }
            Tk::Alias | Tk::Name | Tk::Comma | Tk::RBrace => {
                self.open(Sk::ImportAliases);
                self.error("expected a '{' before this", span);
            }
            _ => {
                self.error(
                    "expected a list of aliases enclosed in '{..}' before this",
                    span,
                );
                self.missing();
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
                    let span = peek.span.clone();
                    self.error("expected an alias here, not a name", span);
                    self.open(Sk::BadName);
                    self.pop_leaf();
                    self.close(Sk::BadName);
                }
                Tk::RBrace => {
                    self.pop_leaf();
                    break;
                }
                Tk::Comma => {
                    let span = peek.span.clone();
                    self.error("extraneous ','", span);
                }
                _ => {
                    let span = peek.span.clone();
                    self.error("expected a '}' before this", span);
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
                    let span = peek.span.clone();
                    self.error("expected a ',' before this", span);
                }
                _ => {
                    let span = peek.span.clone();
                    self.error("expected a '}' before this", span);
                    break;
                }
            }
        }

        self.close(Sk::ImportAliases);
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
        let span = peek.span.clone();
        match peek.kind.clone() {
            Tk::Name if self.starts_single_abs() => self.parse_single_abs(),
            Tk::Name => self.parse_name(),
            Tk::Alias => self.parse_alias(),
            Tk::LParen if self.starts_abs_names() => self.parse_multi_abs(),
            Tk::LParen => self.parse_parend(),
            Tk::Comma => self.parse_multi_abs(),
            Tk::Arrow => self.parse_abs_from_arrow(),
            _ => self.error("expected a term before this", span),
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
        self.missing();

        let arrow_span = self.tokens.peek().span.clone();
        self.error(
            "expected abstraction var(s) enclosed in '(..)' before this",
            arrow_span,
        );

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
                let span = peek.span.clone();
                self.error("expected an '=>' before this", span);
            }
            _ => {
                let span = peek.span.clone();
                self.error("expected an '=>', followed by a term before this", span);
                self.missing();
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
                let span = peek.span.clone();
                self.error("expected a '(' before this", span);
            }
            _ => unreachable!(),
        }

        let mut seen_name = false;
        loop {
            self.skip_trivia();
            let peek = self.tokens.peek();
            match peek.kind {
                Tk::Name => {
                    self.open(Sk::Name);
                    self.pop_leaf();
                    self.close(Sk::Name);
                    seen_name = true;
                }
                Tk::Alias => {
                    let span = peek.span.clone();
                    self.error("expected a var here, not an alias", span);
                    self.open(Sk::BadName);
                    self.pop_leaf();
                    self.close(Sk::BadName);
                    seen_name = true;
                }
                Tk::RParen => {
                    if !seen_name {
                        let span = peek.span.clone();
                        self.error("expected at least one var before this", span);
                    }
                    self.pop_leaf();
                    break;
                }
                Tk::Comma => {
                    let span = peek.span.clone();
                    self.error("extraneous ','", span);
                }
                _ => {
                    let span = peek.span.clone();
                    if !seen_name {
                        self.error("expected at least one var before this", span.clone());
                    }
                    self.error("expected a ')' before this", span);
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
                    let span = peek.span.clone();
                    self.error("expected a ',' before this", span);
                }
                _ => {
                    let span = peek.span.clone();
                    self.error("expected a ')' before this", span);
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
        let lparen_span = lparen.span.clone();
        self.leaf(lparen);

        self.skip_trivia();
        self.parse_tms();

        self.skip_trivia();
        match self.tokens.peek().kind {
            Tk::RParen => self.pop_leaf(),
            _ => self.error("unmatched '('", lparen_span),
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
                Tk::Comma | Tk::Arrow => break true,
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
                    let span = peek.span.clone();
                    self.error("unknown token", span);
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
        self.pos = token.span.end;
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
                        span: Span::new(start, self.pos),
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

    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.errors.push(SimpleError::new(message, span));
    }

    fn missing(&mut self) {
        self.open(Sk::Missing);
        self.close(Sk::Missing);
    }

    /// Extracts a `ParseResult` from this builder.
    ///
    /// # Panics
    ///
    /// This method panics in three separate situations:
    /// 1. No tree has been started.
    /// 2. The `open` method has been called without a corresponding call to `close`.
    /// 3. Multiple toplevel trees have been created.
    pub fn take(mut self) -> ParseResult {
        match self.wip.pop() {
            None => panic!("no tree to take"),
            Some(Entry::InProgress { kind, .. }) => panic!("unmatched `open` ({:?})", kind),
            Some(Entry::Complete(tree)) => {
                if self.wip.is_empty() {
                    ParseResult {
                        tree,
                        errors: self.errors,
                    }
                } else {
                    panic!("multiple toplevel trees")
                }
            }
        }
    }
}

impl<'a> From<&'a str> for TreeBuilder<'a> {
    fn from(source: &'a str) -> Self {
        TreeBuilder {
            tokens: Lexer::from(source),
            wip: Vec::new(),
            errors: Vec::new(),
            pos: 0,
        }
    }
}

enum Entry {
    InProgress { kind: Sk, start: usize },
    Complete(UntypedTree),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt;
    use std::rc::Rc;

    #[derive(PartialEq)]
    enum KindTree {
        Inner { kind: Sk, children: Vec<KindTree> },
        Leaf(Rc<String>),
    }

    impl fmt::Debug for KindTree {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.fmt_debug(f, 0)
        }
    }

    impl KindTree {
        fn inner(kind: Sk, children: Vec<KindTree>) -> Self {
            KindTree::Inner { kind, children }
        }

        fn leaf(text: &str) -> Self {
            KindTree::Leaf(Rc::new(String::from(text)))
        }

        fn fmt_debug(&self, f: &mut fmt::Formatter, level: usize) -> fmt::Result {
            write!(f, "{}", " ".repeat(level * 2))?;
            match self {
                Kt::Inner { kind, children } => {
                    writeln!(f, "{:?}", kind)?;
                    for child in children {
                        child.fmt_debug(f, level + 1)?;
                    }
                    Ok(())
                }
                Kt::Leaf(text) => writeln!(f, r#""{}""#, text),
            }
        }
    }

    impl From<UntypedTree> for KindTree {
        fn from(tree: UntypedTree) -> Self {
            match tree {
                UntypedTree::Inner { kind, children, .. } => {
                    let children = children.into_iter().map(KindTree::from).collect();
                    KindTree::Inner { kind, children }
                }
                UntypedTree::Leaf(Token { text, .. }) => KindTree::Leaf(text),
            }
        }
    }

    impl ToString for KindTree {
        fn to_string(&self) -> String {
            format!("{:?}", self)
        }
    }

    use KindTree as Kt;

    #[test]
    fn parses_valid_repl_def_correctly() {
        let ParseResult { tree, errors } = TreeBuilder::parse_repl_input("Id = x => x");

        assert!(errors.is_empty());
        let tree = KindTree::from(tree);
        let expected = r#"ReplInput
  Def
    Name
      "Id"
    " "
    "="
    " "
    Tms
      Abs
        AbsVars
          Name
            "x"
        " "
        "=>"
        " "
        Tms
          Var
            "x"
"#;

        assert_eq!(tree.to_string(), expected);
    }

    #[test]
    fn single_abs_start_with_name_arrow() {
        let mut builder = TreeBuilder::from("x => x");
        assert_eq!(builder.starts_single_abs(), true);

        let mut builder = TreeBuilder::from("x # A comment\n => x");
        assert_eq!(builder.starts_single_abs(), true);

        let mut builder = TreeBuilder::from("several names =>");
        assert_eq!(builder.starts_single_abs(), false);
    }

    #[test]
    fn multi_abs_names_start_with_lparen_rparen_arrow() {
        let mut builder = TreeBuilder::from("(x, y) => x");
        assert_eq!(builder.starts_abs_names(), true);

        let mut builder = TreeBuilder::from("(X y # a comment\n => z");
        assert_eq!(builder.starts_abs_names(), true);

        let mut builder = TreeBuilder::from("(X y) z =>");
        assert_eq!(builder.starts_abs_names(), false);
    }

    #[test]
    fn defs_start_with_a_name_followed_by_equals() {
        let mut builder = TreeBuilder::from("Id = x => x;");
        assert_eq!(builder.starts_def(), true);

        let mut builder = TreeBuilder::from("Foo\n\n= On Another Line;");
        assert_eq!(builder.starts_def(), true);

        let mut builder = TreeBuilder::from("bar = bad");
        assert_eq!(builder.starts_def(), true);

        let mut builder = TreeBuilder::from("Quux ( => =");
        assert_eq!(builder.starts_def(), false);
    }
}
