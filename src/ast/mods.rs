//! Implementations for [`Module`].

use miette::Result;

use crate::envs::ModuleEnv;

use super::grammar::Mod;

impl Mod {
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
