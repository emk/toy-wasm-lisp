//! Implementations for [`Module`].

use std::sync::Arc;

use miette::{NamedSource, Result};

use super::Func;
use crate::{envs::ModuleEnv, parser::grammar};

#[derive(Clone, Debug)]
pub struct Mod {
    funcs: Vec<Func>,
}

impl Mod {
    pub fn from_grammar(src: Arc<NamedSource<String>>, grammar: &grammar::Mod) -> Self {
        Self {
            funcs: grammar
                .funcs
                .iter()
                .map(|f| Func::from_grammar(src.clone(), f))
                .collect(),
        }
    }

    pub fn emit(&self) -> Result<Vec<u8>> {
        // Create our module-level environment, which contains
        // all the state needed to emit code.
        let mut module_env = ModuleEnv::new();

        // Eventually we will need to emit all function decls first,
        // then all impls.
        for f in &self.funcs {
            f.emit_decl(&mut module_env)?;
        }
        for f in &self.funcs {
            f.emit_impl(&mut module_env)?;
        }
        let module = module_env.build_module();
        Ok(module.finish())
    }
}
