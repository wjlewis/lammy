mod from_untyped;

use crate::source::Span;
use std::rc::Rc;

#[derive(Debug)]
pub enum ReplInput {
    Def(Def),
    Term(Term),
}

#[derive(Debug)]
pub struct Module {
    pub use_decls: Vec<UseDecl>,
    pub defs: Vec<Def>,
    pub span: Span,
}

#[derive(Debug)]
pub struct UseDecl {
    pub aliases: Option<UseAliases>,
    pub filepath: Option<Filepath>,
    pub span: Span,
}

#[derive(Debug)]
pub struct Def {
    pub alias: Option<Name>,
    pub body: Option<Term>,
    pub span: Span,
}

#[derive(Debug)]
pub struct UseAliases {
    pub aliases: Vec<Name>,
    pub span: Span,
}

#[derive(Debug)]
pub struct Filepath {
    pub text: Rc<String>,
    pub span: Span,
}

#[derive(Debug)]
pub enum Term {
    Var {
        text: Rc<String>,
        span: Span,
    },
    Alias {
        text: Rc<String>,
        span: Span,
    },
    Abs {
        names: Option<AbsVars>,
        body: Option<Box<Term>>,
        span: Span,
    },
    App {
        rator: Option<Box<Term>>,
        rands: Vec<Term>,
        span: Span,
    },
}

#[derive(Debug)]
pub struct AbsVars {
    pub names: Vec<Name>,
    pub span: Span,
}

#[derive(Debug)]
pub struct Name {
    pub text: Rc<String>,
    pub span: Span,
    pub ok: bool,
}
