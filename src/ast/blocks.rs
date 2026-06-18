use miette::Result;
use wasm_encoder::InstructionSink;

use super::grammar::Block;

impl Block {
    pub fn emit(&self, sink: &mut InstructionSink<'_>) -> Result<()> {
        self.expr.emit(sink)
    }
}
