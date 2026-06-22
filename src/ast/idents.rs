use std::{
    fmt,
    hash::{Hash, Hasher},
    sync::Arc,
};

use miette::{NamedSource, SourceSpan};
use tree_sitter_wasl_types::nodes;
use type_sitter::Node as _;

use super::node_source;
use crate::locs::Loc;

#[derive(Clone, Debug)]
pub struct Ident {
    pub loc: Loc,
    text: String,
}

impl Ident {
    pub fn from_grammar(src: Arc<NamedSource<String>>, ident: nodes::Ident<'_>) -> Self {
        let loc = Loc::new(src.clone(), ident.raw());
        Ident {
            loc,
            text: node_source(&src, ident.raw()).to_owned(),
        }
    }

    #[cfg(test)]
    pub fn new_for_test(name: &str) -> Ident {
        Ident {
            loc: Loc::new_for_test(),
            text: name.to_owned(),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn src(&self) -> Arc<NamedSource<String>> {
        self.loc.src.clone()
    }

    pub fn src_span(&self) -> SourceSpan {
        SourceSpan::from(self.loc.span.clone())
    }
}

impl PartialEq for Ident {
    fn eq(&self, other: &Self) -> bool {
        self.text.eq(&other.text)
    }
}

impl Eq for Ident {}

impl Hash for Ident {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.text.hash(state);
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}
