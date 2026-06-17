use std::sync::Arc;

use miette::{Diagnostic, NamedSource, SourceSpan};
use rust_sitter::errors::{ParseError as RustSitterParseError, ParseErrorReason};

use tracing::debug;

#[derive(thiserror::Error, Debug, Diagnostic)]
#[error("{message}")]
#[diagnostic()]
pub struct ParseError {
    // The location of the error.
    #[label("here")]
    span: SourceSpan,

    // Text of the error.
    message: String,
}

#[derive(thiserror::Error, Debug, Diagnostic)]
#[error("could not parse program")]
#[diagnostic()]
pub struct ParseErrors {
    // Source file containing the error.
    #[source_code]
    src: Arc<NamedSource<String>>,

    #[related]
    errs: Vec<ParseError>,
}

impl ParseErrors {
    pub fn new(src: Arc<NamedSource<String>>, sitter_errs: &[RustSitterParseError]) -> Self {
        debug!("Errors: {sitter_errs:?}");
        let mut errs = vec![];
        collect_errs(sitter_errs, &mut errs);
        Self { src, errs }
    }
}

fn collect_errs(sitter_errs: &[RustSitterParseError], out_errs: &mut Vec<ParseError>) {
    for err in sitter_errs {
        let span = SourceSpan::from(err.start..err.end);
        match &err.reason {
            ParseErrorReason::UnexpectedToken(token) => {
                out_errs.push(ParseError {
                    span,
                    message: format!("unexpected token: {:?}", token),
                });
            }
            ParseErrorReason::FailedNode(parse_errors) => {
                // Keep walking down the tree until we find real errors,
                // I guess?
                out_errs.push(ParseError {
                    span,
                    message: "could not parse".to_owned(),
                });
                collect_errs(parse_errors, out_errs);
            }
            ParseErrorReason::MissingToken(token) => {
                out_errs.push(ParseError {
                    span,
                    message: format!("missing token: {:?}", token),
                });
            }
        }
    }
}
