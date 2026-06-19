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

use miette::{Result, miette};
use wasm_encoder::{
    CodeSection, ExportKind, ExportSection, FuncType, Function, FunctionSection, Module,
    TypeSection,
};

use crate::{
    ast::grammar::{Func, Ident},
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

    // Convert to a `u32`, failing if it does not fit.
    pub fn as_u32(&self) -> Result<u32> {
        u32::try_from(self.idx).map_err(|_| miette!("index overflow: {}", self.idx))
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

/// Module-level environment.
///
/// This includes both [`wasm_encoder`] "section" types, and our own
/// machinery for keeping track of names and generating indices.
pub struct ModuleEnv {
    types_sec: TypeSection,
    funcs_sec: FunctionSection,
    exports_sec: ExportSection,
    codes_sec: CodeSection,

    type_indexer: TypeIndexer,
    func_map: IdentMap<'static, Func>,
}

impl ModuleEnv {
    /// Create a new [`ModuleEnv`] with default values.
    pub fn new() -> Self {
        Self {
            types_sec: TypeSection::new(),
            funcs_sec: FunctionSection::new(),
            exports_sec: ExportSection::new(),
            codes_sec: CodeSection::new(),
            type_indexer: TypeIndexer::new(),
            func_map: IdentMap::new(),
        }
    }

    /// Insert a type.
    pub fn find_or_insert_type(&mut self, ty: IndexedType) -> DeclIdx<IndexedType> {
        let (is_new, idx) = self.type_indexer.find_or_insert(ty.clone());
        if is_new {
            match ty {
                IndexedType::Func(func_type) => {
                    self.types_sec.ty().func_type(&func_type);
                }
            }
        }
        idx
    }

    /// Insert a function declaration.
    pub fn insert_function(&mut self, name: Ident, func: Func) -> Result<()> {
        let type_idx = self.find_or_insert_type(IndexedType::Func(func.func_type()?));
        self.funcs_sec.function(type_idx.as_u32()?);
        let func_idx = self.func_map.insert(name.clone(), func.clone())?;
        if func.is_exported() {
            self.export(&name, ExportKind::Func, func_idx)?;
        }
        Ok(())
    }

    /// Insert a function implementation. This must be called in the same order as
    /// [`Self::insert_function`].
    ///
    /// TODO: Add some enforcement to make sure we get called the right number of times in
    /// the right order.
    pub fn insert_code(&mut self, code: &Function) {
        self.codes_sec.function(code);
    }

    /// Internal helper for exporting. Make sure `kind` and `T` match!
    fn export<T>(&mut self, name: &Ident, kind: ExportKind, idx: DeclIdx<T>) -> Result<()> {
        self.exports_sec.export(&name.text, kind, idx.as_u32()?);
        Ok(())
    }

    /// Build a WASM module.
    pub fn build_module(&self) -> Module {
        let mut module = Module::new();
        module.section(&self.types_sec);
        module.section(&self.funcs_sec);
        module.section(&self.exports_sec);
        module.section(&self.codes_sec);
        module
    }
}

#[cfg(test)]
mod tests {
    use wasm_encoder::ValType;

    use super::*;

    fn ident(name: &str) -> Ident {
        Ident::new_for_test(name)
    }

    #[test]
    fn ident_map_insert_and_get() {
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

    #[test]
    fn type_index_insert() {
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
