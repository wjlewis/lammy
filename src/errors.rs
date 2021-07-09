use crate::source::{Source, SourceInfo};
use std::fmt;

pub trait Error: fmt::Debug {
    fn report(&self, src: &Source, f: &mut fmt::Formatter) -> fmt::Result;
}

#[derive(Debug)]
pub struct WithErrors<T> {
    pub result: T,
    pub errors: Vec<Box<dyn Error>>,
}

impl<T> WithErrors<T> {
    pub fn new(result: T, errors: Vec<Box<dyn Error>>) -> Self {
        Self { result, errors }
    }
}

#[derive(Debug)]
pub struct SimpleError {
    message: String,
    info: SourceInfo,
}

impl SimpleError {
    pub fn new(message: impl Into<String>, info: SourceInfo) -> Self {
        SimpleError {
            message: message.into(),
            info,
        }
    }
}

impl Error for SimpleError {
    fn report(&self, src: &Source, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "error: {}", self.message)
    }
}
