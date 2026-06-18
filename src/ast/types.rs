use miette::Result;
use wasm_encoder::ValType;

use super::grammar::Type;

impl Type {
    pub fn val_type(&self) -> Result<ValType> {
        match &self {
            Type::I32 => Ok(ValType::I32),
        }
    }
}
