use crate::errors::{Error, SimpleError, WithErrors};
use crate::source::SourceInfo;
use crate::syntax::{Name, Term as SurfaceTerm};
use std::rc::Rc;

#[derive(Debug)]
pub enum DesugaredTerm {
    Var {
        text: Rc<String>,
        info: SourceInfo,
    },
    Alias {
        text: Rc<String>,
        info: SourceInfo,
    },
    Abs {
        var: Option<Name>,
        body: Option<Box<DesugaredTerm>>,
        info: SourceInfo,
    },
    App {
        rator: Option<Box<DesugaredTerm>>,
        rand: Option<Box<DesugaredTerm>>,
        info: SourceInfo,
    },
}

impl SurfaceTerm {
    pub fn desugar(self) -> DesugaredTerm {
        use DesugaredTerm as DTerm;
        use SurfaceTerm as STerm;

        match self {
            STerm::Var { text, info } => DTerm::Var { text, info },
            STerm::Alias { text, info } => DTerm::Alias { text, info },
            STerm::Abs {
                mut vars,
                body,
                info,
            } => {
                let body = body.map(|body| body.desugar()).map(Box::new);
                let last_var = vars.pop();

                let init = DTerm::Abs {
                    var: last_var,
                    body,
                    info: info.clone(),
                };

                vars.into_iter().fold(init, |body, var| DTerm::Abs {
                    var: Some(var),
                    body: Some(Box::new(body)),
                    info: info.clone(),
                })
            }
            STerm::App {
                rator,
                mut rands,
                info,
            } => {
                let rator = rator.map(|rator| rator.desugar()).map(Box::new);

                rands.reverse();
                let first_rand = rands.pop().map(|rand| rand.desugar()).map(Box::new);

                let init = DTerm::App {
                    rator,
                    rand: first_rand,
                    info: info.clone(),
                };

                rands.into_iter().fold(init, |rator, rand| DTerm::App {
                    rator: Some(Box::new(rator)),
                    rand: Some(Box::new(rand.desugar())),
                    info: info.clone(),
                })
            }
        }
    }
}

impl DesugaredTerm {
    pub fn resugar(self) -> SurfaceTerm {
        todo!()
    }

    pub fn index(self) -> WithErrors<IndexedTerm> {
        let mut errors = Vec::new();
        let mut bound_vars = Vec::new();
        let result = self.index_using(&mut errors, &mut bound_vars);
        WithErrors::new(result, errors)
    }

    fn index_using(
        self,
        errors: &mut Vec<Box<dyn Error>>,
        bound_vars: &mut Vec<Name>,
    ) -> IndexedTerm {
        use DesugaredTerm as DTerm;
        use IndexedTerm as ITerm;

        match self {
            DTerm::Var { text, info } => {
                let index = bound_vars.iter().rev().position(|v| v.text == text);
                if index.is_none() {
                    errors.push(Box::new(SimpleError::new("unbound variable", info.clone())));
                }
                ITerm::Index { index, info }
            }
            DTerm::Alias { text, info } => ITerm::Alias { text, info },
            DTerm::Abs { var, body, info } => match var {
                Some(var) if var.ok => {
                    bound_vars.push(var);
                    let body = body
                        .map(|body| body.index_using(errors, bound_vars))
                        .map(Box::new);
                    let var = bound_vars.pop();

                    ITerm::Abs { var, body, info }
                }
                _ => {
                    if var.is_none() {
                        errors.push(Box::new(SimpleError::new(
                            "abstraction needs at least one var",
                            info.clone(),
                        )));
                    }
                    let body = body
                        .map(|body| body.index_using(errors, bound_vars))
                        .map(Box::new);
                    ITerm::Abs { var, body, info }
                }
            },
            DTerm::App { rator, rand, info } => {
                let rator = rator
                    .map(|rator| rator.index_using(errors, bound_vars))
                    .map(Box::new);
                let rand = rand
                    .map(|rand| rand.index_using(errors, bound_vars))
                    .map(Box::new);

                ITerm::App { rator, rand, info }
            }
        }
    }
}

#[derive(Debug)]
pub enum IndexedTerm {
    Index {
        index: Option<usize>,
        info: SourceInfo,
    },
    Alias {
        text: Rc<String>,
        info: SourceInfo,
    },
    Abs {
        var: Option<Name>,
        body: Option<Box<IndexedTerm>>,
        info: SourceInfo,
    },
    App {
        rator: Option<Box<IndexedTerm>>,
        rand: Option<Box<IndexedTerm>>,
        info: SourceInfo,
    },
}

impl IndexedTerm {
    pub fn unindex(self) -> DesugaredTerm {
        todo!()
    }
}
