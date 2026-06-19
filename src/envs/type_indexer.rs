use std::collections::HashMap;

use wasm_encoder::FuncType;

use super::DeclIdx;

/// Type used in a `(type ...)` declaration.
//
/// Currently limited to function types, though eventually we will need to
/// create a custom `enum` for all the types supported by
/// [`wasm_encoder::CoreTypeEncoder`].
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum IndexedType {
    Func(FuncType),
}

/// Keeps track of known types for `(type ...)` declarations.
#[derive(Debug, Default)]
pub struct TypeIndexer {
    next_type_idx: usize,
    type_map: HashMap<IndexedType, DeclIdx<IndexedType>>,
}

impl TypeIndexer {
    /// Creates a new [`TypeIndexer`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Helper function to get the next ID in our sequence.
    fn next_id(&mut self) -> DeclIdx<IndexedType> {
        let idx = DeclIdx::new(self.next_type_idx);
        self.next_type_idx += 1;
        idx
    }

    /// Insert (or lookup) a type, returning a boolean indicating whether the type was inserted,
    /// and the index.
    pub fn find_or_insert(&mut self, ty: IndexedType) -> (bool, DeclIdx<IndexedType>) {
        if let Some(idx) = self.type_map.get(&ty) {
            (false, *idx)
        } else {
            let idx = self.next_id();
            self.type_map.insert(ty, idx);
            (true, idx)
        }
    }
}

#[cfg(test)]
mod tests {
    use wasm_encoder::ValType;

    use super::*;

    #[test]
    fn find_or_insert() {
        let mut type_index = TypeIndexer::new();
        let add_type = FuncType::new(vec![ValType::I32, ValType::I32], vec![ValType::I32]);
        let abs_type = FuncType::new(vec![ValType::I32], vec![ValType::I32]);

        // Different types get different IDs.
        assert_eq!(
            type_index.find_or_insert(IndexedType::Func(add_type.clone())),
            (true, DeclIdx::new(0))
        );
        assert_eq!(
            type_index.find_or_insert(IndexedType::Func(abs_type.clone())),
            (true, DeclIdx::new(1))
        );

        // Multiple insertions return the same ID.
        assert_eq!(
            type_index.find_or_insert(IndexedType::Func(add_type)),
            (false, DeclIdx::new(0))
        );
    }
}
