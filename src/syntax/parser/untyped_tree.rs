use crate::source::SourceInfo;
use crate::syntax::lexer::Token;
use std::fmt;

pub enum UntypedTree {
    Inner {
        kind: SyntaxKind,
        info: SourceInfo,
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
    pub fn is_leaf(&self) -> bool {
        match self {
            Self::Leaf(..) => true,
            _ => false,
        }
    }

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
                info,
                children,
            } => {
                writeln!(f, "{:?}@{:?}", kind, info)?;
                for child in children {
                    child.fmt_debug(f, level + 1)?;
                }
                Ok(())
            }
            UntypedTree::Leaf(Token { kind, text, info }) => {
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
    Dummy,
}
