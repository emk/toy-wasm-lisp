//! Implementations for [`Module`].

use std::sync::Arc;

use miette::{NamedSource, Result};
use tree_sitter_wasl_types::nodes;
use type_sitter::Node as _;

use super::Func;
use crate::{ast::NodeResultExt, envs::ModuleEnv};

#[derive(Clone, Debug)]
pub struct Mod {
    funcs: Vec<Func>,
}

impl Mod {
    pub fn from_grammar(src: Arc<NamedSource<String>>, source_file: nodes::SourceFile<'_>) -> Self {
        let mut cursor = source_file.walk();
        let mut funcs = vec![];
        for func in source_file.funcs(&mut cursor) {
            let func = func.expect_matching();
            funcs.push(Func::from_grammar(src.clone(), func));
        }
        Self { funcs }
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
