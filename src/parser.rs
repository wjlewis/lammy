mod tree_builder;
mod untyped;

// Temporary
pub use self::tree_builder::{ParseResult, TreeBuilder};

use self::untyped::{SyntaxKind, UntypedTree};
use crate::lexer::Token;
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
    pub alias: Option<DefAlias>,
    pub body: Option<Term>,
    pub span: Span,
}

#[derive(Debug)]
pub struct UseAliases {
    pub aliases: Vec<UseAlias>,
    pub span: Span,
}

#[derive(Debug)]
pub struct UseAlias {
    pub text: Rc<String>,
    pub span: Span,
    pub ok: bool,
}

#[derive(Debug)]
pub struct Filepath {
    pub text: Rc<String>,
    pub span: Span,
}

#[derive(Debug)]
pub struct DefAlias {
    pub text: Rc<String>,
    pub span: Span,
    pub ok: bool,
}

#[derive(Debug)]
pub enum Term {
    Name {
        text: Rc<String>,
        span: Span,
    },
    Alias {
        text: Rc<String>,
        span: Span,
    },
    Abs {
        names: Option<AbsNames>,
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
pub struct AbsNames {
    pub names: Vec<AbsName>,
    pub span: Span,
}

#[derive(Debug)]
pub struct AbsName {
    pub text: Rc<String>,
    pub span: Span,
    pub ok: bool,
}

impl From<UntypedTree> for Option<ReplInput> {
    fn from(tree: UntypedTree) -> Option<ReplInput> {
        match tree {
            UntypedTree::Inner {
                kind: SyntaxKind::ReplInput,
                children,
                ..
            } => {
                let mut children = children
                    .into_iter()
                    .filter(|child| !child.is_concrete())
                    .collect::<Vec<UntypedTree>>();
                let input = children.pop()?;
                if input.has_kind(&SyntaxKind::Def) {
                    let def: Option<Def> = input.into();
                    def.map(ReplInput::Def)
                } else if input.has_kind(&SyntaxKind::Tms) {
                    let term: Option<Term> = input.into();
                    term.map(ReplInput::Term)
                } else {
                    None
                }
            }
            UntypedTree::Inner { kind, .. } => panic!(
                "encountered untyped tree of kind {:?} when extracting repl input",
                kind
            ),
            UntypedTree::Leaf(..) => {
                panic!("encountered an untyped leaf when extracting repl input")
            }
        }
    }
}

impl From<UntypedTree> for Module {
    fn from(tree: UntypedTree) -> Module {
        match tree {
            UntypedTree::Inner {
                kind: SyntaxKind::Module,
                span,
                children,
            } => {
                let (use_decls, defs): (Vec<UntypedTree>, Vec<UntypedTree>) = children
                    .into_iter()
                    .filter(|child| !child.is_concrete())
                    .partition(|tree| match tree {
                        UntypedTree::Inner {
                            kind: SyntaxKind::Use,
                            ..
                        } => true,
                        UntypedTree::Inner {
                            kind: SyntaxKind::Def,
                            ..
                        } => false,
                        UntypedTree::Inner { kind, .. } => {
                            panic!(
                                "encountered an untyped tree of kind {:?} when extracting module",
                                kind
                            )
                        }
                        UntypedTree::Leaf(..) => {
                            panic!("encountered an untyped leaf when extracting module")
                        }
                    });

                let use_decls = use_decls
                    .into_iter()
                    .map(|decl| decl.into())
                    .collect::<Option<Vec<UseDecl>>>();

                let defs = defs
                    .into_iter()
                    .map(|def| def.into())
                    .collect::<Option<Vec<Def>>>();

                Module {
                    use_decls: use_decls.unwrap_or(Vec::new()),
                    defs: defs.unwrap_or(Vec::new()),
                    span,
                }
            }
            UntypedTree::Inner { kind, .. } => panic!(
                "attempted to extract a module from an untyped tree of kind {:?}",
                kind
            ),
            UntypedTree::Leaf(..) => panic!("attempted to extract a module from an untyped leaf"),
        }
    }
}

impl From<UntypedTree> for Option<UseDecl> {
    fn from(tree: UntypedTree) -> Option<UseDecl> {
        match tree {
            UntypedTree::Inner {
                kind: SyntaxKind::Use,
                span,
                children,
            } => {
                let mut children = children
                    .into_iter()
                    .filter(|child| !child.is_concrete())
                    .collect::<Vec<UntypedTree>>();

                // Note the ordering here
                let filepath = children.pop();
                let aliases = children.pop();

                let aliases = aliases.and_then(<Option<UseAliases>>::from);
                let filepath = filepath.and_then(<Option<Filepath>>::from);

                Some(UseDecl {
                    aliases,
                    filepath,
                    span,
                })
            }
            _ => None,
        }
    }
}

impl From<UntypedTree> for Option<Def> {
    fn from(tree: UntypedTree) -> Option<Def> {
        match tree {
            UntypedTree::Inner {
                kind: SyntaxKind::Def,
                span,
                children,
            } => {
                let mut children = children
                    .into_iter()
                    .filter(|child| !child.is_concrete())
                    .collect::<Vec<UntypedTree>>();

                // Note the ordering here
                let body = children.pop();
                let alias = children.pop();

                let alias = alias.and_then(|tree| tree.into());
                let body = body.and_then(|tree| tree.into());

                Some(Def { alias, body, span })
            }
            _ => None,
        }
    }
}

impl From<UntypedTree> for Option<UseAliases> {
    fn from(tree: UntypedTree) -> Option<UseAliases> {
        match tree {
            UntypedTree::Inner {
                kind: SyntaxKind::UseAliases,
                span,
                children,
            } => {
                let aliases = children
                    .into_iter()
                    .filter(|child| !child.is_concrete())
                    .map(<Option<UseAlias>>::from)
                    .collect::<Option<Vec<UseAlias>>>();
                aliases.map(|aliases| UseAliases { aliases, span })
            }
            _ => None,
        }
    }
}

impl From<UntypedTree> for Option<UseAlias> {
    fn from(tree: UntypedTree) -> Option<UseAlias> {
        match tree {
            UntypedTree::Inner {
                kind: SyntaxKind::UseAlias,
                span,
                mut children,
            } => match children.pop() {
                Some(UntypedTree::Leaf(Token { text, .. })) => Some(UseAlias {
                    text,
                    span,
                    ok: true,
                }),
                _ => None,
            },
            UntypedTree::Inner {
                kind: SyntaxKind::BadUseAlias,
                span,
                mut children,
            } => match children.pop() {
                Some(UntypedTree::Leaf(Token { text, .. })) => Some(UseAlias {
                    text,
                    span,
                    ok: false,
                }),
                _ => None,
            },
            _ => None,
        }
    }
}

impl From<UntypedTree> for Option<Filepath> {
    fn from(tree: UntypedTree) -> Option<Filepath> {
        match tree {
            UntypedTree::Inner {
                kind: SyntaxKind::UseFilepath,
                span,
                mut children,
            } => match children.pop() {
                Some(UntypedTree::Leaf(Token { text, .. })) => Some(Filepath { text, span }),
                _ => None,
            },
            _ => None,
        }
    }
}

impl From<UntypedTree> for Option<DefAlias> {
    fn from(tree: UntypedTree) -> Option<DefAlias> {
        match tree {
            UntypedTree::Inner {
                kind: SyntaxKind::DefAlias,
                span,
                mut children,
            } => match children.pop() {
                Some(UntypedTree::Leaf(Token { text, .. })) => Some(DefAlias {
                    text,
                    span,
                    ok: true,
                }),
                _ => None,
            },
            UntypedTree::Inner {
                kind: SyntaxKind::BadDefAlias,
                span,
                mut children,
            } => match children.pop() {
                Some(UntypedTree::Leaf(Token { text, .. })) => Some(DefAlias {
                    text,
                    span,
                    ok: false,
                }),
                _ => None,
            },
            _ => None,
        }
    }
}

impl From<UntypedTree> for Option<Term> {
    fn from(tree: UntypedTree) -> Option<Term> {
        match tree {
            UntypedTree::Inner {
                kind: SyntaxKind::Tms,
                span,
                children,
            } => {
                let mut filtered = children
                    .into_iter()
                    .filter(|child| !child.is_concrete())
                    .collect::<Vec<UntypedTree>>();

                match filtered.len() {
                    0 => None,
                    1 => filtered.pop().and_then(UntypedTree::to_term),
                    _ => {
                        let rator = filtered.remove(0).to_term().map(Box::new);
                        let rands = filtered
                            .into_iter()
                            .map(UntypedTree::to_term)
                            .collect::<Option<Vec<Term>>>()
                            .unwrap_or(Vec::new());
                        Some(Term::App { rator, rands, span })
                    }
                }
            }
            _ => None,
        }
    }
}

impl UntypedTree {
    fn to_term(self) -> Option<Term> {
        match self {
            UntypedTree::Inner {
                kind,
                span,
                mut children,
            } => match kind {
                SyntaxKind::Name => match children.pop() {
                    Some(UntypedTree::Leaf(Token { text, .. })) => Some(Term::Name { text, span }),
                    _ => None,
                },
                SyntaxKind::Alias => match children.pop() {
                    Some(UntypedTree::Leaf(Token { text, .. })) => Some(Term::Alias { text, span }),
                    _ => None,
                },
                SyntaxKind::Abs => {
                    let mut children = children
                        .into_iter()
                        .filter(|child| !child.is_concrete())
                        .collect::<Vec<UntypedTree>>();

                    // Note the ordering here
                    let body = children.pop();
                    let names = children.pop();

                    let body = body.and_then(<Option<Term>>::from).map(Box::new);
                    let names = names.and_then(|names| names.into());

                    Some(Term::Abs { names, body, span })
                }
                SyntaxKind::Tms => (UntypedTree::Inner {
                    kind,
                    span,
                    children,
                })
                .into(),
                _ => None,
            },
            _ => None,
        }
    }
}

impl From<UntypedTree> for Option<AbsNames> {
    fn from(tree: UntypedTree) -> Option<AbsNames> {
        match tree {
            UntypedTree::Inner {
                kind: SyntaxKind::AbsNames,
                span,
                children,
            } => {
                let names = children
                    .into_iter()
                    .filter(|child| !child.is_concrete())
                    .map(|child| child.into())
                    .collect::<Option<Vec<AbsName>>>();
                names.map(|names| AbsNames { names, span })
            }
            _ => None,
        }
    }
}

impl From<UntypedTree> for Option<AbsName> {
    fn from(tree: UntypedTree) -> Option<AbsName> {
        match tree {
            UntypedTree::Inner {
                kind: SyntaxKind::AbsName,
                span,
                mut children,
            } => match children.pop() {
                Some(UntypedTree::Leaf(Token { text, .. })) => Some(AbsName {
                    text,
                    span,
                    ok: true,
                }),
                _ => None,
            },
            UntypedTree::Inner {
                kind: SyntaxKind::BadAbsName,
                span,
                mut children,
            } => match children.pop() {
                Some(UntypedTree::Leaf(Token { text, .. })) => Some(AbsName {
                    text,
                    span,
                    ok: false,
                }),
                _ => None,
            },
            _ => None,
        }
    }
}
