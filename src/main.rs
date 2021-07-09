mod errors;
mod source;
mod syntax;

use crate::source::Source;
use crate::syntax::parse_module;
use std::rc::Rc;

fn main() {
    let input = r#"
use { I, K } from "./general-purpose";

# Here's a demonstration of the definition mechanism:
Flip2 = combinator => (x, y) => combinator y x;

# We can use `Flip2` to define a flipped version of `K`:
K' = Flip2 K;

# Some natural numbers:
Zero = (s, z) => z;
Suc = n => (s, z) => s (n s z);

# Alternatively, we can do away with the syntactic sugar:
Zero' = s => z => z;
Suc' = n => s => z => s ((n s) z);

Y = f => (x => f (x x))
          x => f (x x);
"#;

    println!(
        "{:#?}",
        parse_module(Rc::new(Source::new(
            String::from("my-file"),
            String::from(input)
        )))
    );
}
