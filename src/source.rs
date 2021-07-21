use std::fmt;

#[derive(Clone, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }

    pub fn combine_with(self, other: Self) -> Self {
        let start = usize::min(self.start, other.start);
        let end = usize::max(self.end, other.end);

        Span::new(start, end)
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
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
