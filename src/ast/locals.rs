//! Local variables.

use super::{Ident, Type};

/// Local variable declaration.
#[derive(Clone, Debug)]
pub struct Local {
    name: Ident,
    ty: Type,
}

impl Local {
    #[cfg(test)]
    pub fn new_i32_for_test(name: &str) -> Self {
        Self {
            name: Ident::new_for_test(name),
            ty: Type::new_i32_for_test(),
        }
    }
}
