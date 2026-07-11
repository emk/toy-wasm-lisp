//! Error types.

use miette::{Diagnostic, SourceSpan};

use crate::{
    ast::{ExprType, Ident},
    envs::SymbolCategory,
    locs::Loc,
};

/// A parse error.
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

impl ParseError {
    /// Create a new [`ParseError`].
    pub fn new(span: SourceSpan, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }
}

/// Multiple [`ParseError`]s that occurred during parsing.
#[derive(thiserror::Error, Debug, Diagnostic)]
#[error("could not parse program")]
pub struct ParseErrors {
    #[related]
    errs: Vec<ParseError>,
}

impl ParseErrors {
    /// Construct a set of parse errors from source code and errors.
    pub fn new(errs: Vec<ParseError>) -> Self {
        Self { errs }
    }
}

/// A [`crate::envs::SymbolTable`]-related error.
#[derive(thiserror::Error, Debug, Diagnostic)]
pub enum SymbolTableError {
    #[error("unknown identifier: {ident}")]
    UnknownIdentifier {
        ident: Ident,

        /// Location of the error.
        #[label("unknown identifier")]
        span: SourceSpan,
    },

    #[error("duplicate declaration: {ident}")]
    DuplicateDeclaration {
        ident: Ident,

        /// The new declaration that conflicts with the original.
        #[label(primary, "duplicate declaration")]
        span: SourceSpan,

        /// The original declaration we conflict with.
        #[label("original declaration")]
        original_span: SourceSpan,
    },

    #[error("expected {ident} to be {expected_category}, but it was {found_category}")]
    WrongSymbolCategory {
        ident: Ident,
        expected_category: SymbolCategory,
        found_category: SymbolCategory,

        /// The symbol that doesn't match.
        #[label(primary, "expected {expected_category}")]
        span: SourceSpan,
    },
}

impl SymbolTableError {
    pub fn unknown_identifier(ident: Ident) -> Self {
        let span = ident.src_span();
        Self::UnknownIdentifier { ident, span }
    }

    pub fn duplicate_declaration(ident: Ident, original: Ident) -> Self {
        let span = ident.src_span();
        let original_span = original.src_span();
        Self::DuplicateDeclaration {
            ident,
            span,
            original_span,
        }
    }

    pub fn wrong_symbol_category(
        ident: Ident,
        expected_category: SymbolCategory,
        found_category: SymbolCategory,
    ) -> Self {
        let span = ident.src_span();
        Self::WrongSymbolCategory {
            ident,
            expected_category,
            found_category,
            span,
        }
    }
}

/// Type checking errors.
#[derive(thiserror::Error, Debug, Diagnostic)]
pub enum TypeCheckError {
    #[error("expected type `{expected}`, but expression has type `{found}`")]
    IncompatibleTypes {
        expected: ExprType,
        found: ExprType,

        /// Location of the error.
        #[label("expected `{expected}`")]
        span: SourceSpan,
    },

    #[error("expected type `{right}` to be equal to `{left}`")]
    NotEqual {
        left: ExprType,
        right: ExprType,

        #[label("the type `{left}` here")]
        left_span: SourceSpan,

        #[label("does not match `{right}` here")]
        right_span: SourceSpan,
    },

    #[error("expected numeric type, found `{found}`")]
    NotNumeric {
        found: ExprType,

        #[label("found `{found}`")]
        span: SourceSpan,
    },

    #[error("wrong number of arguments (expected {expected}, found {found})")]
    WrongNumberOfArgs {
        expected: usize,
        found: usize,

        #[label("expected {expected} arguments")]
        span: SourceSpan,
    },

    #[error(
        "expression returned wrong number of values (expected {expected_count}, found type {ty}"
    )]
    WrongNumberOfValues {
        expected_count: usize,
        ty: ExprType,
        #[label("found type {ty}")]
        span: SourceSpan,
    },
}

impl TypeCheckError {
    pub fn not_expected(loc: &Loc, found: ExprType, expected: ExprType) -> Self {
        let span = loc.src_span();
        Self::IncompatibleTypes {
            expected,
            found,
            span,
        }
    }

    pub fn not_equal(left_loc: &Loc, left: ExprType, right_loc: &Loc, right: ExprType) -> Self {
        let left_span = left_loc.src_span();
        let right_span = right_loc.src_span();
        Self::NotEqual {
            left,
            right,
            left_span,
            right_span,
        }
    }

    pub fn not_numeric(loc: &Loc, found: ExprType) -> Self {
        let span = loc.src_span();
        Self::NotNumeric { found, span }
    }

    pub fn wrong_number_of_args(loc: &Loc, expected: usize, found: usize) -> Self {
        let span = loc.src_span();
        Self::WrongNumberOfArgs {
            expected,
            found,
            span,
        }
    }

    pub fn wrong_number_of_values(loc: &Loc, expected_count: usize, ty: ExprType) -> Self {
        let span = loc.src_span();
        Self::WrongNumberOfValues {
            expected_count,
            ty,
            span,
        }
    }
}
