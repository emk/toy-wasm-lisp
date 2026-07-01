//! Local variables.

use miette::Result;
use wasm_encoder::InstructionSink;

use super::{ExprType, Ident, ValType};
use crate::{
    ast::InferExprType,
    envs::{DeclIdx, SymbolTable},
};

/// Local variable declaration.
#[derive(Clone, Debug)]
pub struct Local {
    #[expect(dead_code)]
    name: Ident,
    ty: ValType,
}

impl Local {
    pub fn new(name: Ident, ty: ValType) -> Self {
        Self { name, ty }
    }

    #[cfg(test)]
    pub fn i32_for_test(name: &str) -> Self {
        Self {
            name: Ident::new_for_test(name),
            ty: ValType::i32_for_test(),
        }
    }

    pub fn emit_get(&self, idx: DeclIdx<Local>, sink: &mut InstructionSink) -> Result<()> {
        sink.local_get(idx.try_as_u32()?);
        Ok(())
    }
}
impl InferExprType for Local {
    fn infer_expr_type(&self, _symbol_table: &SymbolTable<'_>) -> Result<ExprType> {
        Ok(ExprType::single(self.ty.clone()))
    }
}
