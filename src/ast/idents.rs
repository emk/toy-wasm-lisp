use std::hash::{Hash, Hasher};

use super::grammar::Ident;

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
