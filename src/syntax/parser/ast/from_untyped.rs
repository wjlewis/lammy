use super::super::untyped_tree::{SyntaxKind as Sk, UntypedTree};
use super::{AbsVars, Def, Filepath, Module, Name, ReplInput, Term, UseAliases, UseDecl};
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
                span,
                children,
            } => {
                let (use_decls, defs): (Vec<UntypedTree>, Vec<UntypedTree>) =
                    skip_concrete(children).partition(|tree| tree.is_use_decl());

                let use_decls = use_decls
                    .into_iter()
                    .map(<Option<UseDecl>>::from)
                    .collect::<Option<Vec<UseDecl>>>();

                let defs = defs
                    .into_iter()
                    .map(<Option<Def>>::from)
                    .collect::<Option<Vec<Def>>>();

                Module {
                    use_decls: use_decls.unwrap_or(Vec::new()),
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

impl From<UntypedTree> for Option<UseDecl> {
    fn from(tree: UntypedTree) -> Option<UseDecl> {
        match tree {
            Inner {
                kind: Sk::Use,
                span,
                children,
            } => {
                let mut children: Vec<UntypedTree> = skip_concrete(children).collect();

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

impl From<UntypedTree> for Option<UseAliases> {
    fn from(tree: UntypedTree) -> Option<UseAliases> {
        match tree {
            Inner {
                kind: Sk::UseAliases,
                span,
                children,
            } => {
                let aliases: Option<Vec<Name>> =
                    skip_concrete(children).map(<Option<Name>>::from).collect();
                aliases.map(|aliases| UseAliases { aliases, span })
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
                Sk::UseAlias | Sk::BadUseAlias => match children.pop() {
                    Some(Leaf(Token { text, .. })) => Some(Name {
                        text,
                        span,
                        ok: kind == Sk::UseAlias,
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
                kind: Sk::UseFilepath,
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
                        let rator = children.remove(0).to_term().map(Box::new);
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
                Sk::Name => match children.pop() {
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
                    let names = children.pop();

                    let body = body.and_then(<Option<Term>>::from).map(Box::new);
                    let names = names.and_then(<Option<AbsVars>>::from);

                    Some(Term::Abs { names, body, span })
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

    fn is_use_decl(&self) -> bool {
        match self {
            Inner { kind: Sk::Use, .. } => true,
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

impl From<UntypedTree> for Option<AbsVars> {
    fn from(tree: UntypedTree) -> Option<AbsVars> {
        match tree {
            Inner {
                kind: Sk::AbsVars,
                span,
                children,
            } => {
                let names: Option<Vec<Name>> =
                    skip_concrete(children).map(<Option<Name>>::from).collect();
                names.map(|names| AbsVars { names, span })
            }
            _ => None,
        }
    }
}

fn skip_concrete(children: Vec<UntypedTree>) -> impl Iterator<Item = UntypedTree> {
    children.into_iter().filter(|child| !child.is_leaf())
}
