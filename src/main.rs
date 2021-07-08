mod errors;
mod lexer;
mod parser;
mod source;

use crate::parser::{Module, ParseResult, TreeBuilder};

fn main() {
    let input = r#"
# Natural numbers
use { Cons, Fst, Snd } from "./pairs";
use { K, Y } from "./general-purpose";
use { If, True, False } from "./booleans";

Zero = (s, z) => z;
Suc = n => (s, z) => s (n s z);
Zero? = n => n (K False) True;

Pred = n => Fst (n (prev => Cons (Snd prev) (Suc (Snd prev)))
                   (Cons Zero Zero));

Sum = (m, n) => m Suc n;
Prod = (m, n) => m (Sum n) Zero;
Pow = (b, e) => e (Prod b) (Suc Zero);

Fact = Y fact => n => If (Zero? n)
                         (Suc Zero)
                         (Prod n (fact (Pred n)));
"#;
    let mut builder = TreeBuilder::from(input);

    builder.parse_module();
    let ParseResult { tree, errors } = builder.take();

    println!("{:#?}", Module::from(tree));
    println!("{:#?}", errors);
}
