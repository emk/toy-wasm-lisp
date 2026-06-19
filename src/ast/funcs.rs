use miette::Result;
use wasm_encoder::{FuncType, Function, ValType};

use crate::envs::ModuleEnv;

use super::grammar::{Func, Params, Returns};

impl Func {
    pub fn is_exported(&self) -> bool {
        self.export.is_some()
    }

    fn param_types(&self) -> Result<Vec<ValType>> {
        self.params.types()
    }

    fn return_types(&self) -> Result<Vec<ValType>> {
        match &self.returns {
            None => Ok(vec![]),
            Some(returns) => returns.types(),
        }
    }

    pub fn func_type(&self) -> Result<FuncType> {
        Ok(FuncType::new(self.param_types()?, self.return_types()?))
    }

    pub fn emit_decl(&self, mod_env: &mut ModuleEnv) -> Result<()> {
        mod_env.insert_function(self.name.clone(), self.clone())
    }

    pub fn emit_impl(&self, mod_env: &mut ModuleEnv) -> Result<()> {
        let locals = vec![];
        let mut f = Function::new(locals);
        let mut sink = f.instructions();
        self.body.emit(mod_env, &mut sink)?;
        sink.end();
        mod_env.insert_code(&f);
        Ok(())
    }
}

impl Params {
    fn types(&self) -> Result<Vec<ValType>> {
        self.tys
            .iter()
            .map(|p| p.ty.value.val_type())
            .collect::<Result<Vec<_>>>()
    }
}

impl Returns {
    fn types(&self) -> Result<Vec<ValType>> {
        match self {
            Returns::Single { ty, .. } => Ok(vec![ty.val_type()?]),
            Returns::Multiple { tys, .. } => tys
                .iter()
                .map(|ty| ty.val_type())
                .collect::<Result<Vec<_>>>(),
        }
    }
}
