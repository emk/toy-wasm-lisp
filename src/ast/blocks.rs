use miette::Result;
use tree_sitter_wasl_types::nodes;
use type_sitter::Node as _;
use wasm_encoder::InstructionSink;

use super::{Expr, ExprType};
use crate::{
    ast::{Emit, FromGrammar, InferExprType, NodeResultExt as _, locals::LocalStmt},
    envs::{FuncEnv, SymbolTable},
    errors::ParseError,
    locs::{Loc, Source},
};

/// A "{ ... }" block containing statements. This includes function bodies, etc.
#[derive(Clone, Debug)]
pub struct Block {
    pub loc: Loc,
    stmts: Vec<Stmt>,
    trailing_semi: Option<Loc>,
}

impl FromGrammar for Block {
    type Input<'a> = nodes::Block<'a>;

    fn from_grammar(src: &Source, block: Self::Input<'_>) -> Result<Self, ParseError> {
        let loc = src.loc_for(block.raw());
        let mut stmts = vec![];
        let mut c = block.walk();
        for stmt in block.stmts(&mut c) {
            let stmt = stmt.expect_matching();
            stmts.push(Stmt::from_grammar(src, stmt)?);
        }
        let trailing_semi = block
            .trailing_semi()
            .map(|semi| src.loc_for(semi.expect_matching().raw()));
        Ok(Self {
            loc,
            stmts,
            trailing_semi,
        })
    }
}

impl InferExprType for Block {
    fn infer_expr_type(
        &mut self,
        env: &mut FuncEnv,
        syms: &mut SymbolTable<'_>,
    ) -> Result<ExprType> {
        infer_expr_type_helper(&mut self.stmts, self.trailing_semi.is_some(), env, syms)
    }
}

/// Helper function for processing a series of statements in a block.
/// `local` statements introduce a new child `env`.
fn infer_expr_type_helper(
    stmts: &mut [Stmt],
    has_trailing_semi: bool,
    env: &mut FuncEnv,
    syms: &mut SymbolTable<'_>,
) -> Result<ExprType> {
    let stmts_len = stmts.len();
    if stmts_len == 0 {
        Ok(ExprType::void())
    } else {
        let (first, rest) = stmts
            .split_first_mut()
            .expect("should always have at least one item");
        match first {
            Stmt::Local(local) => {
                let mut syms = syms.child();
                let _void_ty = local.infer_expr_type(env, &mut syms)?;
                infer_expr_type_helper(rest, has_trailing_semi, env, &mut syms)
            }
            Stmt::Expr(expr) => {
                let ty = expr.infer_expr_type(env, syms)?;
                if !rest.is_empty() || has_trailing_semi {
                    // Either we're not the last statement, or we're followed by
                    // a semi-colon, so nobody is expecting us to return a
                    // value.
                    ty.expecting(expr.loc(), &ExprType::void())?
                }
                if rest.is_empty() {
                    Ok(ty)
                } else {
                    infer_expr_type_helper(rest, has_trailing_semi, env, syms)
                }
            }
        }
    }
}

impl Emit for Block {
    fn emit(&self, sink: &mut InstructionSink<'_>) -> Result<()> {
        for stmt in &self.stmts {
            stmt.emit(sink)?;
        }
        Ok(())
    }
}

/// Statements in a block.
#[derive(Clone, Debug)]
pub enum Stmt {
    Expr(Expr),
    Local(LocalStmt),
}

impl FromGrammar for Stmt {
    type Input<'a> = nodes::Stmt<'a>;

    fn from_grammar(src: &Source, stmt: Self::Input<'_>) -> Result<Self, ParseError> {
        match stmt {
            nodes::Stmt::Expr(expr) => Ok(Stmt::Expr(Expr::from_grammar(src, expr)?)),
            nodes::Stmt::Local(local) => Ok(Stmt::Local(LocalStmt::from_grammar(src, local)?)),
        }
    }
}

impl Emit for Stmt {
    fn emit(&self, sink: &mut InstructionSink<'_>) -> Result<()> {
        match self {
            Stmt::Expr(expr) => expr.emit(sink),
            Stmt::Local(local) => local.emit(sink),
        }
    }
}
