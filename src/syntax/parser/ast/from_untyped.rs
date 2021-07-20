//! # Conversions from `UntypedTree`s to abstract syntax trees.
//!
//! The trait implementations in this file need to conspire with the parsing
//! functions defined in `../tree_builder.rs` to produce the expected output.
//! Any panics here are the result of a breached contract between the two.

use super::super::untyped_tree::{SyntaxKind as Sk, UntypedTree};
use super::{Def, Filepath, Import, Module, Name, ReplInput, Term};
use crate::syntax::tokens::Token;

use UntypedTree::*;

impl From<UntypedTree> for Option<ReplInput> {
    fn from(tree: UntypedTree) -> Option<ReplInput> {
        match tree {
            Inner {
                kind: Sk::ReplInput,
                children,
                ..
            } => {
                let mut children: Vec<UntypedTree> = skip_concrete(children).collect();

                let input = children.pop()?;
                if input.has_kind(&Sk::Def) {
                    let def: Option<Def> = input.into();
                    def.map(ReplInput::Def)
                } else if input.has_kind(&Sk::Tms) {
                    let term: Option<Term> = input.into();
                    term.map(ReplInput::Term)
                } else {
                    None
                }
            }
            Inner { kind, .. } => panic!(
                "encountered untyped tree of kind {:?} when extracting repl input",
                kind
            ),
            Leaf(..) => {
                panic!("encountered an untyped leaf when extracting repl input")
            }
        }
    }
}

impl From<UntypedTree> for Module {
    fn from(tree: UntypedTree) -> Module {
        match tree {
            Inner {
                kind: Sk::Module,
                span,
                children,
            } => {
                let (imports, defs): (Vec<UntypedTree>, Vec<UntypedTree>) =
                    skip_concrete(children).partition(|tree| tree.is_import());

                let imports = imports
                    .into_iter()
                    .map(<Option<Import>>::from)
                    .collect::<Option<Vec<Import>>>();

                let defs = defs
                    .into_iter()
                    .map(<Option<Def>>::from)
                    .collect::<Option<Vec<Def>>>();

                Module {
                    imports: imports.unwrap_or(Vec::new()),
                    defs: defs.unwrap_or(Vec::new()),
                    span,
                }
            }
            Inner { kind, .. } => panic!(
                "attempted to extract a module from an untyped tree of kind {:?}",
                kind
            ),
            Leaf(..) => panic!("attempted to extract a module from an untyped leaf"),
        }
    }
}

impl From<UntypedTree> for Option<Import> {
    fn from(tree: UntypedTree) -> Option<Import> {
        match tree {
            Inner {
                kind: Sk::Import,
                span,
                children,
            } => {
                let mut children: Vec<UntypedTree> = skip_concrete(children).collect();

                // Note the ordering here
                let filepath = children.pop();
                let aliases = children.pop();

                let aliases = aliases.map(<Vec<Name>>::from).unwrap_or(Vec::new());
                let filepath = filepath.and_then(<Option<Filepath>>::from);

                Some(Import {
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
            Inner {
                kind: Sk::Def,
                span,
                children,
            } => {
                let mut children: Vec<UntypedTree> = skip_concrete(children).collect();

                // Note the ordering here
                let body = children.pop();
                let alias = children.pop();

                let alias = alias.and_then(<Option<Name>>::from);
                let body = body.and_then(<Option<Term>>::from);

                Some(Def { alias, body, span })
            }
            _ => None,
        }
    }
}

impl From<UntypedTree> for Option<Name> {
    fn from(tree: UntypedTree) -> Option<Name> {
        if let Inner {
            kind,
            span,
            mut children,
        } = tree
        {
            match kind {
                Sk::Name | Sk::BadName => match children.pop() {
                    Some(Leaf(Token { text, .. })) => Some(Name {
                        text,
                        span,
                        bad: kind == Sk::BadName,
                    }),
                    _ => None,
                },
                _ => None,
            }
        } else {
            None
        }
    }
}

impl From<UntypedTree> for Option<Filepath> {
    fn from(tree: UntypedTree) -> Option<Filepath> {
        match tree {
            Inner {
                kind: Sk::ImportFilepath,
                span,
                mut children,
            } => match children.pop() {
                Some(Leaf(Token { text, .. })) => Some(Filepath { text, span }),
                _ => None,
            },
            _ => None,
        }
    }
}

impl From<UntypedTree> for Option<Term> {
    fn from(tree: UntypedTree) -> Option<Term> {
        match tree {
            Inner {
                kind: Sk::Tms,
                span,
                children,
            } => {
                let mut children: Vec<UntypedTree> = skip_concrete(children).collect();

                match children.len() {
                    0 => None,
                    1 => children.pop().and_then(UntypedTree::to_term),
                    _ => {
                        let rator = children
                            .remove(0)
                            .to_term()
                            .map(Box::new)
                            .expect("parsed application doesn't include operator term");

                        let rands = children
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
            Inner {
                kind,
                span,
                mut children,
            } => match kind {
                Sk::Var => match children.pop() {
                    Some(Leaf(Token { text, .. })) => Some(Term::Var { text, span }),
                    _ => None,
                },
                Sk::Alias => match children.pop() {
                    Some(Leaf(Token { text, .. })) => Some(Term::Alias { text, span }),
                    _ => None,
                },
                Sk::Abs => {
                    let mut children: Vec<UntypedTree> = skip_concrete(children).collect();

                    // Note the ordering here
                    let body = children.pop();
                    let vars = children.pop();

                    let body = body.and_then(<Option<Term>>::from).map(Box::new);
                    let vars = vars.map(<Vec<Name>>::from).unwrap_or(Vec::new());

                    Some(Term::Abs { vars, body, span })
                }
                Sk::Tms => {
                    let terms = Inner {
                        kind,
                        span,
                        children,
                    };
                    <Option<Term>>::from(terms)
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn is_import(&self) -> bool {
        match self {
            Inner {
                kind: Sk::Import, ..
            } => true,
            Inner { kind: Sk::Def, .. } => false,
            Inner { kind, .. } => {
                panic!(
                    "encountered an untyped tree of kind {:?} when extracting module",
                    kind
                )
            }
            Leaf(..) => {
                panic!("encountered an untyped leaf when extracting module")
            }
        }
    }
}

impl From<UntypedTree> for Vec<Name> {
    fn from(tree: UntypedTree) -> Vec<Name> {
        match tree {
            Inner {
                kind: Sk::AbsVars,
                children,
                ..
            } => {
                let names: Option<Vec<Name>> =
                    skip_concrete(children).map(<Option<Name>>::from).collect();
                names.unwrap_or(Vec::new())
            }
            _ => Vec::new(),
        }
    }
}

/// Skips unimportant leaf nodes, leaving an iterator over the important ones.
fn skip_concrete(children: Vec<UntypedTree>) -> impl Iterator<Item = UntypedTree> {
    children.into_iter().filter(|child| !child.is_leaf())
}
