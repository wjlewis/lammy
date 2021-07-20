//! # Abstract syntax trees

mod from_untyped;

use crate::source::Span;
use std::rc::Rc;

/// Possible input to an REPL.
#[derive(Debug)]
pub enum ReplInput {
    /// A definition, e.g. `Id = x => x`.
    Def(Def),
    /// A term to reduce, e.g. `(x => x x) x => x x`.
    Term(Term),
}

/// A module (file).
#[derive(Debug)]
pub struct Module {
    /// All of the module's imports.
    pub imports: Vec<Import>,
    /// All of the module's definitions.
    pub defs: Vec<Def>,
    pub span: Span,
}

/// A possibly incomplete/incorrect import declaration.
#[derive(Debug)]
pub struct Import {
    /// The aliases (and vars, potentially) mentioned in the import.
    /// In the import `import { Id, K, bad } from "./common";`, the aliases
    /// are `"Id"`, `"K"`, and `"bad"` (even though `"bad"` is a var, not an
    /// alias).
    pub aliases: Vec<Name>,
    /// The import's filepath.
    pub filepath: Option<Filepath>,
    pub span: Span,
}

/// A possibly incomplete/incorrect alias definition.
#[derive(Debug)]
pub struct Def {
    /// The alias being defined (e.g. `"Id"` in `Id = x => x`).
    pub alias: Option<Name>,
    /// The term being associated with the alias (e.g. `x => x` in `Id = x => x`).
    pub body: Option<Term>,
    pub span: Span,
}

/// An import filepath.
#[derive(Debug)]
pub struct Filepath {
    pub text: Rc<String>,
    pub span: Span,
}

/// A possibly incomplete/incorrect lambda calculus term.
#[derive(Debug)]
pub enum Term {
    /// A variable reference (i.e. _not_ a bound variable).
    Var { text: Rc<String>, span: Span },
    /// An alias reference.
    Alias { text: Rc<String>, span: Span },
    /// An abstraction.
    /// Note that the abstraction may or may not contain a body, and that its
    /// `vars` may be empty. The second of these has already been addressed
    /// (i.e. errors have been recorded) during the parsing process; the first
    /// is addressed in the desugaring phase.
    Abs {
        vars: Vec<Name>,
        body: Option<Box<Term>>,
        span: Span,
    },
    /// An application.
    /// Note that the operands (`rands`) may contain no terms.
    App {
        rator: Box<Term>,
        rands: Vec<Term>,
        span: Span,
    },
}

/// A representation of a "name" (text), used for both aliases and vars.
#[derive(Debug, Clone)]
pub struct Name {
    /// The name's text.
    pub text: Rc<String>,
    pub span: Span,
    /// Whether or not the name is "bad": this is `true` if the name is an alias
    /// appearing where a var is expected (e.g. in an abstraction's bound vars),
    /// or a var where an alias is expected (e.g. in an import declaration).
    pub bad: bool,
}
