// Parser based on [`rust_sitter`].

use std::sync::Arc;

use miette::{NamedSource, Result};
use tracing::debug;

use crate::{ast::Mod, errors::ParseErrors};

/// Parse `src`, using `
pub fn parse(filename: &str, src: &str) -> Result<Mod> {
    debug!(%filename, %src, "Parsing");
    let src = Arc::new(NamedSource::new(filename, src.to_owned()));
    let grammar_ast = grammar::parse(src.as_ref().inner())
        .map_err(|errs| ParseErrors::new(src.clone(), &errs))?;
    let ast = Mod::from_grammar(src, &grammar_ast);
    Ok(ast)
}

#[rust_sitter::grammar("wasl")]
pub mod grammar {
    use rust_sitter::Spanned;

    #[rust_sitter::language]
    #[derive(Debug)]
    pub struct Mod {
        pub funcs: Vec<Spanned<Func>>,
    }

    #[derive(Clone, Debug)]
    pub struct Func {
        #[rust_sitter::leaf(text = "export")]
        pub export: Option<()>,

        #[rust_sitter::leaf(text = "func")]
        _func: (),

        pub name: Ident,
        pub params: Spanned<Params>,
        pub returns: Spanned<Option<Returns>>,
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
        pub params: Vec<Spanned<Param>>,

        #[rust_sitter::leaf(text = ")")]
        _params_end: (),
    }

    #[derive(Clone, Debug)]
    pub struct Param {
        pub name: Ident,
        #[rust_sitter::leaf(text = ":")]
        _colon: (),
        pub ty: Spanned<Type>,
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

        pub expr: Spanned<Expr>,

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
        #[rust_sitter::prec(3)]
        Var(Ident),
        #[rust_sitter::prec(3)]
        Call {
            func_name: Ident,
            #[rust_sitter::leaf(text = "(")]
            _args_start: (),
            #[rust_sitter::delimited(
                #[rust_sitter::leaf(text = ",")]
                ()
            )]
            args: Vec<Spanned<Expr>>,
            #[rust_sitter::leaf(text = ")")]
            _args_end: (),
        },
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
