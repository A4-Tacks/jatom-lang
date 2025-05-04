pub mod syntax;
pub mod parser;

use std::collections::BTreeSet;
pub use std::sync::Arc;
pub use syntax::*;

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Default)]
pub struct ParseState {
    ident_id: usize,
    pool: BTreeSet<Arc<str>>,
}

impl ParseState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn str_pool(&mut self, s: &str) -> Arc<str> {
        if !self.pool.contains(s) {
            self.pool.insert(s.into());
        }
        self.pool.get(s).unwrap().clone()
    }

    pub fn ident(&mut self, name: &str) -> Ident {
        let name = self.str_pool(name);
        let ident = Ident { name, id: self.ident_id };
        self.ident_id += 1;
        ident
    }
}
