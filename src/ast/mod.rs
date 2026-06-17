//! Abstract syntax tree nodes for WASL.
//!
//! Note that the actual node _types_ are declared in the [`rust_sitter`]
//! grammar. These modules just contain the `impl` blocks, to keep the
//! grammar slightly cleaner.

mod blocks;
mod exprs;
mod funcs;
mod types;
