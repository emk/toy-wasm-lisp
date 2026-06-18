use miette::Result;
use wasm_encoder::ValType;

use super::grammar::{Func, Params, Returns};

impl Func {
    pub fn is_exported(&self) -> bool {
        self.export.is_some()
    }

    pub fn param_types(&self) -> Result<Vec<ValType>> {
        self.params.types()
    }

    pub fn return_types(&self) -> Result<Vec<ValType>> {
        match &self.returns {
            None => Ok(vec![]),
            Some(returns) => returns.types(),
        }
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
