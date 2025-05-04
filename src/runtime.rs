use std::{borrow::Borrow, collections::BTreeMap, hash::Hash};

use itermaps::MapExt;
use ordered_float::OrderedFloat;
use smol_str::SmolStr;
use jatom_parser::{
    self as p,
    syntax::{BinaryOp, SingleOp}, Arc, Expr, ExprValue
};


#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Value {
    pub data: ValueData,
    pub location: usize,
}
impl From<&Expr> for Value {
    fn from(value: &Expr) -> Self {
        Self {
            data: value.value.as_ref().into(),
            location: value.location.0,
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Runtime {
    scopes: Vec<BTreeMap<Arc<str>, Arc<Value>>>,
}
impl Default for Runtime {
    fn default() -> Self {
        Self {
            scopes: vec![Default::default()],
        }
    }
}

#[derive(Debug, Eq, Clone)]
pub struct Ident {
    name: Arc<str>,
    id: usize,
    pub(crate)
    value: Option<Arc<Value>>,
}

impl Borrow<usize> for Ident {
    fn borrow(&self) -> &usize {
        &self.id
    }
}
impl Hash for Ident {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
impl Ord for Ident {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}
impl PartialOrd for Ident {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.cmp(other).into()
    }
}
impl PartialEq for Ident {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Ident {
    pub fn name(&self) -> &str {
        &self.name
    }
}
impl From<&p::Ident> for Ident {
    fn from(value: &p::Ident) -> Self {
        Self {
            name: value.name.clone(),
            id: value.id,
            value: None,
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct If {
    pub cond: Arc<Value>,
    pub yes: Arc<Value>,
    pub no: Option<Arc<Value>>,
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum ValueData {
    Number(OrderedFloat<f64>),
    String(SmolStr),
    Pipe(Arc<[Value]>),
    Op1(SingleOp, Arc<Value>),
    Op2(BinaryOp, Arc<Value>, Arc<Value>),
    And(Arc<Value>, Arc<Value>),
    Or(Arc<Value>, Arc<Value>),
    Assign(Ident, Arc<Value>),
    Call(Arc<Value>, Arc<[Value]>),
    If(If),
    Ident(Ident),
}
impl From<Arc<ExprValue>> for ValueData {
    fn from(value: Arc<ExprValue>) -> Self {
        value.as_ref().into()
    }
}
impl From<&Arc<ExprValue>> for ValueData {
    fn from(value: &Arc<ExprValue>) -> Self {
        value.as_ref().into()
    }
}
impl From<&ExprValue> for ValueData {
    fn from(value: &ExprValue) -> Self {
        fn arc(expr: &Expr) -> Arc<Value> {
            Arc::new(expr.into())
        }
        match value {
            ExprValue::Pipe(vec) => {
                Self::Pipe(vec.iter()
                    .map_into()
                    .collect())
            },
            ExprValue::Op1(single_op, expr) => {
                Self::Op1(*single_op, arc(expr))
            },
            ExprValue::Op2(binary_op, expr, expr1) => {
                Self::Op2(*binary_op, arc(expr), arc(expr1))
            },
            ExprValue::And(expr, expr1) => {
                Self::And(arc(expr), arc(expr1))
            },
            ExprValue::Or(expr, expr1) => {
                Self::Or(arc(expr), arc(expr1))
            },
            ExprValue::If(p::If { cond, yes, no }) => {
                Self::If(If {
                    cond: arc(cond),
                    yes: arc(yes),
                    no: no.as_ref().map(arc),
                })
            },
            ExprValue::Assign(name, value) => {
                Self::Assign(name.into(), arc(value.into()))
            },
            ExprValue::Call(expr, args) => {
                Self::Call(arc(expr), args.iter().map_into().collect())
            },
            ExprValue::Literal(p::Literal::String(s)) => {
                Self::String(s.clone().into())
            },
            ExprValue::Literal(p::Literal::Number(num)) => {
                Self::Number(*num)
            },
            ExprValue::Ident(i) => Self::Ident(i.into()),
        }
    }
}
