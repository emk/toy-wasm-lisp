//! Environment inside functions.

use miette::Result;
use tracing::trace;

use super::{DeclTable, Symbol, SymbolTable, VarSymbol};
use crate::{
    ast::{FuncSig, Ident, LocalSymbol},
    envs::DeclIdx,
    errors::SymbolTableError,
};

pub struct FuncEnv {
    // Table of local variable declarations. This is only one
    // of these per function.
    decls: DeclTable<LocalSymbol>,

    // The number of params this function has.
    param_count: Option<usize>,
}

impl FuncEnv {
    pub fn new() -> Self {
        Self {
            decls: DeclTable::new(),
            param_count: None,
        }
    }

    // Declare our parameters.
    //
    // This really only lives here so that it can set `param_count`.
    pub fn declare_params(&mut self, sig: &FuncSig, syms: &mut SymbolTable) -> Result<()> {
        assert!(self.param_count.is_none());
        let params = sig.params();
        trace!(f = %sig.name(), ?params, "Declaring parameters");
        params.declare(self, syms)?;
        self.param_count = Some(params.len());
        Ok(())
    }

    // Our decls, excluding params.
    pub fn locals(&self) -> impl Iterator<Item = (DeclIdx<LocalSymbol>, &LocalSymbol)> {
        assert!(self.param_count.is_some());
        self.decls.items().skip(
            self.param_count
                .expect("parameters should have already been declared"),
        )
    }

    pub fn insert_local(
        &mut self,
        name: Ident,
        local: LocalSymbol,
        symbol_table: &mut SymbolTable,
    ) -> Result<DeclIdx<LocalSymbol>, SymbolTableError> {
        let idx = self.decls.insert(local.clone());
        let sym = Symbol::Var(VarSymbol::Local {
            idx,
            local: Box::new(local),
        });
        symbol_table.insert(name, sym)?;
        Ok(idx)
    }
}
