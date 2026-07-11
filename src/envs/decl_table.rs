use super::DeclIdx;

pub struct DeclTable<T> {
    decls: Vec<T>,
}

impl<T> DeclTable<T> {
    /// Create a new [`DeclTable`].
    pub fn new() -> Self {
        Self { decls: vec![] }
    }

    /// Insert an entry and return its ID.
    pub fn insert(&mut self, value: T) -> DeclIdx<T> {
        let index = self.decls.len();
        self.decls.push(value);
        DeclIdx::new(index)
    }

    /// Our underlying items.
    pub fn items(&self) -> impl Iterator<Item = (DeclIdx<T>, &T)> {
        self.decls
            .iter()
            .enumerate()
            .map(|(idx, item)| (DeclIdx::new(idx), item))
    }
}
