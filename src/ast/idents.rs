use std::{
    fmt,
    hash::{Hash, Hasher},
};

// TODO: This type is still shared with the parser. We may
// make it a standalone type at some point.
use crate::parser::grammar::Ident;

impl Ident {
    #[cfg(test)]
    pub fn new_for_test(name: &str) -> Ident {
        use rust_sitter::Spanned;

        Ident {
            text: Spanned {
                value: name.to_owned(),
                span: (0, 0),
            },
        }
    }
}

impl PartialEq for Ident {
    fn eq(&self, other: &Self) -> bool {
        self.text.value.eq(&other.text.value)
    }
}

impl Eq for Ident {}

impl Hash for Ident {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.text.value.hash(state);
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text.value)
    }
}
