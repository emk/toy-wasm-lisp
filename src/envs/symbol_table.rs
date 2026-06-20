//! Symbol tables for looking up names.

use std::{collections::HashMap, fmt};

use super::DeclIdx;
use crate::{
    ast::{Func, Ident, Local},
    errors::SymbolTableError,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SymbolCategory {
    /// A callable function.
    Func,
    /// A variable (global, param, local, etc).
    Var,
}

impl fmt::Display for SymbolCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SymbolCategory::Func => write!(f, "a function"),
            SymbolCategory::Var => write!(f, "a variable"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Symbol {
    /// Module-level function declaration.
    Func { idx: DeclIdx<Func>, func: Box<Func> },
    /// Local variable declaration (including function parameters).
    Local {
        idx: DeclIdx<Local>,
        local: Box<Local>,
    },
}

impl Symbol {
    /// Get the category of this symbol.
    pub fn category(&self) -> SymbolCategory {
        match self {
            Symbol::Func { .. } => SymbolCategory::Func,
            Symbol::Local { .. } => SymbolCategory::Var,
        }
    }
}

/// Table for looking up symbols/names used in source code.
/// May be chained in a hierachy.
pub struct SymbolTable<'parent> {
    /// Parent [`SymbolTable`], if any.
    parent: Option<&'parent SymbolTable<'parent>>,
    /// Our own local symbols.
    map: HashMap<Ident, Symbol>,
}

impl SymbolTable<'static> {
    pub fn new() -> Self {
        Self {
            parent: None,
            map: HashMap::new(),
        }
    }
}

impl<'parent> SymbolTable<'parent> {
    /// Create a child [`SymbolTable`] which may shadow symbols in the parent.
    pub fn child<'new_parent: 'parent>(&'new_parent self) -> SymbolTable<'new_parent> {
        Self {
            parent: Some(self),
            map: HashMap::new(),
        }
    }

    /// Insert `ident` with value `sym`, returning an error if it already
    /// exists.
    pub fn insert(&mut self, ident: Ident, sym: Symbol) -> Result<(), SymbolTableError> {
        if let Some((original_ident, _v)) = self.map.get_key_value(&ident) {
            return Err(SymbolTableError::duplicate_declaration(
                ident,
                original_ident.to_owned(),
            ));
        }
        self.map.insert(ident, sym);
        Ok(())
    }

    /// Look up `ident`, returning `None` if it does not exist.
    pub fn try_get<'a>(&'a self, ident: &Ident) -> Option<&'a Symbol> {
        self.map
            .get(ident)
            .or_else(|| self.parent.and_then(|p| p.try_get(ident)))
    }

    /// Look up `ident`, returning an error if it does not exist.
    pub fn get<'a>(&'a self, ident: &Ident) -> Result<&'a Symbol, SymbolTableError> {
        self.try_get(ident)
            .ok_or_else(|| SymbolTableError::unknown_identifier(ident.to_owned()))
    }

    /// Get a function value.
    pub fn get_func<'a>(
        &'a self,
        ident: &Ident,
    ) -> Result<(DeclIdx<Func>, &'a Func), SymbolTableError> {
        match self.get(ident)? {
            Symbol::Func { idx, func } => Ok((*idx, func)),
            other => Err(SymbolTableError::wrong_symbol_category(
                ident.to_owned(),
                SymbolCategory::Func,
                other.category(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ident(name: &str) -> Ident {
        Ident::new_for_test(name)
    }

    fn local(idx: usize) -> Symbol {
        let idx = DeclIdx::new(idx);
        let local = Local::new_i32_for_test("x");
        Symbol::Local {
            idx,
            local: Box::new(local),
        }
    }

    fn idx(sym: &Symbol) -> usize {
        match sym {
            Symbol::Func { idx, .. } => idx.as_usize(),
            Symbol::Local { idx, .. } => idx.as_usize(),
        }
    }

    #[test]
    fn insert_and_get() {
        let mut map = SymbolTable::new();

        // Insert and get.
        map.insert(ident("foo"), local(1)).unwrap();
        assert_eq!(idx(map.get(&ident("foo")).unwrap()), 1);

        // Duplicate insert.
        assert!(map.insert(ident("foo"), local(5)).is_err());

        // Insert a new key.
        map.insert(ident("bar"), local(2)).unwrap();

        // Create child.
        let mut child = map.child();
        assert_eq!(idx(child.get(&ident("foo")).unwrap()), 1);

        // Insert into child.
        child.insert(ident("foo"), local(3)).unwrap();
        assert_eq!(idx(child.get(&ident("foo")).unwrap()), 3);

        // Verify parent is unaffected.
        assert_eq!(idx(map.get(&ident("foo")).unwrap()), 1);

        // Make sure other key unshadowed in child.
        assert_eq!(idx(child.get(&ident("bar")).unwrap()), 2);
    }
}
