use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use super::DeclIdx;

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
    pub fn insert(&mut self, value: T) -> DeclIdx<T> {
        let mut decls = self.decls.borrow_mut();
        let index = decls.len();
        decls.push(value);
        DeclIdx::new(index)
    }

    /// Get an entry by its ID.
    pub fn get(&self, index: DeclIdx<T>) -> Option<Ref<'_, T>> {
        // Some moderately obscure trickery to take a borrowed RefCell, and map
        // a function over it, maintaining the original borrow with the derived
        // value.
        Ref::filter_map(self.decls.borrow(), |decls| decls.get(index.as_usize())).ok()
    }

    /// Borrow our underlying storage (for iteration, etc).
    #[cfg(test)]
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
