use std::sync::Arc;

use miette::{NamedSource, Result};
use rust_sitter::Spanned;
use wasm_encoder::ValType;

use crate::{locs::Loc, parser::grammar};

#[derive(Clone, Debug)]
pub struct Type {
    #[expect(dead_code)]
    loc: Loc,
    variant: TypeVariant,
}

#[derive(Clone, Debug)]
pub enum TypeVariant {
    I32,
}

impl Type {
    pub fn from_grammar(src: Arc<NamedSource<String>>, ty: &Spanned<grammar::Type>) -> Self {
        let loc = Loc::new(src, ty);
        let variant = match &ty.value {
            grammar::Type::I32 => TypeVariant::I32,
        };
        Type { loc, variant }
    }

    #[cfg(test)]
    pub fn new_i32_for_test() -> Self {
        Self {
            loc: Loc::new_for_test(),
            variant: TypeVariant::I32,
        }
    }

    pub fn val_type(&self) -> Result<ValType> {
        match &self.variant {
            TypeVariant::I32 => Ok(ValType::I32),
        }
    }
}
