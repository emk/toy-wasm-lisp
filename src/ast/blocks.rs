use miette::Result;
use wasm_encoder::InstructionSink;

use crate::envs::ModuleEnv;

use super::grammar::Block;

impl Block {
    pub fn emit(&self, env: &ModuleEnv, sink: &mut InstructionSink<'_>) -> Result<()> {
        self.expr.emit(env, sink)
    }
}
