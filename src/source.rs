use std::fmt;

#[derive(Debug)]
pub struct Source {
    filename: Option<String>,
    text: String,
}

impl Source {
    pub fn from_file(filename: String, text: String) -> Self {
        Source {
            filename: Some(filename),
            text,
        }
    }

    pub fn from_repl(text: String) -> Self {
        Source {
            filename: None,
            text,
        }
    }
}

#[derive(Clone, Copy)]
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
