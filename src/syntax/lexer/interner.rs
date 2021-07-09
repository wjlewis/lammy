use std::collections::HashMap;
use std::rc::Rc;

#[derive(Default)]
pub struct Interner<'a> {
    seen: HashMap<&'a str, Rc<String>>,
}

impl<'a> Interner<'a> {
    pub fn intern(&mut self, text: &'a str) -> Rc<String> {
        self.seen.get(text).map(Rc::clone).unwrap_or_else(|| {
            let new = Rc::new(String::from(text));
            self.seen.insert(text, Rc::clone(&new));
            new
        })
    }
}
