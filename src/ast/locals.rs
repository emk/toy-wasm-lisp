//! Local variables.

use miette::Result;
use wasm_encoder::InstructionSink;

use super::{Ident, Type};
use crate::envs::DeclIdx;

/// Local variable declaration.
#[derive(Clone, Debug)]
pub struct Local {
    #[expect(dead_code)]
    name: Ident,
    #[expect(dead_code)]
    ty: Type,
}

impl Local {
    pub fn new(name: Ident, ty: Type) -> Self {
        Self { name, ty }
    }

    #[cfg(test)]
    pub fn new_i32_for_test(name: &str) -> Self {
        Self {
            name: Ident::new_for_test(name),
            ty: Type::new_i32_for_test(),
        }
    }

    pub fn emit_get(&self, idx: DeclIdx<Local>, sink: &mut InstructionSink) -> Result<()> {
        sink.local_get(idx.try_as_u32()?);
        Ok(())
    }
}
