use crate::source::{SourceFile, Span};
use std::fmt;

pub trait Error: fmt::Debug {
    fn report(&self, src: &SourceFile, f: &mut fmt::Formatter) -> fmt::Result;
}

#[derive(Debug)]
pub struct SimpleError {
    message: String,
    span: Span,
}

impl SimpleError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        SimpleError {
            message: message.into(),
            span,
        }
    }
}

impl Error for SimpleError {
    fn report(&self, src: &SourceFile, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "error: {}", self.message)
    }
}
