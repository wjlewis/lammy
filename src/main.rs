mod errors;
mod source;
mod syntax;

use syntax::parse_module;

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
"#;

    println!(
        "{:#?}",
        parse_module(String::from(input), String::from("./my-first-file"))
    );
}
