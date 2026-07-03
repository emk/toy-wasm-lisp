//! Source location, for error messages.

use std::fmt::{self};

use miette::{NamedSource, SourceCode, SourceSpan};
use type_sitter::raw;

/// A named source code file stored in [`Sources`].
#[derive(Debug)]
pub struct Source {
    offset: usize,
    source: NamedSource<String>,
}

impl Source {
    /// Construct a [`Loc`] for a tree-sitter node.
    pub fn loc_for(&self, node: &raw::Node<'_>) -> Loc {
        let start = self.offset.strict_add(node.start_byte());
        let end = self.offset.strict_add(node.end_byte());
        let span = SourceSpan::from(start..end);
        debug_assert_eq!(span.len(), node.end_byte().strict_sub(node.start_byte()));
        debug_assert!(
            self.offset <= span.offset()
                && span.offset() <= self.offset.strict_add(self.source.inner().len())
        );
        debug_assert!(
            span.offset().strict_add(span.len())
                <= self.offset.strict_add(self.source.inner().len())
        );
        Loc::new(span)
    }

    /// Construct a [`Loc`] immediately after a tree-sitter node.
    pub fn loc_after(&self, node: &raw::Node<'_>) -> Loc {
        let end = self.offset.strict_add(node.end_byte());
        let span = SourceSpan::from(end..end);
        Loc::new(span)
    }

    /// Get the text of this source.
    pub fn text(&self) -> &str {
        self.source.inner()
    }

    /// Expect UTF-8 text in parsed data. We require UTF-8 input, so this
    /// should always succeed.
    pub fn node_text<'src>(&'src self, node: &raw::Node<'_>) -> &'src str {
        node.utf8_text(self.text().as_bytes())
            .expect("should always be UTF-8")
    }

    /// Does this source file contain the specified offset span?
    pub fn contains_offset_span(&self, span: &SourceSpan) -> bool {
        let source_begin = self.offset;
        let source_end = self.offset.strict_add(self.source.inner().len());
        let source_span = source_begin..source_end;
        source_span.contains(&span.offset())
            && source_span.contains(&span.offset().strict_add(span.len()))
    }
}

/// Interface to multiple source files.
///
/// [`miette`] supports source ranges in a single virtual "source file". In
/// order to support multiple source files, we pretend to "append" the files
/// into a single file one after another. But in reality, we keep the file
/// boundarieds and the names.
///
#[derive(Debug, Default)]
pub struct Sources {
    /// The offset we will use for the next file added.
    next_offset: usize,
    /// All the files that have been added, sorted by ascending starting offset
    /// so that we can do binary search later.
    sources: Vec<Source>,
}

impl Sources {
    /// Add a new source file. Returns the `&Source` wrapper.
    pub fn add_source(&mut self, filename: &str, src: String) -> &Source {
        let offset = self.next_offset;
        self.next_offset += src.len();
        self.sources.push(Source {
            offset,
            source: NamedSource::new(filename, src),
        });
        self.sources.last().expect("just pushed")
    }
}

impl SourceCode for Sources {
    /// This is the trick that maps our offsets from the "combined" virtual file
    /// back into the appropriate individual file.
    fn read_span<'a>(
        &'a self,
        span: &SourceSpan,
        context_lines_before: usize,
        context_lines_after: usize,
    ) -> Result<Box<dyn miette::SpanContents<'a> + 'a>, miette::MietteError> {
        // Use binary search to locate our starting offset in the correct
        // underlying file.
        let source_idx_result = self
            .sources
            .binary_search_by_key(&span.offset(), |s| s.offset);
        let source_idx = match source_idx_result {
            // We are right at the beginning of a source file.
            Ok(idx) => idx,
            // We are in-between the beginning points of two source files. Take
            // the previous one.
            Err(idx) => idx.strict_sub(1),
        };
        let source = &self.sources[source_idx];
        assert!(
            source.contains_offset_span(span),
            "{span:?} should fall in {source:?}"
        );

        // Shift our `span` back into the single-file space, and look it up
        // normally. Note that this will do the right thing with file
        // boundaries.
        let shifted_offset = span.offset().strict_sub(source.offset);
        let shifted_span = SourceSpan::from((shifted_offset, span.len()));
        source
            .source
            .read_span(&shifted_span, context_lines_before, context_lines_after)
    }
}

/// The location of an AST node in the source. This is normally used in
/// [`miette`] errors.
#[derive(Clone)]
pub struct Loc {
    /// Span in bytes.
    pub span: SourceSpan,
}

impl Loc {
    /// Create a [`Loc`] from a source location and a type-sitter node.
    pub fn new(span: SourceSpan) -> Self {
        Self { span }
    }

    /// Get the [`miette::SourceSpan`] for this location.
    pub fn src_span(&self) -> SourceSpan {
        self.span
    }

    /// Create a [`Loc`] for use in tests, with an empty span pointing
    /// at an empty source file.
    #[cfg(test)]
    pub fn new_for_test() -> Loc {
        Loc {
            span: SourceSpan::from(0..0),
        }
    }
}

impl fmt::Debug for Loc {
    // Print as "file:begin:end" to reduce clutter in dumps.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let offset = self.span.offset();
        (offset..offset.strict_add(self.span.len())).fmt(f)
    }
}
