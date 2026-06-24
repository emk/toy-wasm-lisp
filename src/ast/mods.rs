//! Implementations for [`Mod`].

use std::sync::Arc;

use miette::{NamedSource, Result};
use tree_sitter_wasl_types::nodes;
use type_sitter::Node as _;

use super::{Func, Import, NodeResultExt};
use crate::{ast::Ident, envs::ModuleEnv};

#[derive(Clone, Debug)]
pub struct Mod {
    imports: Vec<Import>,
    funcs: Vec<Func>,
}

impl Mod {
    pub fn from_grammar(src: Arc<NamedSource<String>>, source_file: nodes::SourceFile<'_>) -> Self {
        let mut cursor = source_file.walk();
        let mut imports = vec![];
        let mut funcs = vec![];
        for tl in source_file.top_levels(&mut cursor) {
            let tl = tl.expect_matching();
            match tl {
                nodes::TopLevel::Func(func) => funcs.push(Func::from_grammar(src.clone(), func)),
                nodes::TopLevel::ImportBlock(import_block) => {
                    let mod_name =
                        Ident::from_grammar(src.clone(), import_block.mod_name().expect_matching());
                    let mut c = import_block.walk();
                    for import in import_block.imports(&mut c) {
                        let import = import.expect_matching();
                        imports.push(Import::from_grammar(src.clone(), mod_name.clone(), import));
                    }
                }
            }
        }
        Self { imports, funcs }
    }

    pub fn emit(&self) -> Result<Vec<u8>> {
        // Create our module-level environment, which contains
        // all the state needed to emit code.
        let mut module_env = ModuleEnv::new();

        //
        for i in &self.imports {
            i.emit_decl(&mut module_env)?;
        }
        for f in &self.funcs {
            f.emit_decl(&mut module_env)?;
        }
        for f in &self.funcs {
            f.emit_impl(&mut module_env)?;
        }
        let module = module_env.build_module();
        let wasm_bytes = module.finish();

        // Check to make sure we didn't mess it up.
        wasmparser::validate(&wasm_bytes).expect("wasm should be valid");
        Ok(wasm_bytes)
    }
}
