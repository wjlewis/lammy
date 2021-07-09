mod errors;
mod source;
mod syntax;
mod terms;

use crate::source::Source;
use crate::syntax::{parse_module, parse_repl_input, Parsed, ReplInput};
use std::rc::Rc;

fn main() {
    let input = r#"
import { Id, K } from "./test";
import { Another } from "./another";

Quux = (x, y) => y;
"#;

    let result = parse_module(Rc::new(Source::new(
        String::from("my-file"),
        String::from(input),
    )));

    println!("{:#?}", result);
}
