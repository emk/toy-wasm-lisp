//! Imports of external definitions appearing in a WASM program.

use miette::Result;
use tree_sitter_wasl_types::nodes;
use type_sitter::Node as _;

use crate::{
    ast::{FromGrammar, NodeResultExt, funcs::FuncSig},
    envs::ModuleEnv,
    errors::ParseError,
    locs::{Loc, Source},
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
    pub fn sig(&self) -> &FuncSig {
        &self.sig
    }

    pub fn emit_decl(&self, mod_env: &mut ModuleEnv) -> Result<()> {
        mod_env.insert_import(self.mod_name.clone(), self.sig.name().clone(), self.clone())
    }
}

impl FromGrammar for Import {
    // Cheat slightly on our input type so we can pass in the mod_name.
    type Input<'a> = (Ident, nodes::ImportFunc<'a>);

    fn from_grammar(
        src: &Source,
        (mod_name, import): (Ident, nodes::ImportFunc<'_>),
    ) -> Result<Self, ParseError> {
        let loc = src.loc_for(import.raw());
        let sig = FuncSig::from_grammar(src, import.func_sig().expect_matching())?;
        Ok(Self { loc, mod_name, sig })
    }
}
