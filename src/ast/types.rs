use miette::Result;
use wasm_encoder::ValType;

use crate::parser::grammar::Type;

impl Type {
    pub fn val_type(&self) -> Result<ValType> {
        match &self {
            Type::I32 => Ok(ValType::I32),
        }
    }
}
