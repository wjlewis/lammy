use std::fmt;
use std::rc::Rc;

#[derive(Clone)]
pub struct SourceInfo {
    pub source: Rc<Source>,
    pub span: Span,
}

impl SourceInfo {
    pub fn new(source: Rc<Source>, start: usize, end: usize) -> Self {
        Self {
            source,
            span: Span::new(start, end),
        }
    }

    pub fn combine_with(self, info: SourceInfo) -> Self {
        let start = usize::min(self.span.start, info.span.start);
        let end = usize::max(self.span.end, info.span.end);

        SourceInfo {
            source: self.source,
            span: Span::new(start, end),
        }
    }
}

impl fmt::Debug for SourceInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, r#"{:?} in "{}""#, self.span, self.source.filename)
    }
}

#[derive(Debug)]
pub struct Source {
    pub filename: String,
    pub text: String,
}

impl Source {
    pub fn new(filename: String, text: String) -> Self {
        Source { filename, text }
    }
}

#[derive(Clone, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}
