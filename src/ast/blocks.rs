use miette::Result;
use tree_sitter_wasl_types::nodes;
use type_sitter::Node as _;
use wasm_encoder::InstructionSink;

use super::Expr;
use crate::{
    ast::{FromGrammar, InferExprType, NodeResultExt as _},
    envs::{LocalEnv, SymbolTable},
    errors::ParseError,
    locs::{Loc, Source},
};

#[derive(Clone, Debug)]
pub struct Block {
    pub loc: Loc,
    expr: Expr,
}

impl Block {
    pub fn emit(&self, env: &LocalEnv<'_>, sink: &mut InstructionSink<'_>) -> Result<()> {
        self.expr.emit(env, sink)
    }
}

impl FromGrammar for Block {
    type Input<'a> = nodes::Block<'a>;

    fn from_grammar(src: &Source, block: Self::Input<'_>) -> Result<Self, ParseError> {
        let loc = src.loc_for(block.raw());
        Ok(Self {
            loc,
            expr: Expr::from_grammar(src, block.expr().expect_matching())?,
        })
    }
}

impl InferExprType for Block {
    fn infer_expr_type(&mut self, symbol_table: &SymbolTable<'_>) -> Result<super::ExprType> {
        self.expr.infer_expr_type(symbol_table)
    }
}
