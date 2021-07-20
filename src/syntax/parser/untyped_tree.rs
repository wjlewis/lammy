use crate::source::Span;
use crate::syntax::tokens::Token;
use std::fmt;

/// A homogeneous (e.g. untyped) tree.
pub enum UntypedTree {
    Inner {
        kind: SyntaxKind,
        span: Span,
        children: Vec<UntypedTree>,
    },
    Leaf(Token),
}

impl fmt::Debug for UntypedTree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.fmt_debug(f, 0)
    }
}

impl UntypedTree {
    /// Tests if this tree is a `Leaf` node.
    pub fn is_leaf(&self) -> bool {
        match self {
            Self::Leaf(..) => true,
            _ => false,
        }
    }

    /// Tests if this tree is an `Inner` node with the provided `SyntaxKind`.
    pub fn has_kind(&self, kind: &SyntaxKind) -> bool {
        match self {
            Self::Inner { kind: my_kind, .. } if my_kind == kind => true,
            _ => false,
        }
    }

    fn fmt_debug(&self, f: &mut fmt::Formatter, level: usize) -> fmt::Result {
        Self::indent(f, level)?;

        match self {
            UntypedTree::Inner {
                kind,
                span,
                children,
            } => {
                writeln!(f, "{:?}@{:?}", kind, span)?;
                for child in children {
                    child.fmt_debug(f, level + 1)?;
                }
                Ok(())
            }
            UntypedTree::Leaf(Token {
                kind,
                text,
                span: info,
            }) => {
                writeln!(f, r#"{:?}("{}")@{:?}"#, kind, text, info)
            }
        }
    }

    #[inline]
    fn indent(f: &mut fmt::Formatter, level: usize) -> fmt::Result {
        for _ in 0..level {
            write!(f, "  ")?;
        }
        Ok(())
    }
}

/// The possible types that a tree (specifically, an `Inner` node) might have.
/// These are intended to demarcate the important parts of syntax that will
/// later be extracted into a struct.
#[derive(Debug, PartialEq)]
pub enum SyntaxKind {
    ReplInput,
    Module,
    Def,
    Import,
    ImportAliases,
    ImportFilepath,
    Tms,
    Var,
    Alias,
    Abs,
    AbsVars,
    Name,
    BadName,
    Missing,
}
