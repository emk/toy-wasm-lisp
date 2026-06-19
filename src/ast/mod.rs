//! Abstract syntax tree nodes for WASL.
//!
//! Note that the actual node _types_ are declared in [`self::grammar`]
//! grammar, but the actual

use std::sync::Arc;

use miette::{NamedSource, Result};
use tracing::debug;

use crate::errors::ParseErrors;

mod blocks;
mod exprs;
mod funcs;
mod idents;
mod mods;
mod types;

pub fn parse(filename: &str, src: &str) -> Result<self::grammar::Mod> {
    debug!(%filename, %src, "Parsing");
    let src = Arc::new(NamedSource::new(filename, src.to_owned()));
    Ok(
        grammar::parse(src.as_ref().inner())
            .map_err(|errs| ParseErrors::new(src.clone(), &errs))?,
    )
}

#[rust_sitter::grammar("wasl")]
pub mod grammar {
    use rust_sitter::Spanned;

    #[rust_sitter::language]
    #[derive(Debug)]
    pub struct Mod {
        pub(super) func: Func,
    }

    #[derive(Clone, Debug)]
    pub struct Func {
        #[rust_sitter::leaf(text = "export")]
        pub(super) export: Option<()>,

        #[rust_sitter::leaf(text = "func")]
        _func: (),

        pub name: Ident,
        pub(super) params: Params,
        pub(super) returns: Option<Returns>,
        pub body: Spanned<Block>,
    }

    #[derive(Clone, Debug)]
    pub struct Params {
        #[rust_sitter::leaf(text = "(")]
        _params_start: (),

        #[rust_sitter::delimited(
            #[rust_sitter::leaf(text = ",")]
            ()
        )]
        pub(super) tys: Vec<Spanned<Param>>,

        #[rust_sitter::leaf(text = ")")]
        _params_end: (),
    }

    #[derive(Clone, Debug)]
    pub struct Param {
        _name: Ident,
        #[rust_sitter::leaf(text = ":")]
        _colon: (),
        pub(super) ty: Spanned<Type>,
    }

    #[derive(Clone, Debug)]
    pub enum Returns {
        Single {
            #[rust_sitter::leaf(text = "->")]
            _arrow: (),
            ty: Spanned<Type>,
        },
        Multiple {
            #[rust_sitter::leaf(text = "->")]
            _arrow: (),
            #[rust_sitter::leaf(text = "(")]
            _results_start: (),
            #[rust_sitter::delimited(
                #[rust_sitter::leaf(text = ",")]
                ()
            )]
            tys: Vec<Spanned<Type>>,
            #[rust_sitter::leaf(text = ")")]
            _results_end: (),
        },
    }

    #[derive(Clone, Copy, Debug)]
    pub enum Type {
        #[rust_sitter::leaf(text = "i32")]
        I32,
    }

    #[derive(Clone, Debug)]
    pub struct Block {
        #[rust_sitter::leaf(text = "{")]
        _body_start: (),

        pub(super) expr: Spanned<Expr>,

        #[rust_sitter::leaf(text = "}")]
        _body_end: (),
    }

    #[derive(Clone, Debug)]
    pub enum Expr {
        Number(#[rust_sitter::leaf(pattern = r"\d+", transform = |v| v.parse().unwrap())] i32),
        #[rust_sitter::prec_left(1)]
        Add(
            Box<Spanned<Expr>>,
            #[rust_sitter::leaf(text = "+")] (),
            Box<Spanned<Expr>>,
        ),
        #[rust_sitter::prec_left(2)]
        Mul(
            Box<Spanned<Expr>>,
            #[rust_sitter::leaf(text = "*")] (),
            Box<Spanned<Expr>>,
        ),
    }

    #[derive(Clone, Debug)]
    pub struct Ident {
        #[rust_sitter::word]
        #[rust_sitter::leaf(pattern = "[_a-zA-Z][_a-zA-Z0-9]*", transform = |v| v.to_string())]
        pub text: Spanned<String>,
    }

    #[rust_sitter::extra]
    #[allow(dead_code)]
    struct Whitespace {
        #[rust_sitter::leaf(pattern = r"\s|(//.*)")]
        _whitespace: (),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_parses() {
        parse("<test>", "func f() -> (i32) { 1 + 2 * 3 }").unwrap();
    }
}
