//! Keeping track of names and indices.
//!
//! This is a bit unusual, because we want to support nested namespaces, but at
//! the same time, we also want to support the ID allocation schema used by
//! WASM.

pub use self::{
    decl_idx::DeclIdx,
    decl_table_handle::DeclTableHandle,
    ident_map::IdentMap,
    local_env::LocalEnv,
    module_env::ModuleEnv,
    type_indexer::{IndexedType, TypeIndexer},
};

mod decl_idx;
mod decl_table_handle;
mod ident_map;
mod local_env;
mod module_env;
mod symbol_table;
mod type_indexer;
