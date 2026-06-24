// Parser based on [`rust_sitter`].

use std::sync::Arc;

use miette::{NamedSource, Result, SourceSpan};
use tracing::{debug, trace};
use tree_sitter_wasl_types::nodes;
use type_sitter::{Node as _, Parser, raw};

use crate::{
    ast::Mod,
    errors::{ParseError, ParseErrors},
};

/// If we collect this many parse errors, stop reporting more.
const MAX_PARSE_ERRORS: usize = 3;

/// Parse `src`, using `
pub fn parse(filename: &str, src: &str) -> Result<Mod, ParseErrors> {
    debug!(%filename, %src, "Parsing");
    let src = Arc::new(NamedSource::new(filename, src.to_owned()));

    let mut parser = Parser::<nodes::SourceFile<'static>>::new(&tree_sitter_wasl::LANGUAGE.into())
        .expect("tree-sitter version mistmatch");
    let parsed = parser
        .parse(src.as_ref().inner(), None)
        .expect("language not assigned to parser");

    let source_file = parsed.root_node().expect("expected source_file node");
    if let Some(errs) = collect_errors(src.clone(), source_file.raw()) {
        return Err(errs);
    }
    let ast = Mod::from_grammar(src, source_file);
    Ok(ast)
}

/// Recursively walk the node tree, finding all errors.
fn collect_errors(src: Arc<NamedSource<String>>, root: &raw::Node<'_>) -> Option<ParseErrors> {
    trace!(%root, "looking for errors");
    let mut out = vec![];
    let mut cursor = root.walk();
    let mut existing_error_count_stack = vec![];
    'walk: loop {
        let node = cursor.node();

        // We keep track of how many errors we've seen every time we enter a node,
        // because this allows us to selectively emit errors for `has_error`
        existing_error_count_stack.push(out.len());

        // See if we have any kind of useful error here.
        if node.is_error() {
            out.push(ParseError::new(source_span(node), "unexpected input"));
        } else if node.is_missing() {
            out.push(ParseError::new(source_span(node), "missing token"));
        }
        if out.len() >= MAX_PARSE_ERRORS {
            break 'walk;
        }

        // We don't have a good error at this level, so walk deeper.
        if node.has_error()
            && out.len()
                == *existing_error_count_stack
                    .last()
                    .expect("should always have stack entry here")
            && cursor.goto_first_child()
        {
            continue 'walk;
        }

        // Figure out how to advance.
        loop {
            // Try to progress at the same level.
            if cursor.goto_next_sibling() {
                continue 'walk;
            }

            // We're finishing this node (one way or another). So if we haven't
            // output a decent error since we entered this level, add
            // _something_.
            if node.has_error()
                && out.len()
                    == existing_error_count_stack
                        .pop()
                        .expect("should always have stack entry here")
            {
                out.push(ParseError::new(source_span(node), "syntax error"));
            }

            // Try to go up.
            if cursor.goto_parent() {
                continue;
            }

            // No sibling or parent; we're done.
            break 'walk;
        }
    }

    if out.is_empty() {
        return None;
    }
    Some(ParseErrors::new(src, out))
}

/// Get the span for a node.
fn source_span(node: raw::Node<'_>) -> SourceSpan {
    SourceSpan::from((node.start_byte(), node.end_byte()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_parses() {
        parse("<test>", "func f() -> (i32) { 1 + 2 * 3 }").unwrap();
    }
}
