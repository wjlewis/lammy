mod from_untyped;

use crate::source::SourceInfo;
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
    pub info: SourceInfo,
}

#[derive(Debug)]
pub struct UseDecl {
    pub aliases: Option<UseAliases>,
    pub filepath: Option<Filepath>,
    pub info: SourceInfo,
}

#[derive(Debug)]
pub struct Def {
    pub alias: Option<Name>,
    pub body: Option<Term>,
    pub info: SourceInfo,
}

#[derive(Debug)]
pub struct UseAliases {
    pub aliases: Vec<Name>,
    pub info: SourceInfo,
}

#[derive(Debug)]
pub struct Filepath {
    pub text: Rc<String>,
    pub info: SourceInfo,
}

#[derive(Debug)]
pub enum Term {
    Var {
        text: Rc<String>,
        info: SourceInfo,
    },
    Alias {
        text: Rc<String>,
        info: SourceInfo,
    },
    Abs {
        names: Option<AbsVars>,
        body: Option<Box<Term>>,
        info: SourceInfo,
    },
    App {
        rator: Option<Box<Term>>,
        rands: Vec<Term>,
        info: SourceInfo,
    },
}

#[derive(Debug)]
pub struct AbsVars {
    pub names: Vec<Name>,
    pub info: SourceInfo,
}

#[derive(Debug)]
pub struct Name {
    pub text: Rc<String>,
    pub info: SourceInfo,
    pub ok: bool,
}
