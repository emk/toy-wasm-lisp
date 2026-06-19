use std::{cell::Ref, collections::HashMap};

use miette::Result;

use super::{DeclIdx, DeclTableHandle};
use crate::{
    ast::Ident,
    errors::{DuplicateDeclarationError, UnknownIdentifierError},
};

/// A map from [`Ident`] to values of type `T`. Supports nested child scopes.
///
/// An `IdentMap` and all its children share a single underlying set of indices.
pub struct IdentMap<'parent, T> {
    decl_table: DeclTableHandle<T>,
    parent: Option<&'parent IdentMap<'parent, T>>,
    idents: HashMap<Ident, DeclIdx<T>>,
}

impl<T> IdentMap<'static, T> {
    /// Create a new [`IdentMap`].
    pub fn new() -> Self {
        Self {
            decl_table: DeclTableHandle::new(),
            parent: None,
            idents: HashMap::new(),
        }
    }
}

impl<'parent, T> IdentMap<'parent, T> {
    /// Create a child [`IdentMap`] which shares the same [`DeclTableHandle`].
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
    #[cfg(test)]
    pub fn decl_table(&self) -> &DeclTableHandle<T> {
        &self.decl_table
    }

    /// Insert `ident`, returning an error if it already exists.
    pub fn insert(
        &mut self,
        ident: Ident,
        value: T,
    ) -> Result<DeclIdx<T>, DuplicateDeclarationError> {
        if let Some((original_ident, _v)) = self.idents.get_key_value(&ident) {
            return Err(DuplicateDeclarationError::new(
                ident,
                original_ident.to_owned(),
            ));
        }
        let id = self.decl_table.insert(value);
        self.idents.insert(ident, id);
        Ok(id)
    }

    /// Look up `ident`, returning `None` if it does not exist.
    pub fn try_get<'a>(&'a self, ident: &Ident) -> Option<(DeclIdx<T>, Ref<'a, T>)> {
        if let Some(index) = self.idents.get(ident) {
            Some((
                *index,
                self.decl_table
                    .get(index.to_owned())
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
