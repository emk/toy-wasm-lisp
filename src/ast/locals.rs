//! Local variables.

use miette::Result;
use tree_sitter_wasl_types::nodes;
use type_sitter::Node as _;
use wasm_encoder::InstructionSink;

use super::{ExprType, Ident, ValType};
use crate::{
    ast::{Emit, Expr, FromGrammar, GetExprType, InferExprType, NodeResultExt as _},
    envs::{DeclIdx, FuncEnv, SymbolTable},
    errors::ParseError,
    locs::{Loc, Source},
};

/// Local variable statement (`local x = ...;`).
#[derive(Clone, Debug)]
pub struct LocalStmt {
    pub loc: Loc,
    name: Ident,
    expr: Expr,
    /// Inferred at type inference time.
    inferred_idx: Option<DeclIdx<LocalSymbol>>,
}

impl FromGrammar for LocalStmt {
    type Input<'a> = nodes::Local<'a>;

    fn from_grammar(src: &Source, local: Self::Input<'_>) -> Result<Self, ParseError> {
        let loc = src.loc_for(local.raw());
        let name = Ident::from_grammar(src, local.name().expect_matching())?;
        let expr = Expr::from_grammar(src, local.expr().expect_matching())?;
        Ok(LocalStmt {
            loc,
            name,
            expr,
            inferred_idx: None,
        })
    }
}

impl InferExprType for LocalStmt {
    /// Handle type inference for a `local x = expr` statement. Note that we
    /// expect to be called with a fresh child `syms` created by our caller,
    /// because every `local` creates a new implicit child scope, like in Rust
    /// and many other languages.
    fn infer_expr_type(
        &mut self,
        env: &mut FuncEnv,
        syms: &mut SymbolTable<'_>,
    ) -> Result<ExprType> {
        let ty = self.expr.infer_expr_type(env, syms)?;

        // We only support single values for now, though we'll probably add
        // `local (x, y) = expr;` later.
        ty.expecting_value_count(&self.loc, 1)?;
        let ty = ty.expect_single();

        // Insert this into
        let local = LocalSymbol::new(self.name.clone(), ty.clone());
        let idx = env.insert_local(self.name.clone(), local, syms)?;
        self.inferred_idx = Some(idx);

        // We always return a void value.
        Ok(ExprType::void())
    }
}

impl Emit for LocalStmt {
    fn emit(&self, sink: &mut InstructionSink<'_>) -> Result<()> {
        self.expr.emit(sink)?;
        let idx = self
            .inferred_idx
            .expect("idx should be inferred at type inference time");

        sink.local_set(idx.try_as_u32()?);
        Ok(())
    }
}

/// Local variable declaration.
#[derive(Clone, Debug)]
pub struct LocalSymbol {
    #[expect(dead_code)]
    name: Ident,
    ty: ValType,
}

impl LocalSymbol {
    pub fn new(name: Ident, ty: ValType) -> Self {
        Self { name, ty }
    }

    #[cfg(test)]
    pub fn i32_for_test(name: &str) -> Self {
        Self {
            name: Ident::new_for_test(name),
            ty: ValType::i32_for_test(),
        }
    }

    pub fn emit_get(&self, idx: DeclIdx<LocalSymbol>, sink: &mut InstructionSink) -> Result<()> {
        sink.local_get(idx.try_as_u32()?);
        Ok(())
    }
}

impl GetExprType for LocalSymbol {
    fn expr_type(&self) -> Result<ExprType> {
        Ok(ExprType::single(self.ty.clone()))
    }
}
