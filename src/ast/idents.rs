use std::{
    fmt,
    hash::{Hash, Hasher},
    sync::Arc,
};

use miette::NamedSource;

use crate::{locs::Loc, parser::grammar};

#[derive(Clone, Debug)]
pub struct Ident {
    pub loc: Loc,
    text: String,
}

impl Ident {
    pub fn from_grammar(src: Arc<NamedSource<String>>, ident: &grammar::Ident) -> Self {
        Ident {
            loc: Loc::new(src, &ident.text),
            text: ident.text.value.clone(),
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
