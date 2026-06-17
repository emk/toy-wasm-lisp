use miette::Result;
use wasm_encoder::InstructionSink;

use crate::parser::grammar::Expr;

impl Expr {
    pub fn emit(&self, sink: &mut InstructionSink<'_>) -> Result<()> {
        match self {
            Expr::Number(i) => {
                sink.i32_const(*i);
            }
            Expr::Add(expr1, _, expr2) => {
                expr1.emit(sink)?;
                expr2.emit(sink)?;
                sink.i32_add();
            }
            Expr::Mul(expr1, _, expr2) => {
                expr1.emit(sink)?;
                expr2.emit(sink)?;
                sink.i32_mul();
            }
        }
        Ok(())
    }
}
