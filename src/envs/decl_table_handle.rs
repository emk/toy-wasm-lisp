use super::DeclIdx;

pub struct DeclTable<T> {
    decls: Vec<T>,
}

impl<T> DeclTable<T> {
    /// Create a new [`DeclTableHandle`].
    pub fn new() -> Self {
        Self { decls: vec![] }
    }

    /// Insert an entry and return its ID.
    pub fn insert(&mut self, value: T) -> DeclIdx<T> {
        let index = self.decls.len();
        self.decls.push(value);
        DeclIdx::new(index)
    }

    /// Borrow our underlying storage (for iteration, etc).
    #[cfg(test)]
    pub fn decls(&self) -> &[T] {
        &self.decls
    }
}
