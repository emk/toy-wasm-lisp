//! Abstract syntax tree nodes for WASL.
//!
//! These are what we use internally, instead of the the "AST" produced by the
//! parser. Conversions are handled by various `TYPE::from_grammar` functions.
use miette::NamedSource;
use type_sitter::{NodeResult, raw};

pub use self::{
    blocks::Block,
    exprs::Expr,
    funcs::{Func, FuncSig},
    idents::Ident,
    imports::Import,
    locals::Local,
    mods::Mod,
    types::{ExprType, ValType},
};

mod blocks;
mod exprs;
mod funcs;
mod idents;
mod imports;
mod locals;
mod mods;
mod types;

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

/// Expect UTF-8 text in parsed data. We require UTF-8 input, so this
/// should always succeed.
pub fn node_source<'src>(src: &'src NamedSource<String>, node: &raw::Node<'_>) -> &'src str {
    node.utf8_text(src.inner().as_bytes())
        .expect("should always be UTF-8")
}
