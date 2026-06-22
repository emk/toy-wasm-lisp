//! Source location, for error messages.

use std::{
    fmt::{self, Write},
    ops::Range,
    sync::Arc,
};

use miette::NamedSource;
use type_sitter::raw;

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
    /// Create a [`Loc`] from a source location and a type-sitter node.
    pub fn new(src: Arc<NamedSource<String>>, node: &raw::Node<'_>) -> Self {
        Self {
            src,
            span: node.start_byte()..node.end_byte(),
        }
    }

    /// Create a [`Loc`] pointing just _after_ `prev_node`. Used for optional elements.
    pub fn after(src: Arc<NamedSource<String>>, prev_node: &raw::Node<'_>) -> Self {
        Self {
            src,
            span: prev_node.end_byte()..prev_node.end_byte(),
        }
    }

    /// Create a [`Loc`] for use in tests, with an empty span pointing
    /// at an empty source file.
    #[cfg(test)]
    pub fn new_for_test() -> Loc {
        Loc {
            src: Arc::new(NamedSource::new("<test>", "".to_string())),
            span: 0..0,
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
