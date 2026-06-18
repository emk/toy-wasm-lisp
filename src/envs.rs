//! Keeping track of names and indices.
//!
//! This is a bit unusual, because we want to support nested namespaces,
//! but at the same time, we also want to support

use std::{
    cell::{Ref, RefCell},
    collections::HashMap,
    rc::Rc,
};

use miette::Result;

use crate::{
    ast::grammar::Ident,
    errors::{DuplicateDeclarationError, UnknownIdentifierError},
};

pub struct DeclTableHandle<T> {
    decls: Rc<RefCell<Vec<T>>>,
}

impl<T> DeclTableHandle<T> {
    /// Create a new [`DeclTableHandle`].
    pub fn new() -> Self {
        Self {
            decls: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Insert an entry and return its ID.
    fn insert(&mut self, value: T) -> usize {
        let mut decls = self.decls.borrow_mut();
        let index = decls.len();
        decls.push(value);
        index
    }

    /// Get an entry by its ID.
    fn get(&self, index: usize) -> Option<Ref<'_, T>> {
        // Some moderately obscure trickery to take a borrowed RefCell, and map
        // a function over it, maintaining the original borrow with the derived
        // value.
        Ref::filter_map(self.decls.borrow(), |decls| decls.get(index)).ok()
    }

    /// Borrow our underlying storage (for iteration, etc).
    pub fn decls(&self) -> Ref<'_, Vec<T>> {
        self.decls.borrow()
    }
}

impl<T> Clone for DeclTableHandle<T> {
    fn clone(&self) -> Self {
        Self {
            decls: self.decls.clone(),
        }
    }
}

pub struct NameMap<'parent, T> {
    decl_table: DeclTableHandle<T>,
    parent: Option<&'parent NameMap<'parent, T>>,
    idents: HashMap<Ident, usize>,
}

/// Create a new [`NameEnv`].
impl<T> NameMap<'static, T> {
    pub fn new() -> Self {
        Self {
            decl_table: DeclTableHandle::new(),
            parent: None,
            idents: HashMap::new(),
        }
    }
}

impl<'parent, T> NameMap<'parent, T> {
    /// Create a child [`NameEnv`] which shares the same [`DeclTableHandle`].
    /// This is needed because WASM frequently requires declaring all locals (or whatever)
    /// at the top of a function, sharing a single set of indices.
    pub fn child<'new_parent: 'parent>(&'new_parent self) -> NameMap<'new_parent, T> {
        Self {
            decl_table: self.decl_table.clone(),
            parent: Some(self),
            idents: HashMap::new(),
        }
    }

    /// Get the underlying [`DeclTableHandle`].
    pub fn decl_table(&self) -> &DeclTableHandle<T> {
        &self.decl_table
    }

    /// Insert `ident`, returning an error if it already exists.
    pub fn insert(&mut self, ident: Ident, value: T) -> Result<(), DuplicateDeclarationError> {
        if let Some((original_ident, _v)) = self.idents.get_key_value(&ident) {
            return Err(DuplicateDeclarationError::new(
                ident,
                original_ident.to_owned(),
            ));
        }
        let id = self.decl_table.insert(value);
        self.idents.insert(ident, id);
        Ok(())
    }

    /// Look up `ident`, returning `None` if it does not exist.
    pub fn try_get<'a>(&'a self, ident: &Ident) -> Option<Ref<'a, T>> {
        if let Some(index) = self.idents.get(ident) {
            self.decl_table.get(*index)
        } else if let Some(parent) = self.parent {
            parent.try_get(ident)
        } else {
            None
        }
    }

    /// Look up `ident`, returning an error if it does not exist.
    pub fn get<'a>(&'a self, ident: &Ident) -> Result<Ref<'a, T>, UnknownIdentifierError> {
        self.try_get(ident)
            .ok_or_else(|| UnknownIdentifierError::new(ident.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ident(name: &str) -> Ident {
        Ident::new_for_test(name)
    }

    #[test]
    fn insert_and_get() {
        let mut map = NameMap::new();

        // Insert and get.
        map.insert(ident("foo"), 1).unwrap();
        assert_eq!(*map.get(&ident("foo")).unwrap(), 1);

        // Duplicate insert.
        assert!(map.insert(ident("foo"), 5).is_err());

        // Insert a new key.
        map.insert(ident("bar"), 2).unwrap();

        // Create child.
        let mut child = map.child();
        assert_eq!(*child.get(&ident("foo")).unwrap(), 1);

        // Insert into child.
        child.insert(ident("foo"), 3).unwrap();
        assert_eq!(*child.get(&ident("foo")).unwrap(), 3);

        // Verify parent is unaffected.
        assert_eq!(*map.get(&ident("foo")).unwrap(), 1);

        // Make sure other key unshadowed in child.
        assert_eq!(*child.get(&ident("bar")).unwrap(), 2);

        // Verify we have the right entries.
        let decls = map.decl_table().decls();
        let collected_decls = decls.iter().cloned().collect::<Vec<_>>();
        assert_eq!(collected_decls.len(), 3);
        assert_eq!(collected_decls[0], 1);
        assert_eq!(collected_decls[1], 2);
        assert_eq!(collected_decls[2], 3);
    }
}
