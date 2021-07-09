use super::super::untyped_tree::{SyntaxKind as Sk, UntypedTree};
use super::{Def, Filepath, Import, ImportAliases, Module, Name, ReplInput, Term};
use crate::syntax::lexer::Token;

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
                info,
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
                    info,
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
                info,
                children,
            } => {
                let mut children: Vec<UntypedTree> = skip_concrete(children).collect();

                // Note the ordering here
                let filepath = children.pop();
                let aliases = children.pop();

                let aliases = aliases.and_then(<Option<ImportAliases>>::from);
                let filepath = filepath.and_then(<Option<Filepath>>::from);

                Some(Import {
                    aliases,
                    filepath,
                    info,
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
                info,
                children,
            } => {
                let mut children: Vec<UntypedTree> = skip_concrete(children).collect();

                // Note the ordering here
                let body = children.pop();
                let alias = children.pop();

                let alias = alias.and_then(<Option<Name>>::from);
                let body = body.and_then(<Option<Term>>::from);

                Some(Def { alias, body, info })
            }
            _ => None,
        }
    }
}

impl From<UntypedTree> for Option<ImportAliases> {
    fn from(tree: UntypedTree) -> Option<ImportAliases> {
        match tree {
            Inner {
                kind: Sk::ImportAliases,
                info,
                children,
            } => {
                let aliases: Option<Vec<Name>> =
                    skip_concrete(children).map(<Option<Name>>::from).collect();
                aliases.map(|aliases| ImportAliases { aliases, info })
            }
            _ => None,
        }
    }
}

impl From<UntypedTree> for Option<Name> {
    fn from(tree: UntypedTree) -> Option<Name> {
        if let Inner {
            kind,
            info,
            mut children,
        } = tree
        {
            match kind {
                Sk::Name | Sk::BadName => match children.pop() {
                    Some(Leaf(Token { text, .. })) => Some(Name {
                        text,
                        info,
                        ok: kind == Sk::Name,
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
                info,
                mut children,
            } => match children.pop() {
                Some(Leaf(Token { text, .. })) => Some(Filepath { text, info }),
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
                info,
                children,
            } => {
                let mut children: Vec<UntypedTree> = skip_concrete(children).collect();

                match children.len() {
                    0 => None,
                    1 => children.pop().and_then(UntypedTree::to_term),
                    _ => {
                        let rator = children.remove(0).to_term().map(Box::new);
                        let rands = children
                            .into_iter()
                            .map(UntypedTree::to_term)
                            .collect::<Option<Vec<Term>>>()
                            .unwrap_or(Vec::new());
                        Some(Term::App { rator, rands, info })
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
                info,
                mut children,
            } => match kind {
                Sk::Var => match children.pop() {
                    Some(Leaf(Token { text, .. })) => Some(Term::Var { text, info }),
                    _ => None,
                },
                Sk::Alias => match children.pop() {
                    Some(Leaf(Token { text, .. })) => Some(Term::Alias { text, info }),
                    _ => None,
                },
                Sk::Abs => {
                    let mut children: Vec<UntypedTree> = skip_concrete(children).collect();

                    // Note the ordering here
                    let body = children.pop();
                    let vars = children.pop();

                    let body = body.and_then(<Option<Term>>::from).map(Box::new);
                    let vars = vars.map(<Vec<Name>>::from).unwrap_or(Vec::new());

                    Some(Term::Abs { vars, body, info })
                }
                Sk::Tms => {
                    let terms = Inner {
                        kind,
                        info,
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
                info,
                children,
            } => {
                let names: Option<Vec<Name>> =
                    skip_concrete(children).map(<Option<Name>>::from).collect();
                names.unwrap_or(Vec::new())
            }
            _ => Vec::new(),
        }
    }
}

fn skip_concrete(children: Vec<UntypedTree>) -> impl Iterator<Item = UntypedTree> {
    children.into_iter().filter(|child| !child.is_leaf())
}
