//! Abstract syntax tree nodes for WASL.
//!
//! Note that the actual node _types_ are declared in [`crate::parser::grammar`]
//! grammar, but the actual

pub use self::{
    blocks::Block, exprs::Expr, funcs::Func, idents::Ident, locals::Local, mods::Mod, types::Type,
};

mod blocks;
mod exprs;
mod funcs;
mod idents;
mod locals;
mod mods;
mod types;
