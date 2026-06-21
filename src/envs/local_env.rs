//! Environment inside functions.

use miette::Result;

use super::{DeclTable, Symbol, SymbolTable};
use crate::{
    ast::{Ident, Local},
    errors::SymbolTableError,
};

pub struct LocalEnv<'parent> {
    // Table of local variable declarations. This is only one
    // of these per function.
    decls: &'parent mut DeclTable<Local>,

    // Our symbol table (which links back up the chain to
    // our parent symbol table).
    symbol_table: SymbolTable<'parent>,
}

impl<'parent> LocalEnv<'parent> {
    pub fn new(decls: &'parent mut DeclTable<Local>, symbol_table: &'parent SymbolTable) -> Self {
        Self {
            decls,
            symbol_table: symbol_table.child(),
        }
    }

    #[expect(dead_code)]
    pub fn child<'new_parent: 'parent>(&'new_parent mut self) -> Self {
        Self {
            decls: self.decls,
            symbol_table: self.symbol_table.child(),
        }
    }

    pub fn insert_local(&mut self, name: Ident, local: Local) -> Result<&Symbol, SymbolTableError> {
        let idx = self.decls.insert(local.clone());
        let sym = Symbol::Local {
            idx,
            local: Box::new(local),
        };
        self.symbol_table.insert(name, sym)
    }

    pub fn symbol_table(&self) -> &SymbolTable<'_> {
        &self.symbol_table
    }
}
