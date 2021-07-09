mod lexer;
mod parser;

use self::parser::tree_builder::{ParseResult, TreeBuilder};
use crate::errors::SimpleError;
use crate::source::Source;
use std::rc::Rc;

pub use parser::ast::{Def, Filepath, Import, ImportAliases, Module, Name, ReplInput, Term};

pub fn parse_repl_input(source: Rc<Source>) -> Parsed<Option<ReplInput>> {
    let mut builder = TreeBuilder::new(&source);
    builder.parse_repl_input();
    let ParseResult { tree, errors } = builder.take();
    let result = <Option<ReplInput>>::from(tree);

    Parsed { result, errors }
}

pub fn parse_module(source: Rc<Source>) -> Parsed<Module> {
    let mut builder = TreeBuilder::new(&source);
    builder.parse_module();
    let ParseResult { tree, errors } = builder.take();
    let result = Module::from(tree);

    Parsed { result, errors }
}

#[derive(Debug)]
pub struct Parsed<T> {
    pub result: T,
    pub errors: Vec<SimpleError>,
}
