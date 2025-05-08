use std::{collections::BTreeMap, fmt::Display, result};
use crate::runtime::{Ident, If, Value, ValueData};
use itermaps::short_funcs::default;
use jatom_parser::Arc;

#[derive(Debug, Clone)]
pub struct Error {
    error: ErrorInfo,
    location: usize,
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <ErrorInfo as Display>::fmt(&self.error, f)
    }
}
impl Error {
    pub fn location(&self) -> usize {
        self.location
    }
}

#[derive(Debug, Clone)]
pub enum ErrorInfo {
    UndefinedIdent(Ident),
}
impl Display for ErrorInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorInfo::UndefinedIdent(ident) => {
                write!(f, "undefined `{ident}` in scope")?
            },
        }
        Ok(())
    }
}

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct ScopeGuard<'a> {
    ctx: &'a mut AnalysisContext,
}

impl<'a> std::ops::DerefMut for ScopeGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ctx
    }
}

impl<'a> std::ops::Deref for ScopeGuard<'a> {
    type Target = &'a mut AnalysisContext;

    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}
impl<'a> ScopeGuard<'a> {
    fn new(ctx: &'a mut AnalysisContext) -> Self {
        ctx.scopes.push(default());
        Self { ctx }
    }
}
impl<'a> Drop for ScopeGuard<'a> {
    fn drop(&mut self) {
        self.ctx.scopes.pop().unwrap();
    }
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct AnalysisContext {
    scopes: Vec<BTreeMap<Ident, Arc<Value>>>,
}
impl AnalysisContext {
    pub fn new() -> Self {
        Self { scopes: vec![default()] }
    }

    fn scoper(&mut self) -> ScopeGuard<'_> {
        ScopeGuard::new(self)
    }

    pub fn analysis(&mut self, ast: &mut Value) -> Result<()> {
        let err = |error| {
            Err(Error { error, location: ast.location })
        };

        match &mut ast.data {
            ValueData::Number(_) => (),
            ValueData::String(_) => (),
            ValueData::Pipe(values) => {
                let mut this = self.scoper();
                for ast in Arc::make_mut(values) {
                    this.analysis(ast)?
                }
            },
            ValueData::Op1(_, value) => {
                self.scoper().analysis(Arc::make_mut(value))?
            },
            ValueData::And(value, value1)
            | ValueData::Or(value, value1)
            | ValueData::Op2(_, value, value1) => {
                self.scoper().analysis(Arc::make_mut(value))?;
                self.scoper().analysis(Arc::make_mut(value1))?;
            },
            ValueData::Call(fun) => {
                self.scoper().analysis(Arc::make_mut(fun))?;
            },
            ValueData::If(If { cond, yes, no }) => {
                self.scoper().analysis(Arc::make_mut(cond))?;
                self.scoper().analysis(Arc::make_mut(yes))?;
                if let Some(no) = no {
                    self.scoper().analysis(Arc::make_mut(no))?;
                }
            },
            ValueData::List(list) => {
                let mut this = self.scoper();
                for ast in Arc::make_mut(list) {
                    this.analysis(ast)?
                }
            },
            ValueData::Ident(ident) => {
                if let Some(value) = self.scopes
                    .iter_mut()
                    .rev()
                    .find_map(|map: _| map.get_mut(ident))
                {
                    ident.value = value.clone().into();
                } else {
                    return err(ErrorInfo::UndefinedIdent(ident.clone()));
                }
            },
            ValueData::Assign(ident, value) => {
                self.scopes.last_mut().unwrap()
                    .insert(ident.clone(), value.clone());
            },
            ValueData::This | ValueData::Null => (),
        }

        Ok(())
    }
}
