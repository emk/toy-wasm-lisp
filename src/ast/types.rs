use std::{cmp::max, fmt};

use miette::Result;
use smallvec::{SmallVec, smallvec};
use tree_sitter_wasl_types::nodes;
use type_sitter::Node as _;
use wasm_encoder::ValType as WasmValType;

use crate::{
    ast::{Ident, NodeResultExt},
    errors::TypeCheckError,
    locs::{Loc, Source},
};

/// Check for a subtype relationship.
pub trait IsSubtypeOf {
    /// Can `self` be used anywhere that `other` can be used?
    fn is_subtype_of(&self, other: &Self) -> bool;

    /// Are these two types equal?
    fn type_eq(&self, other: &Self) -> bool {
        self.is_subtype_of(other) && other.is_subtype_of(self)
    }

    /// Is this a numeric type that can be used with standard numeric
    /// operations? This is used for operators like `+` and `*` in place of a
    /// proper numeric subtype hierarchy.
    fn is_numeric(&self) -> bool {
        false
    }
}

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
    pub fn from_grammar(src: &Source, ty: nodes::PtrType<'_>) -> Self {
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

impl fmt::Display for PtrType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "*")?;
        if self.is_mut {
            write!(f, "mut ")?;
        }
        if self.is_nullable {
            write!(f, "null ")?;
        }
        write!(f, "{}", self.storage_ty)
    }
}

impl IsSubtypeOf for PtrType {
    /// Can `self` _always_ be passed to something expecting `other`?
    fn is_subtype_of(&self, other: &Self) -> bool {
        // Cannot pass *T to *mut T.
        if !self.is_mut && other.is_mut {
            return false;
        }
        // Cannot pass *null T to *T.
        if self.is_nullable && !other.is_nullable {
            return false;
        }
        self.storage_ty.is_subtype_of(&other.storage_ty)
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
    pub fn from_grammar(src: &Source, ty: nodes::LinearValType<'_>) -> Self {
        let loc = src.loc_for(ty.raw());
        let variant = match ty {
            nodes::LinearValType::I32(_) => LinearValTypeVariant::I32,
            nodes::LinearValType::U32(_) => LinearValTypeVariant::U32,
            nodes::LinearValType::PtrType(ptr_type) => {
                LinearValTypeVariant::Ptr(Box::new(PtrType::from_grammar(src, ptr_type)))
            }
        };
        Self { loc, variant }
    }

    pub fn i32(loc: &Loc) -> Self {
        Self {
            loc: loc.clone(),
            variant: LinearValTypeVariant::I32,
        }
    }

    #[cfg(test)]
    pub fn i32_for_test() -> Self {
        Self {
            loc: Loc::new_for_test(),
            variant: LinearValTypeVariant::I32,
        }
    }
}

impl fmt::Display for LinearValType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.variant {
            LinearValTypeVariant::I32 => "i32".fmt(f),
            LinearValTypeVariant::U32 => "u32".fmt(f),
            LinearValTypeVariant::Ptr(ptr_type) => write!(f, "{ptr_type}"),
        }
    }
}

impl IsSubtypeOf for LinearValType {
    fn is_subtype_of(&self, other: &Self) -> bool {
        use LinearValTypeVariant as LVTV;
        match (&self.variant, &other.variant) {
            (LVTV::I32, LVTV::I32) => true,
            (LVTV::U32, LVTV::U32) => true,
            (LVTV::Ptr(ptr1), LVTV::Ptr(ptr2)) => ptr1.is_subtype_of(ptr2),
            _ => false,
        }
    }

    fn is_numeric(&self) -> bool {
        matches!(
            self.variant,
            LinearValTypeVariant::I32 | LinearValTypeVariant::U32
        )
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
    pub fn from_grammar(src: &Source, ty: nodes::LinearStorageType<'_>) -> Self {
        let loc = src.loc_for(ty.raw());
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

impl fmt::Display for LinearStorageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.variant {
            LinearStorageTypeVariant::I8 => "i8".fmt(f),
            LinearStorageTypeVariant::U8 => "u8".fmt(f),
            LinearStorageTypeVariant::LinearValType(ty) => write!(f, "{ty}"),
            LinearStorageTypeVariant::LinearRecordType(rec) => write!(f, "{rec}"),
        }
    }
}

impl IsSubtypeOf for LinearStorageType {
    fn is_subtype_of(&self, other: &Self) -> bool {
        use LinearStorageTypeVariant as LSTV;
        match (&self.variant, &other.variant) {
            (LSTV::I8, LSTV::I8) => true,
            (LSTV::U8, LSTV::U8) => true,
            (LSTV::LinearValType(ty1), LSTV::LinearValType(ty2)) => ty1.is_subtype_of(ty2),
            (LSTV::LinearRecordType(rec1), LSTV::LinearRecordType(rec2)) => {
                rec1.is_subtype_of(rec2)
            }
            _ => false,
        }
    }

    fn is_numeric(&self) -> bool {
        matches!(
            self.variant,
            LinearStorageTypeVariant::I8 | LinearStorageTypeVariant::U8
        )
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
    pub fn from_grammar(src: &Source, ty: nodes::LinearRecordType<'_>) -> Self {
        let mut rec = Self::empty();
        let mut c = ty.walk();
        for field in ty.fields(&mut c) {
            let field = field.expect_matching();
            let name = Ident::from_grammar(src, field.name().expect_matching());
            let field_ty = LinearStorageType::from_grammar(src, field.r#type().expect_matching());
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

impl fmt::Display for LinearRecordType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "record {{ ")?;
        for (i, field) in self.fields.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}: {}", field.name, field.ty)?;
        }
        write!(f, " }}")
    }
}

impl IsSubtypeOf for LinearRecordType {
    fn is_subtype_of(&self, other: &Self) -> bool {
        if self.fields.len() != other.fields.len() {
            return false;
        }
        for (field1, field2) in self.fields.iter().zip(other.fields.iter()) {
            // Field types are invariant for now.
            if !field1.ty.type_eq(&field2.ty) {
                return false;
            }
        }
        true
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
    #[expect(dead_code)]
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
    pub loc: Loc,
    variant: ValTypeVariant,
}

impl ValType {
    pub fn from_grammar(src: &Source, ty: nodes::ValType<'_>) -> Self {
        let loc = src.loc_for(ty.raw());
        let variant = match ty {
            nodes::ValType::LinearValType(ty) => {
                ValTypeVariant::Linear(LinearValType::from_grammar(src, ty))
            }
        };
        Self { loc, variant }
    }

    pub fn i32(loc: &Loc) -> Self {
        Self {
            loc: loc.clone(),
            variant: ValTypeVariant::Linear(LinearValType::i32(loc)),
        }
    }

    #[cfg(test)]
    pub fn i32_for_test() -> Self {
        Self {
            loc: Loc::new_for_test(),
            variant: ValTypeVariant::Linear(LinearValType::i32_for_test()),
        }
    }
}

impl fmt::Display for ValType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.variant {
            ValTypeVariant::Linear(ty) => write!(f, "{}", ty),
        }
    }
}

impl IsSubtypeOf for ValType {
    fn is_subtype_of(&self, other: &Self) -> bool {
        use ValTypeVariant as VTV;
        match (&self.variant, &other.variant) {
            (VTV::Linear(ty1), VTV::Linear(ty2)) => ty1.is_subtype_of(ty2),
        }
    }

    fn is_numeric(&self) -> bool {
        match &self.variant {
            ValTypeVariant::Linear(ty) => ty.is_numeric(),
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

/// The type of an expression. Used in the type inference algorithm, not the
/// grammar. Void types are created by semi-colon terminated blocks and function
/// calls, and multiple value types are created by function calls.
#[derive(Clone, Debug)]
pub struct ExprType {
    tys: SmallVec<[ValType; 1]>,
}

impl ExprType {
    /// Construct a single type.
    pub fn single(ty: ValType) -> Self {
        Self { tys: smallvec![ty] }
    }

    // Construct a multiple-value type.
    pub fn multiple(iter: impl Iterator<Item = ValType>) -> Self {
        Self {
            tys: SmallVec::from_iter(iter),
        }
    }

    pub fn expecting(&self, loc: &Loc, expected: &ExprType) -> Result<()> {
        if self.is_subtype_of(expected) {
            Ok(())
        } else {
            Err(TypeCheckError::not_expected(loc, self.clone(), expected.clone()).into())
        }
    }
}

impl fmt::Display for ExprType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.tys.len() {
            0 => write!(f, "void"),
            1 => write!(f, "{}", self.tys[0]),
            _ => {
                write!(f, "(")?;
                for (i, ty) in self.tys.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", ty)?;
                }
                write!(f, ")")
            }
        }
    }
}

impl IsSubtypeOf for ExprType {
    fn is_subtype_of(&self, other: &Self) -> bool {
        self.tys.len() == other.tys.len()
            && self
                .tys
                .iter()
                .zip(other.tys.iter())
                .all(|(a, b)| a.is_subtype_of(b))
    }

    fn is_numeric(&self) -> bool {
        self.tys.len() == 1 && self.tys[0].is_numeric()
    }
}
