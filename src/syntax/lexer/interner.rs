use std::collections::HashMap;
use std::rc::Rc;

/// A simple string interner. Given a `&str`, produces an `Rc<String>`. The
/// latter can thus outlive the interner (obviating borrowing issues).
#[derive(Default)]
pub struct Interner<'a> {
    seen: HashMap<&'a str, Rc<String>>,
}

impl<'a> Interner<'a> {
    /// Produces an `Rc<String>` whose content is equal (`==`) to that of `text`.
    /// Additionally, if `text` has already been interned it doesn't allocate a
    /// new `String`; instead, it simply returns a clone of the pointer to the
    /// previously allocated `String`.
    ///
    /// ```no_run
    /// let mut i = Interner::default();
    ///
    /// // Since this is the first time we've interned the slice `"apples"`, a
    /// // new `String` is allocated:
    /// let a1 = i.intern("apples");
    ///
    /// // Since we've already interned the slice `"apples"`, no allocation
    /// // occurs; only the `Rc`'s refcount is bumped:
    /// let a2 = i.intern("apples");
    /// ```
    pub fn intern(&mut self, text: &'a str) -> Rc<String> {
        self.seen.get(text).map(Rc::clone).unwrap_or_else(|| {
            let new = Rc::new(String::from(text));
            self.seen.insert(text, Rc::clone(&new));
            new
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interner_shares_duplicate_strings() {
        let mut i = Interner::default();

        let a1 = i.intern("apple");
        let _a2 = i.intern("apple");
        let b1 = i.intern("banana");
        let _a3 = i.intern("apple");
        let c1 = i.intern("cantaloupe");
        let _b2 = i.intern("banana");

        drop(i);

        assert_eq!(Rc::strong_count(&a1), 3);
        assert_eq!(Rc::strong_count(&b1), 2);
        assert_eq!(Rc::strong_count(&c1), 1);
    }
}
