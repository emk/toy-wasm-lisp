use miette::Result;
use wasm_encoder::InstructionSink;

use crate::parser::grammar::Block;

impl Block {
    pub fn emit(&self, sink: &mut InstructionSink<'_>) -> Result<()> {
        self.expr.emit(sink)
    }
}
