use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub struct Name(Rc<String>);

impl Name {
    pub fn new(name: impl Into<String>) -> Self {
        Name(Rc::new(name.into()))
    }
}

impl Name {
    pub fn freshen_in(&self, used: &List<Name>) -> Name {
        if !used.includes(self) {
            self.clone()
        } else {
            let mut ticks = String::new();
            let mut candidate;
            loop {
                ticks.push('\'');
                candidate = format!("{}{}", self.0, ticks);

                if !used.includes(&candidate) {
                    return Name(Rc::new(candidate));
                }
            }
        }
    }
}

impl AsRef<Name> for Name {
    fn as_ref(&self) -> &Name {
        self
    }
}

impl AsRef<String> for Name {
    fn as_ref(&self) -> &String {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct Term(Rc<_Term>);

pub enum _Term {
    Index { index: usize },
    Abs { name: Name, body: Term },
    App { rator: Term, rand: Term },
}

#[derive(Debug, Clone)]
pub struct Value(Rc<_Value>);

enum _Value {
    Closure { name: Name, body: Term, env: Env },
    Stuck(Stuck),
    Thunk(Thunk),
}

#[derive(Debug, Clone)]
pub struct Stuck(Rc<_Stuck>);

enum _Stuck {
    Index { binder_count: usize },
    App { op: Stuck, arg: Value },
}

#[derive(Debug, Clone)]
pub struct Thunk(Rc<RefCell<ThunkContent>>);

#[derive(Clone)]
enum ThunkContent {
    Frozen { term: Term, env: Env },
    Thawed(Value),
}

impl Thunk {
    pub fn thaw(&self) -> Value {
        let mut content = self.0.borrow_mut();
        match &*content {
            ThunkContent::Frozen { term, env } => {
                let value = term.eval(env);
                *content = ThunkContent::Thawed(value.clone());
                value
            }
            ThunkContent::Thawed(value) => value.clone(),
        }
    }

    pub fn new(term: Term, env: Env) -> Self {
        Thunk(Rc::new(RefCell::new(ThunkContent::Frozen { term, env })))
    }
}

pub type Env = List<Value>;

impl Term {
    pub fn norm(&self) -> Term {
        let val = self.eval(&Env::new());
        val.quote()
    }

    pub fn eval(&self, env: &Env) -> Value {
        match &*self.0 {
            _Term::Index { index } => env.get(*index).map(Clone::clone).unwrap(),
            _Term::Abs { name, body } => Value::closure(name.clone(), body.clone(), env.clone()),
            _Term::App { rator, rand } => {
                let op = rator.eval(env);
                let rand = rand.eval_or_freeze(env);
                op.apply(rand)
            }
        }
    }

    fn eval_or_freeze(&self, env: &Env) -> Value {
        match &*self.0 {
            _Term::App { .. } => Value::thunk(self.clone(), env.clone()),
            _ => self.eval(env),
        }
    }

    pub fn index(index: usize) -> Self {
        Term(Rc::new(_Term::Index { index }))
    }

    pub fn abs(name: Name, body: Term) -> Self {
        Term(Rc::new(_Term::Abs { name, body }))
    }

    pub fn app(rator: Term, rand: Term) -> Self {
        Term(Rc::new(_Term::App { rator, rand }))
    }
}

impl Value {
    pub fn apply(&self, arg: Value) -> Value {
        match &*self.0 {
            _Value::Closure { body, env, .. } => {
                let env = env.push(arg);
                body.eval(&env)
            }
            _Value::Stuck(op) => Value::stuck(Stuck::app(op.clone(), arg)),
            _Value::Thunk(thunk) => {
                let op = thunk.thaw();
                op.apply(arg)
            }
        }
    }

    pub fn quote(&self) -> Term {
        self.quote_from(0, &List::new())
    }

    fn quote_from(&self, binder_count: usize, used_names: &List<Name>) -> Term {
        match &*self.0 {
            _Value::Closure { name, body, env } => {
                // Update binder count to account for new binder
                let new_binder_count = binder_count + 1;
                let proxy_arg = Value::stuck(Stuck::index(new_binder_count));
                let body_val = body.eval(&env.push(proxy_arg));
                let name = name.freshen_in(used_names);
                let used_names = used_names.push(name.clone());

                Term::abs(name, body_val.quote_from(new_binder_count, &used_names))
            }
            _Value::Stuck(stuck) => stuck.quote_from(binder_count, used_names),
            _Value::Thunk(thunk) => {
                let val = thunk.thaw();
                val.quote_from(binder_count, used_names)
            }
        }
    }

    pub fn closure(name: Name, body: Term, env: Env) -> Self {
        Value(Rc::new(_Value::Closure { name, body, env }))
    }

    pub fn stuck(stuck: Stuck) -> Self {
        Value(Rc::new(_Value::Stuck(stuck)))
    }

    pub fn thunk(term: Term, env: Env) -> Self {
        Value(Rc::new(_Value::Thunk(Thunk::new(term, env))))
    }
}

impl Stuck {
    pub fn quote_from(&self, binder_count: usize, used_names: &List<Name>) -> Term {
        match &*self.0 {
            _Stuck::Index {
                binder_count: creation_binder_count,
            } => {
                let index = binder_count - creation_binder_count;
                Term::index(index)
            }
            _Stuck::App { op, arg } => {
                let rator = op.quote_from(binder_count, used_names);
                let rand = arg.quote_from(binder_count, used_names);
                Term::app(rator, rand)
            }
        }
    }

    pub fn index(binder_count: usize) -> Self {
        Stuck(Rc::new(_Stuck::Index { binder_count }))
    }

    pub fn app(op: Stuck, arg: Value) -> Self {
        Stuck(Rc::new(_Stuck::App { op, arg }))
    }
}

impl fmt::Debug for _Term {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            _Term::Index { index } => write!(f, "{}", index),
            _Term::Abs { name, body } => write!(f, "{:?} => {:?}", name, body),
            _Term::App { rator, rand } => write!(f, "({:?} {:?})", rator, rand),
        }
    }
}

impl fmt::Debug for _Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            _Value::Closure { name, body, env } => {
                write!(f, "<{:?} : {:?} in {:?}>", name, body, env)
            }
            _Value::Stuck(stuck) => write!(f, "{:?}", stuck),
            _Value::Thunk(thunk) => write!(f, "{:?}", thunk),
        }
    }
}

impl fmt::Debug for _Stuck {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            _Stuck::Index { binder_count } => {
                write!(f, "{{{}}}", binder_count)
            }
            _Stuck::App { op, arg } => write!(f, "{{{:?} @ {:?}}}", op, arg),
        }
    }
}

impl fmt::Debug for ThunkContent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ThunkContent::Frozen { term, env } => write!(f, "<<{:?} in {:?}>>", term, env),
            ThunkContent::Thawed(value) => write!(f, "<<{:?}>>", value),
        }
    }
}

#[derive(Debug)]
pub struct List<T>(Rc<_List<T>>);

enum _List<T> {
    Empty,
    Cons(T, List<T>),
}

impl<T> List<T> {
    pub fn new() -> Self {
        List(Rc::new(_List::Empty))
    }

    pub fn push(&self, x: T) -> Self {
        List(Rc::new(_List::Cons(x, self.clone())))
    }

    pub fn get(&self, i: usize) -> Option<&T> {
        match &*self.0 {
            _List::Empty => None,
            _List::Cons(first, rest) => {
                if i == 0 {
                    Some(first)
                } else {
                    rest.get(i - 1)
                }
            }
        }
    }

    pub fn includes<U>(&self, x: &U) -> bool
    where
        U: PartialEq,
        T: AsRef<U>,
    {
        match &*self.0 {
            _List::Empty => false,
            _List::Cons(first, rest) => {
                if first.as_ref() == x {
                    true
                } else {
                    rest.includes(x)
                }
            }
        }
    }
}

impl<T> Clone for List<T> {
    fn clone(&self) -> Self {
        List(Rc::clone(&self.0))
    }
}

impl<T: fmt::Debug> fmt::Debug for _List<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            _List::Empty => write!(f, "()"),
            _List::Cons(first, rest) => {
                write!(f, "{:?}", first)?;
                rest.0.debug_rest(f)
            }
        }
    }
}

impl<T: fmt::Debug> _List<T> {
    fn debug_rest(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            _List::Empty => Ok(()),
            _List::Cons(first, rest) => {
                write!(f, ", {:?}", first)?;
                rest.0.debug_rest(f)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freshen() {
        let used = List::new()
            .push(Name::new("a"))
            .push(Name::new("a'"))
            .push(Name::new("b"));

        let name = Name::new("a");
        assert_eq!(name.freshen_in(&used), Name::new("a''"));
    }
}
