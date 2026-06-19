//! Abstract syntax tree nodes for WASL.
//!
//! Note that the actual node _types_ are declared in [`crate::parser::grammar`]
//! grammar, but the actual

pub use self::{blocks::Block, exprs::Expr, funcs::Func, mods::Mod};

mod blocks;
mod exprs;
mod funcs;
mod idents;
mod mods;
mod types;
