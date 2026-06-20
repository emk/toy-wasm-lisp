use miette::Result;
use wasm_encoder::{
    CodeSection, ExportKind, ExportSection, Function, FunctionSection, Module, TypeSection,
};

pub use super::{DeclIdx, IdentMap, IndexedType, TypeIndexer};
use crate::ast::{Func, Ident};

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

    /// Get our function map, for looking up callsites.
    ///
    /// TODO: Merge into a more complete env mechanism.
    pub fn func_map(&self) -> &IdentMap<'static, Func> {
        &self.func_map
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
        self.funcs_sec.function(type_idx.try_as_u32()?);
        let func_idx = self.func_map.insert(name.clone(), func.clone())?;
        if func.should_export() {
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
        self.exports_sec
            .export(name.as_str(), kind, idx.try_as_u32()?);
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
