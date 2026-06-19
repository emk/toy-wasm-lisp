use miette::Result;
use wasm_encoder::ValType;

// TODO: This type is still shared with the parser. We may
// make it a standalone type at some point.
use crate::parser::grammar::Type;

impl Type {
    pub fn val_type(&self) -> Result<ValType> {
        match &self {
            Type::I32 => Ok(ValType::I32),
        }
    }
}
