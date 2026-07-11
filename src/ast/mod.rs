//! Abstract syntax tree nodes for WASL.
//!
//! These are what we use internally, instead of the the "AST" produced by the
//! parser. Conversions are handled by various `TYPE::from_grammar` functions.
use miette::Result;
use type_sitter::NodeResult;
use wasm_encoder::InstructionSink;

pub use self::{
    blocks::Block,
    exprs::Expr,
    funcs::{Func, FuncSig},
    idents::Ident,
    imports::Import,
    locals::LocalSymbol,
    mods::Mod,
    types::{ExprType, ToWasmType, ValType},
};
use crate::{
    envs::{FuncEnv, SymbolTable},
    errors::ParseError,
    locs::Source,
};

mod blocks;
mod exprs;
mod funcs;
mod idents;
mod imports;
mod locals;
mod mods;
mod types;

/// Convert a [`type_sitter`] parse tree node into an internal AST type.
pub trait FromGrammar: Sized {
    /// Our [`type_sitter`] input type.
    type Input<'a>;

    /// Convert `node` into our AST type. `src` is the current source file.
    fn from_grammar(src: &Source, node: Self::Input<'_>) -> Result<Self, ParseError>;
}

/// Infer the type of an expression, performing any type checks as we go.
///
/// This may update the AST node with type information where needed for later
/// passes.
pub trait InferExprType {
    fn infer_expr_type(
        &mut self,
        env: &mut FuncEnv,
        syms: &mut SymbolTable<'_>,
    ) -> Result<ExprType>;
}

/// Get the type of an expression.
///
/// This does not perform any type inference or mutate the object. It is used to
/// access the already-known types of things like resolved symbols.
pub trait GetExprType {
    fn expr_type(&self) -> Result<ExprType>;
}

/// Emit code for a node.
pub trait Emit {
    /// Emit instructions to an [`InstructionSink`].
    fn emit(&self, sink: &mut InstructionSink<'_>) -> Result<()>;
}

/// Extension used to verify that our parse tree matches our grammar. This
/// produces panics because if the grammar and parse tree don't match, something
/// has gone very wrong.
pub trait NodeResultExt {
    type Unwrapped;

    /// At this point in the grammar, we expect a valid node of the correct type.
    /// Errors and missing nodes should have been dealt with after parsing, which means that remaining
    /// issues are probably grammar mismatch problems.
    fn expect_matching(self) -> Self::Unwrapped;
}

impl<'tree, T> NodeResultExt for NodeResult<'tree, T> {
    type Unwrapped = T;
    fn expect_matching(self) -> Self::Unwrapped {
        self.expect("grammar mismatch")
    }
}

impl<'tree, T> NodeResultExt for Option<NodeResult<'tree, T>> {
    type Unwrapped = Option<T>;

    fn expect_matching(self) -> Self::Unwrapped {
        self.map(|result| result.expect_matching())
    }
}
