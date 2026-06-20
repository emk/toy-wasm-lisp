use std::sync::Arc;

use miette::{NamedSource, Result, miette};
use rust_sitter::Spanned;
use wasm_encoder::InstructionSink;

use super::Ident;
use crate::{envs::ModuleEnv, locs::Loc, parser::grammar};

#[derive(Clone, Debug)]
pub struct Expr {
    #[expect(dead_code)]
    loc: Loc,
    variant: ExprVariant,
}

#[derive(Clone, Debug)]
pub enum ExprVariant {
    Number(i32),
    Add { expr1: Box<Expr>, expr2: Box<Expr> },
    Mul { expr1: Box<Expr>, expr2: Box<Expr> },
    Call { func_name: Ident, args: Vec<Expr> },
}

impl Expr {
    pub fn from_grammar(src: Arc<NamedSource<String>>, block: &Spanned<grammar::Expr>) -> Self {
        let loc = Loc::new(src.clone(), block);
        let variant = match &block.value {
            grammar::Expr::Number(value) => ExprVariant::Number(*value),
            grammar::Expr::Add(expr1, _, expr2) => ExprVariant::Add {
                expr1: Box::new(Expr::from_grammar(src.clone(), expr1)),
                expr2: Box::new(Expr::from_grammar(src.clone(), expr2)),
            },
            grammar::Expr::Mul(expr1, _, expr2) => ExprVariant::Mul {
                expr1: Box::new(Expr::from_grammar(src.clone(), expr1)),
                expr2: Box::new(Expr::from_grammar(src.clone(), expr2)),
            },
            grammar::Expr::Call {
                func_name, args, ..
            } => ExprVariant::Call {
                func_name: Ident::from_grammar(src.clone(), func_name),
                args: args
                    .iter()
                    .map(|arg| Expr::from_grammar(src.clone(), arg))
                    .collect(),
            },
        };
        Self { loc, variant }
    }

    pub fn emit(&self, env: &ModuleEnv, sink: &mut InstructionSink<'_>) -> Result<()> {
        match &self.variant {
            ExprVariant::Number(i) => {
                sink.i32_const(*i);
            }
            ExprVariant::Add { expr1, expr2 } => {
                expr1.emit(env, sink)?;
                expr2.emit(env, sink)?;
                sink.i32_add();
            }
            ExprVariant::Mul { expr1, expr2 } => {
                expr1.emit(env, sink)?;
                expr2.emit(env, sink)?;
                sink.i32_mul();
            }
            ExprVariant::Call { func_name, args } => {
                let (idx, func) = env.symbol_table().get_func(func_name)?;
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
                sink.call(idx.try_as_u32()?);
            }
        }
        Ok(())
    }
}
