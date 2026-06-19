use std::sync::Arc;

use miette::{NamedSource, Result};
use rust_sitter::Spanned;
use wasm_encoder::InstructionSink;

use super::Expr;
use crate::{envs::ModuleEnv, locs::Loc, parser::grammar};

#[derive(Clone, Debug)]
pub struct Block {
    #[expect(dead_code)]
    loc: Loc,
    expr: Expr,
}

impl Block {
    pub fn from_grammar(src: Arc<NamedSource<String>>, block: &Spanned<grammar::Block>) -> Self {
        let loc = Loc::new(src.clone(), block);
        let grammar::Block { expr, .. } = &block.value;
        Self {
            loc,
            expr: Expr::from_grammar(src, expr),
        }
    }

    pub fn emit(&self, env: &ModuleEnv, sink: &mut InstructionSink<'_>) -> Result<()> {
        self.expr.emit(env, sink)
    }
}
