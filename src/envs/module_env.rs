use miette::Result;
use wasm_encoder::{
    CodeSection, EntityType, ExportKind, ExportSection, Function, FunctionSection, ImportSection,
    Module, TypeSection,
};

pub use super::{DeclIdx, IndexedType, Symbol, SymbolTable, TypeIndexer};
use crate::{
    ast::{Func, FuncSig, Ident, Import},
    envs::DeclTable,
};

/// Module-level environment.
///
/// This includes both [`wasm_encoder`] "section" types, and our own
/// machinery for keeping track of names and generating indices.
pub struct ModuleEnv {
    types_sec: TypeSection,
    imports_sec: ImportSection,
    funcs_sec: FunctionSection,
    exports_sec: ExportSection,
    codes_sec: CodeSection,

    type_indexer: TypeIndexer,
    func_decls: DeclTable<FuncSig>,
    symbol_table: SymbolTable<'static>,
}

impl ModuleEnv {
    /// Create a new [`ModuleEnv`] with default values.
    pub fn new() -> Self {
        Self {
            types_sec: TypeSection::new(),
            imports_sec: ImportSection::new(),
            funcs_sec: FunctionSection::new(),
            exports_sec: ExportSection::new(),
            codes_sec: CodeSection::new(),
            type_indexer: TypeIndexer::new(),
            func_decls: DeclTable::new(),
            symbol_table: SymbolTable::new(),
        }
    }

    /// Get our symbol table, for looking up names.
    pub fn symbol_table(&self) -> &SymbolTable<'static> {
        &self.symbol_table
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

    /// (Helper.) Insert a function signature
    fn insert_func_sig(
        &mut self,
        name: &Ident,
        sig: &FuncSig,
    ) -> Result<(DeclIdx<IndexedType>, DeclIdx<FuncSig>)> {
        let type_idx = self.find_or_insert_type(IndexedType::Func(sig.func_type()?));
        let idx = self.func_decls.insert(sig.clone());
        self.symbol_table.insert(
            name.clone(),
            Symbol::Func {
                idx,
                func_sig: Box::new(sig.clone()),
            },
        )?;
        Ok((type_idx, idx))
    }

    /// Insert an import declaration.
    pub fn insert_import(&mut self, mod_name: Ident, name: Ident, import: Import) -> Result<()> {
        let (type_idx, _idx) = self.insert_func_sig(&name, import.sig())?;
        self.imports_sec.import(
            mod_name.as_str(),
            name.as_str(),
            EntityType::Function(type_idx.try_as_u32()?),
        );
        Ok(())
    }

    /// Insert a function declaration.
    pub fn insert_function(&mut self, name: Ident, func: Func) -> Result<()> {
        let (type_idx, idx) = self.insert_func_sig(&name, func.sig())?;
        self.funcs_sec.function(type_idx.try_as_u32()?);
        if func.should_export() {
            self.export(&name, ExportKind::Func, idx)?;
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
        // Correct order (I believe): type (1), import (2), function (3), table
        // (4), memory (5), global (6), export (7), start (8), element (9), data
        // count (12), code (10), data (11).
        let mut module = Module::new();
        module.section(&self.types_sec);
        module.section(&self.imports_sec);
        module.section(&self.funcs_sec);
        module.section(&self.exports_sec);
        module.section(&self.codes_sec);
        module
    }
}
