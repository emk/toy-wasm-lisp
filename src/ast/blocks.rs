use miette::Result;
use tree_sitter_wasl_types::nodes;
use type_sitter::Node as _;
use wasm_encoder::InstructionSink;

use super::{Expr, ExprType};
use crate::{
    ast::{FromGrammar, InferExprType, NodeResultExt as _},
    envs::{LocalEnv, SymbolTable},
    errors::ParseError,
    locs::{Loc, Source},
};

#[derive(Clone, Debug)]
pub struct Block {
    pub loc: Loc,
    exprs: Vec<Expr>,
    trailing_semi: Option<Loc>,
}

impl Block {
    pub fn emit(&self, env: &LocalEnv<'_>, sink: &mut InstructionSink<'_>) -> Result<()> {
        for expr in &self.exprs {
            expr.emit(env, sink)?;
        }
        Ok(())
    }
}

impl FromGrammar for Block {
    type Input<'a> = nodes::Block<'a>;

    fn from_grammar(src: &Source, block: Self::Input<'_>) -> Result<Self, ParseError> {
        let loc = src.loc_for(block.raw());
        let mut exprs = vec![];
        let mut c = block.walk();
        for expr in block.exprs(&mut c) {
            let expr = expr.expect_matching();
            exprs.push(Expr::from_grammar(src, expr)?);
        }
        let trailing_semi = block
            .trailing_semi()
            .map(|semi| src.loc_for(semi.expect_matching().raw()));
        Ok(Self {
            loc,
            exprs,
            trailing_semi,
        })
    }
}

impl InferExprType for Block {
    fn infer_expr_type(&mut self, symbol_table: &SymbolTable<'_>) -> Result<ExprType> {
        let exprs_len = self.exprs.len();
        for (i, expr) in self.exprs.iter_mut().enumerate() {
            let is_last = i + 1 == exprs_len;
            if !is_last || self.trailing_semi.is_some() {
                let ty = expr.infer_expr_type(symbol_table)?;
                ty.expecting(&expr.loc, &ExprType::void())?;
            } else {
                return expr.infer_expr_type(symbol_table);
            }
        }
        Ok(ExprType::void())
    }
}
