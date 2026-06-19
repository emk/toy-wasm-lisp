use miette::{Result, miette};
use wasm_encoder::InstructionSink;

use crate::envs::ModuleEnv;

use super::grammar::Expr;

impl Expr {
    pub fn emit(&self, env: &ModuleEnv, sink: &mut InstructionSink<'_>) -> Result<()> {
        match self {
            Expr::Number(i) => {
                sink.i32_const(*i);
            }
            Expr::Add(expr1, _, expr2) => {
                expr1.emit(env, sink)?;
                expr2.emit(env, sink)?;
                sink.i32_add();
            }
            Expr::Mul(expr1, _, expr2) => {
                expr1.emit(env, sink)?;
                expr2.emit(env, sink)?;
                sink.i32_mul();
            }
            Expr::Call {
                func_name, args, ..
            } => {
                let (id, func) = env.func_map().get(func_name)?;
                let func_type = func.func_type()?;

                // TODO: Actually set up type checking.
                if func_type.params().len() != args.len() {
                    return Err(miette!(
                        "expected {} arguments, got {}",
                        func_type.params().len(),
                        args.len()
                    ));
                }
                if func_type.results().len() != 1 {
                    return Err(miette!(
                        "expected 1 result, got {}",
                        func_type.results().len()
                    ));
                }

                // Emit args and call.
                for arg in args {
                    arg.emit(env, sink)?;
                }
                sink.call(id.try_as_u32()?);
            }
        }
        Ok(())
    }
}
