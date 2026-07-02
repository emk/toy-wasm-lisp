use std::{
    fmt,
    hash::{Hash, Hasher},
};

use miette::{Result, SourceSpan};
use tree_sitter_wasl_types::nodes;
use type_sitter::Node as _;

use crate::{
    ast::FromGrammar,
    errors::ParseError,
    locs::{Loc, Source},
};

#[derive(Clone, Debug)]
pub struct Ident {
    pub loc: Loc,
    text: String,
}

impl Ident {
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

    pub fn src_span(&self) -> SourceSpan {
        self.loc.src_span()
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

impl FromGrammar for Ident {
    type Input<'a> = nodes::Ident<'a>;

    fn from_grammar(src: &Source, ident: nodes::Ident<'_>) -> Result<Self, ParseError> {
        let loc = src.loc_for(ident.raw());
        Ok(Ident {
            loc,
            text: src.node_text(ident.raw()).to_owned(),
        })
    }
}
