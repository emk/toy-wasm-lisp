//! Error types.

use std::sync::Arc;

use miette::{Diagnostic, NamedSource, SourceSpan};
use rust_sitter::errors::{ParseError as RustSitterParseError, ParseErrorReason};

use tracing::debug;

use crate::ast::grammar::Ident;

/// A [`rust_sitter`]-derived parse error.
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

/// Multiple [`ParseError`]s that occurred during parsing.
#[derive(thiserror::Error, Debug, Diagnostic)]
#[error("could not parse program")]
pub struct ParseErrors {
    // Source file containing the error.
    #[source_code]
    src: Arc<NamedSource<String>>,

    #[related]
    errs: Vec<ParseError>,
}

impl ParseErrors {
    /// Construct a set of parse errors from source code and [`RustSitterParseError`]s. This
    /// will filter out less interesting errors and truncate after a few errors.
    pub fn new(src: Arc<NamedSource<String>>, sitter_errs: &[RustSitterParseError]) -> Self {
        debug!("Errors: {sitter_errs:?}");
        let mut errs = vec![];
        collect_errs(sitter_errs, &mut errs);
        if errs.len() > 3 {
            errs.truncate(3);
        }
        Self { src, errs }
    }
}

/// Recursively collect parse errors.
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
                let before_count = out_errs.len();
                collect_errs(parse_errors, out_errs);

                // If we haven't pushed any better child error, try
                // something more generic.
                if out_errs.len() == before_count {
                    out_errs.push(ParseError {
                        span,
                        message: "could not parse".to_owned(),
                    });
                }
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

/// An unknown identifier error.
#[derive(thiserror::Error, Debug, Diagnostic)]
#[error("unknown identifier: {ident}")]
pub struct UnknownIdentifierError {
    ident: Ident,

    #[label("unknown identifier")]
    span: SourceSpan,
}

impl UnknownIdentifierError {
    pub fn new(ident: Ident) -> Self {
        let span = SourceSpan::from(ident.text.span);
        Self { ident, span }
    }
}

/// A duplicate declaration error.
#[derive(thiserror::Error, Debug, Diagnostic)]
#[error("duplicate declaration: {ident}")]
pub struct DuplicateDeclarationError {
    ident: Ident,

    #[label("original declaration")]
    original_span: SourceSpan,

    #[label(primary, "duplicate declaration")]
    duplicate_span: SourceSpan,
}

impl DuplicateDeclarationError {
    pub fn new(duplicate_ident: Ident, original_ident: Ident) -> Self {
        let original_span = SourceSpan::from(original_ident.text.span);
        let duplicate_span = SourceSpan::from(duplicate_ident.text.span);
        Self {
            ident: duplicate_ident,
            original_span,
            duplicate_span,
        }
    }
}
