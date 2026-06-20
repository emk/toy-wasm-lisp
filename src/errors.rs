//! Error types.

use std::sync::Arc;

use miette::{Diagnostic, NamedSource, SourceSpan};
use rust_sitter::errors::{ParseError as RustSitterParseError, ParseErrorReason};

use tracing::debug;

use crate::{ast::Ident, envs::SymbolCategory};

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

/// A [`SymbolTable`]-related error.
#[derive(thiserror::Error, Debug, Diagnostic)]
pub enum SymbolTableError {
    #[error("unknown identifier: {ident}")]
    UnknownIdentifier {
        ident: Ident,

        /// Source file containing the error.
        #[source_code]
        src: Arc<NamedSource<String>>,

        /// Location of the error.
        #[label("unknown identifier")]
        span: SourceSpan,
    },

    #[error("duplicate declaration: {ident}")]
    DuplicateDeclaration {
        ident: Ident,

        /// Source file containing the error.
        #[source_code]
        src: Arc<NamedSource<String>>,

        /// The new declaration that conflicts with the original.
        #[label(primary, "duplicate declaration")]
        span: SourceSpan,

        /// The original declaration we conflict with.
        ///
        /// Because [`miette`] only supports one source code per error (at least
        /// without shenanigans), we will set this to `None` if it occurs in a
        /// different file than `Self::duplicate_span`.
        #[label("original declaration")]
        original_span: Option<SourceSpan>,
    },

    #[error("expected {ident} to be {expected_category}, but it was {found_category}")]
    WrongSymbolCategory {
        ident: Ident,
        expected_category: SymbolCategory,
        found_category: SymbolCategory,

        /// Source file containing the error.
        #[source_code]
        src: Arc<NamedSource<String>>,

        /// The symbol that doesn't match.
        #[label(primary, "expected {expected_category}")]
        span: SourceSpan,
    },
}

impl SymbolTableError {
    pub fn unknown_identifier(ident: Ident) -> Self {
        let src = ident.src();
        let span = ident.src_span();
        Self::UnknownIdentifier { ident, src, span }
    }

    pub fn duplicate_declaration(ident: Ident, original: Ident) -> Self {
        let src = ident.src();
        let span = ident.src_span();
        let original_span = if Arc::ptr_eq(&src, &original.src()) {
            Some(original.src_span())
        } else {
            None
        };
        Self::DuplicateDeclaration {
            ident,
            src,
            span,
            original_span,
        }
    }

    pub fn wrong_symbol_category(
        ident: Ident,
        expected_category: SymbolCategory,
        found_category: SymbolCategory,
    ) -> Self {
        let src = ident.src();
        let span = ident.src_span();
        Self::WrongSymbolCategory {
            ident,
            expected_category,
            found_category,
            src,
            span,
        }
    }
}
