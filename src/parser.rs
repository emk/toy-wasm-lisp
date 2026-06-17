use std::sync::Arc;

use miette::{NamedSource, Result};
use tracing::debug;

use crate::errors::ParseErrors;

pub fn parse(filename: &str, src: &str) -> Result<self::grammar::Expr> {
    debug!(%filename, %src, "Parsing");
    let src = Arc::new(NamedSource::new(filename, src.to_owned()));
    Ok(
        grammar::parse(src.as_ref().inner())
            .map_err(|errs| ParseErrors::new(src.clone(), &errs))?,
    )
}

#[rust_sitter::grammar("arithmetic")]
pub mod grammar {
    #[rust_sitter::language]
    #[derive(Debug, Eq, PartialEq)]
    pub enum Expr {
        Number(#[rust_sitter::leaf(pattern = r"\d+", transform = |v| v.parse().unwrap())] i32),
        #[rust_sitter::prec_left(1)]
        Add(Box<Expr>, #[rust_sitter::leaf(text = "+")] (), Box<Expr>),
        #[rust_sitter::prec_left(2)]
        Mul(Box<Expr>, #[rust_sitter::leaf(text = "*")] (), Box<Expr>),
    }

    #[rust_sitter::extra]
    #[allow(dead_code)]
    struct Whitespace {
        #[rust_sitter::leaf(pattern = r"\s")]
        _whitespace: (),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grammar::Expr;

    #[test]
    fn successful_parses() {
        assert_eq!(
            parse("<test>", "1 + 2 * 3").unwrap(),
            Expr::Add(
                Box::new(Expr::Number(1)),
                (),
                Box::new(Expr::Mul(
                    Box::new(Expr::Number(2)),
                    (),
                    Box::new(Expr::Number(3))
                ))
            )
        );
    }
}
