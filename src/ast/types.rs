use std::{cmp::max, sync::Arc};

use miette::NamedSource;
use tree_sitter_wasl_types::nodes;
use type_sitter::Node as _;
use wasm_encoder::ValType as WasmValType;

use crate::{
    ast::{Ident, NodeResultExt},
    locs::Loc,
};

/// Convert to a native WASM-representable type.
pub trait ToWasmType {
    type Output;

    /// Convert this type to its WASM representation.
    fn to_wasm_type(&self) -> Self::Output;
}

/// Properties shared by types which can be stored in linear memory.
pub trait LinearStorable {
    /// Size of values in bytes.
    fn size_of(&self) -> usize;

    /// Minimum alignment of values in bytes. WASM will allow unaligned reads
    /// and writes, but we try to minimize them.
    fn align_of(&self) -> usize {
        self.size_of()
    }
}

#[derive(Clone, Debug)]
pub struct PtrType {
    is_mut: bool,
    is_nullable: bool,
    storage_ty: Box<LinearStorageType>,
}

impl PtrType {
    pub fn from_grammar(src: Arc<NamedSource<String>>, ty: nodes::PtrType<'_>) -> Self {
        let is_mut = ty.r#mut().is_some();
        let is_nullable = ty.null().is_some();
        let storage_ty = Box::new(LinearStorageType::from_grammar(
            src,
            ty.to_type().expect_matching(),
        ));
        Self {
            is_mut,
            is_nullable,
            storage_ty,
        }
    }
}

#[derive(Clone, Debug)]
pub enum LinearValTypeVariant {
    I32,
    U32,
    Ptr(Box<PtrType>),
}

#[derive(Clone, Debug)]
pub struct LinearValType {
    #[expect(dead_code)]
    loc: Loc,
    variant: LinearValTypeVariant,
}

impl LinearValType {
    pub fn from_grammar(src: Arc<NamedSource<String>>, ty: nodes::LinearValType<'_>) -> Self {
        let loc = Loc::new(src.clone(), ty.raw());
        let variant = match ty {
            nodes::LinearValType::I32(_) => LinearValTypeVariant::I32,
            nodes::LinearValType::U32(_) => LinearValTypeVariant::U32,
            nodes::LinearValType::PtrType(ptr_type) => {
                LinearValTypeVariant::Ptr(Box::new(PtrType::from_grammar(src, ptr_type)))
            }
        };
        Self { loc, variant }
    }

    #[cfg(test)]
    pub fn new_i32_for_test() -> Self {
        Self {
            loc: Loc::new_for_test(),
            variant: LinearValTypeVariant::I32,
        }
    }
}

impl ToWasmType for LinearValType {
    type Output = WasmValType;

    fn to_wasm_type(&self) -> Self::Output {
        WasmValType::I32
    }
}

impl LinearStorable for LinearValType {
    fn size_of(&self) -> usize {
        match &self.variant {
            LinearValTypeVariant::I32
            | LinearValTypeVariant::U32
            | LinearValTypeVariant::Ptr { .. } => 4,
        }
    }
}

#[derive(Clone, Debug)]
pub enum LinearStorageTypeVariant {
    I8,
    U8,
    LinearValType(Box<LinearValType>),
    LinearRecordType(Box<LinearRecordType>),
}

#[derive(Clone, Debug)]
pub struct LinearStorageType {
    #[expect(dead_code)]
    loc: Loc,
    variant: LinearStorageTypeVariant,
}

impl LinearStorageType {
    pub fn from_grammar(src: Arc<NamedSource<String>>, ty: nodes::LinearStorageType<'_>) -> Self {
        let loc = Loc::new(src.clone(), ty.raw());
        let variant = match ty {
            nodes::LinearStorageType::I8(_) => LinearStorageTypeVariant::I8,
            nodes::LinearStorageType::U8(_) => LinearStorageTypeVariant::U8,
            nodes::LinearStorageType::LinearValType(ty) => LinearStorageTypeVariant::LinearValType(
                Box::new(LinearValType::from_grammar(src, ty)),
            ),
            nodes::LinearStorageType::LinearRecordType(ty) => {
                LinearStorageTypeVariant::LinearRecordType(Box::new(
                    LinearRecordType::from_grammar(src, ty),
                ))
            }
        };
        Self { loc, variant }
    }
}

impl LinearStorable for LinearStorageType {
    fn size_of(&self) -> usize {
        match &self.variant {
            LinearStorageTypeVariant::I8 | LinearStorageTypeVariant::U8 => 2,
            LinearStorageTypeVariant::LinearValType(ty) => ty.size_of(),
            LinearStorageTypeVariant::LinearRecordType(rec) => rec.size_of(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct LinearRecordType {
    fields: Vec<LinearField>,
    size: usize,
    align: usize,
}

impl LinearRecordType {
    pub fn from_grammar(src: Arc<NamedSource<String>>, ty: nodes::LinearRecordType<'_>) -> Self {
        let mut rec = Self::empty();
        let mut c = ty.walk();
        for field in ty.fields(&mut c) {
            let field = field.expect_matching();
            let name = Ident::from_grammar(src.clone(), field.name().expect_matching());
            let field_ty =
                LinearStorageType::from_grammar(src.clone(), field.r#type().expect_matching());
            rec.add_field(name, field_ty);
        }
        rec
    }

    /// Create an empty record type. Fields may be added using [`Self::add_field`].
    fn empty() -> Self {
        Self {
            fields: vec![],
            size: 0,
            align: 0,
        }
    }

    /// Add a [`LinearField`] to this record type, updating size, alignment and
    /// offset appropriately.
    fn add_field(&mut self, name: Ident, ty: LinearStorageType) -> &mut Self {
        let need_align = ty.align_of();
        let overhang_bytes = self.size % need_align;
        if overhang_bytes > 0 {
            let padding = need_align.strict_sub(overhang_bytes);
            self.size += padding;
        }
        debug_assert!(
            self.size.is_multiple_of(need_align),
            "computed incorrect field alignment"
        );
        let offset = self.size;
        self.size += ty.size_of();
        self.align = max(self.align, need_align);
        self.fields.push(LinearField {
            name,
            ty: Box::new(ty),
            offset,
        });
        self
    }
}

impl LinearStorable for LinearRecordType {
    fn size_of(&self) -> usize {
        self.size
    }

    fn align_of(&self) -> usize {
        self.align
    }
}

#[derive(Clone, Debug)]
pub struct LinearField {
    name: Ident,
    ty: Box<LinearStorageType>,
    /// Offset from start of record.
    offset: usize,
}

impl LinearStorable for LinearField {
    fn size_of(&self) -> usize {
        self.ty.size_of()
    }

    fn align_of(&self) -> usize {
        self.ty.align_of()
    }
}

/// Type variants for [`ValType`].
#[derive(Clone, Debug)]
pub enum ValTypeVariant {
    Linear(LinearValType),
}

/// Types that can be stored on the managed stack, stored in managed locals,
/// passed to functions and returned from functions.
#[derive(Clone, Debug)]
pub struct ValType {
    #[expect(dead_code)]
    loc: Loc,
    variant: ValTypeVariant,
}

impl ValType {
    pub fn from_grammar(src: Arc<NamedSource<String>>, ty: nodes::ValType<'_>) -> Self {
        let loc = Loc::new(src.clone(), ty.raw());
        let variant = match ty {
            nodes::ValType::LinearValType(ty) => {
                ValTypeVariant::Linear(LinearValType::from_grammar(src, ty))
            }
        };
        Self { loc, variant }
    }

    #[cfg(test)]
    pub fn new_i32_for_test() -> Self {
        Self {
            loc: Loc::new_for_test(),
            variant: ValTypeVariant::Linear(LinearValType::new_i32_for_test()),
        }
    }
}

impl ToWasmType for ValType {
    type Output = WasmValType;

    fn to_wasm_type(&self) -> Self::Output {
        match &self.variant {
            ValTypeVariant::Linear(ty) => ty.to_wasm_type(),
        }
    }
}
