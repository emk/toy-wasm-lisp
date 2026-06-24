//! Imports of external definitions appearing in a WASM program.

use std::sync::Arc;

use miette::{NamedSource, Result};
use tree_sitter_wasl_types::nodes;
use type_sitter::Node as _;

use crate::{
    ast::{NodeResultExt, funcs::FuncSig},
    envs::ModuleEnv,
    locs::Loc,
};

use super::Ident;

/// A function imported into a WASM program.
#[derive(Clone, Debug)]
pub struct Import {
    #[expect(dead_code)]
    loc: Loc,
    mod_name: Ident,
    sig: FuncSig,
}

impl Import {
    pub fn from_grammar(
        src: Arc<NamedSource<String>>,
        mod_name: Ident,
        import: nodes::ImportFunc<'_>,
    ) -> Self {
        let loc = Loc::new(src.clone(), import.raw());
        let sig = FuncSig::from_grammar(src.clone(), import.func_sig().expect_matching());
        Self { loc, mod_name, sig }
    }

    pub fn sig(&self) -> &FuncSig {
        &self.sig
    }

    pub fn emit_decl(&self, mod_env: &mut ModuleEnv) -> Result<()> {
        mod_env.insert_import(self.mod_name.clone(), self.sig.name().clone(), self.clone())
    }
}
