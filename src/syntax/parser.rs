mod ast;
mod tree_builder;
mod untyped_tree;

use self::tree_builder::{ParseResult, TreeBuilder};
use crate::errors::SimpleError;
use crate::source::Source;

pub use self::ast::{AbsVars, Def, Filepath, Module, Name, ReplInput, Term, UseAliases, UseDecl};

pub fn parse_repl_input(input: String) -> Parsed<Option<ReplInput>> {
    let mut builder = TreeBuilder::from(input.as_str());
    builder.parse_repl_input();
    let ParseResult { tree, errors } = builder.take();
    let result = <Option<ReplInput>>::from(tree);

    Parsed {
        result,
        source: Source::from_repl(input),
        errors,
    }
}

pub fn parse_module(input: String, filename: String) -> Parsed<Module> {
    let mut builder = TreeBuilder::from(input.as_str());
    builder.parse_module();
    let ParseResult { tree, errors } = builder.take();
    let result = Module::from(tree);

    Parsed {
        result,
        source: Source::from_file(filename, input),
        errors,
    }
}

#[derive(Debug)]
pub struct Parsed<T> {
    pub result: T,
    pub source: Source,
    pub errors: Vec<SimpleError>,
}
