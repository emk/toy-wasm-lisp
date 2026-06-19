//! Source location, for error messages.

use std::{
    fmt::{self, Write},
    ops::Range,
    sync::Arc,
};

use miette::NamedSource;
use rust_sitter::Spanned;

/// The location of an AST node in the source. This is normally used in
/// [`miette`] errors.
#[derive(Clone)]
pub struct Loc {
    /// Source file.
    pub src: Arc<NamedSource<String>>,

    /// Span in bytes.
    pub span: Range<usize>,
}

impl Loc {
    /// Create a [`Loc`] from a source location and a spanned
    /// grammar node.
    pub fn new<T>(src: Arc<NamedSource<String>>, spanned: &Spanned<T>) -> Self {
        Self {
            src,
            span: spanned.span.0..spanned.span.1,
        }
    }
}

impl fmt::Debug for Loc {
    // Print as "file:begin:end" to reduce clutter in dumps.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.src.name())?;
        f.write_char(':')?;
        self.span.start.fmt(f)?;
        f.write_char(':')?;
        self.span.end.fmt(f)
    }
}
