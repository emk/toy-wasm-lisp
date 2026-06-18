//! Keeping track of names and indices.
//!
//! This is a bit unusual, because we want to support nested namespaces,
//! but at the same time, we also want to support

use std::{
    cell::{Ref, RefCell},
    collections::HashMap,
    fmt,
    marker::PhantomData,
    rc::Rc,
};

use miette::Result;

use crate::{
    ast::grammar::Ident,
    errors::{DuplicateDeclarationError, UnknownIdentifierError},
};

pub struct DeclIdx<T> {
    idx: usize,
    _phantom: PhantomData<T>,
}

impl<T> DeclIdx<T> {
    /// Private method for creating a new ID.
    fn new(idx: usize) -> Self {
        Self {
            idx,
            _phantom: PhantomData,
        }
    }
}

impl<T> Clone for DeclIdx<T> {
    fn clone(&self) -> Self {
        Self {
            idx: self.idx,
            _phantom: self._phantom,
        }
    }
}

impl<T> Copy for DeclIdx<T> {}

impl<T> fmt::Debug for DeclIdx<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.idx.fmt(f)
    }
}

impl<T> PartialEq for DeclIdx<T> {
    fn eq(&self, other: &Self) -> bool {
        self.idx == other.idx
    }
}

impl<T> Eq for DeclIdx<T> {}

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
    fn insert(&mut self, value: T) -> DeclIdx<T> {
        let mut decls = self.decls.borrow_mut();
        let index = decls.len();
        decls.push(value);
        DeclIdx::new(index)
    }

    /// Get an entry by its ID.
    fn get(&self, index: DeclIdx<T>) -> Option<Ref<'_, T>> {
        // Some moderately obscure trickery to take a borrowed RefCell, and map
        // a function over it, maintaining the original borrow with the derived
        // value.
        Ref::filter_map(self.decls.borrow(), |decls| decls.get(index.idx)).ok()
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

/// A map from [`Ident`] to values of type `T`. Supports nested child scopes.
///
/// An `IdentMap` and all its children share a single underlying set of indices.
pub struct IdentMap<'parent, T> {
    decl_table: DeclTableHandle<T>,
    parent: Option<&'parent IdentMap<'parent, T>>,
    idents: HashMap<Ident, DeclIdx<T>>,
}

impl<T> IdentMap<'static, T> {
    /// Create a new [`NameEnv`].
    pub fn new() -> Self {
        Self {
            decl_table: DeclTableHandle::new(),
            parent: None,
            idents: HashMap::new(),
        }
    }
}

impl<'parent, T> IdentMap<'parent, T> {
    /// Create a child [`NameEnv`] which shares the same [`DeclTableHandle`].
    /// This is needed because WASM frequently requires declaring all locals (or whatever)
    /// at the top of a function, sharing a single set of indices.
    pub fn child<'new_parent: 'parent>(&'new_parent self) -> IdentMap<'new_parent, T> {
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
    pub fn try_get<'a>(&'a self, ident: &Ident) -> Option<(DeclIdx<T>, Ref<'a, T>)> {
        if let Some(index) = self.idents.get(ident) {
            Some((
                *index,
                self.decl_table
                    .get(index.clone())
                    .expect("environment missing idx"),
            ))
        } else if let Some(parent) = self.parent {
            parent.try_get(ident)
        } else {
            None
        }
    }

    /// Look up `ident`, returning an error if it does not exist.
    pub fn get<'a>(
        &'a self,
        ident: &Ident,
    ) -> Result<(DeclIdx<T>, Ref<'a, T>), UnknownIdentifierError> {
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
        let mut map = IdentMap::new();

        // Insert and get.
        map.insert(ident("foo"), 1).unwrap();
        assert_eq!(map.get(&ident("foo")).unwrap().0, DeclIdx::new(0));
        assert_eq!(*map.get(&ident("foo")).unwrap().1, 1);

        // Duplicate insert.
        assert!(map.insert(ident("foo"), 5).is_err());

        // Insert a new key.
        map.insert(ident("bar"), 2).unwrap();

        // Create child.
        let mut child = map.child();
        assert_eq!(*child.get(&ident("foo")).unwrap().1, 1);

        // Insert into child.
        child.insert(ident("foo"), 3).unwrap();
        assert_eq!(*child.get(&ident("foo")).unwrap().1, 3);

        // Verify parent is unaffected.
        assert_eq!(*map.get(&ident("foo")).unwrap().1, 1);

        // Make sure other key unshadowed in child.
        assert_eq!(*child.get(&ident("bar")).unwrap().1, 2);

        // Verify we have the right entries.
        let decls = map.decl_table().decls();
        let collected_decls = decls.iter().cloned().collect::<Vec<_>>();
        assert_eq!(collected_decls.len(), 3);
        assert_eq!(collected_decls[0], 1);
        assert_eq!(collected_decls[1], 2);
        assert_eq!(collected_decls[2], 3);
    }
}
