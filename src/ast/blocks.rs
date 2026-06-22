use std::sync::Arc;

use miette::{NamedSource, Result};
use tree_sitter_wasl_types::nodes;
use type_sitter::Node as _;
use wasm_encoder::InstructionSink;

use super::Expr;
use crate::{ast::NodeResultExt as _, envs::LocalEnv, locs::Loc};

#[derive(Clone, Debug)]
pub struct Block {
    #[expect(dead_code)]
    loc: Loc,
    expr: Expr,
}

impl Block {
    pub fn from_grammar(src: Arc<NamedSource<String>>, block: nodes::Block<'_>) -> Self {
        let loc = Loc::new(src.clone(), block.raw());
        Self {
            loc,
            expr: Expr::from_grammar(src, block.expr().expect_matching()),
        }
    }

    pub fn emit(&self, env: &LocalEnv<'_>, sink: &mut InstructionSink<'_>) -> Result<()> {
        self.expr.emit(env, sink)
    }
}
