use std::{cmp::max, fmt};

use miette::Result;
use smallvec::{SmallVec, smallvec};
use tree_sitter_wasl_types::nodes;
use type_sitter::Node as _;
use wasm_encoder::{InstructionSink, ValType as WasmValType};

use crate::{
    ast::{FromGrammar, Ident, NodeResultExt},
    errors::{ParseError, TypeCheckError},
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

    /// Is this a signed numeric type?
    fn is_signed(&self) -> bool {
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

impl FromGrammar for PtrType {
    type Input<'a> = nodes::PtrType<'a>;

    fn from_grammar(src: &Source, ty: nodes::PtrType<'_>) -> Result<Self, ParseError> {
        let is_mut = ty.r#mut().is_some();
        let is_nullable = ty.null().is_some();
        let storage_ty = Box::new(LinearStorageType::from_grammar(
            src,
            ty.to_type().expect_matching(),
        )?);
        Ok(Self {
            is_mut,
            is_nullable,
            storage_ty,
        })
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

/// Types that can be used as "value" types (pushed to the managed stack, passed
/// to and returned from managed functions, stored in locals and globals, etc).
///
/// In reality, any numeric type smaller than 32 bits will be represented as 32
/// bits in many value contexts, and masking will be used to limit the number of
/// bits.
///
/// Note that this does not include GC reference types, which cannot be stored
/// in linear memory.
#[derive(Clone, Debug)]
pub enum LinearValTypeVariant {
    I8,
    U8,
    I32,
    U32,
    Bool,
    Ptr(Box<PtrType>),
}

/// Types that can be used as "value" types (pushed to the managed stack, etc.).
/// See [`LinearValTypeVariant`] for details.
#[derive(Clone, Debug)]
pub struct LinearValType {
    #[expect(dead_code)]
    loc: Loc,
    variant: LinearValTypeVariant,
}

impl LinearValType {
    pub fn i8(loc: &Loc) -> Self {
        Self {
            loc: loc.clone(),
            variant: LinearValTypeVariant::I8,
        }
    }

    pub fn u8(loc: &Loc) -> Self {
        Self {
            loc: loc.clone(),
            variant: LinearValTypeVariant::U8,
        }
    }
    pub fn i32(loc: &Loc) -> Self {
        Self {
            loc: loc.clone(),
            variant: LinearValTypeVariant::I32,
        }
    }

    pub fn u32(loc: &Loc) -> Self {
        Self {
            loc: loc.clone(),
            variant: LinearValTypeVariant::U32,
        }
    }

    pub fn bool(loc: &Loc) -> Self {
        Self {
            loc: loc.clone(),
            variant: LinearValTypeVariant::Bool,
        }
    }

    #[cfg(test)]
    pub fn i32_for_test() -> Self {
        Self {
            loc: Loc::new_for_test(),
            variant: LinearValTypeVariant::I32,
        }
    }

    /// Emit a potential "mask" operation after constructing a new stack-based
    /// value of a type. This is used to truncate `u8` (etc) values, and
    /// truncate and sign-extend `i8` (etc) values after performing arithmetic
    /// operations.
    pub fn emit_mask(&self, sink: &mut InstructionSink<'_>) -> Result<()> {
        match &self.variant {
            LinearValTypeVariant::I8 => {
                // Truncate and sign extend.
                sink.i32_const(0xFF);
                sink.i32_and();
                sink.i32_extend8_s();
            }
            LinearValTypeVariant::U8 => {
                // Truncate.
                sink.i32_const(0xFF);
                sink.i32_and();
            }
            LinearValTypeVariant::I32 | LinearValTypeVariant::U32 => {}
            LinearValTypeVariant::Bool => {}
            LinearValTypeVariant::Ptr(_) => {
                unreachable!("pointer types are not currently numeric")
            }
        }
        Ok(())
    }
}

impl fmt::Display for LinearValType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.variant {
            LinearValTypeVariant::I8 => "i8".fmt(f),
            LinearValTypeVariant::U8 => "u8".fmt(f),
            LinearValTypeVariant::I32 => "i32".fmt(f),
            LinearValTypeVariant::U32 => "u32".fmt(f),
            LinearValTypeVariant::Bool => "bool".fmt(f),
            LinearValTypeVariant::Ptr(ptr_type) => write!(f, "{ptr_type}"),
        }
    }
}

impl FromGrammar for LinearValType {
    type Input<'a> = nodes::LinearValType<'a>;

    fn from_grammar(src: &Source, ty: nodes::LinearValType<'_>) -> Result<Self, ParseError> {
        let loc = src.loc_for(ty.raw());
        let variant = match ty {
            nodes::LinearValType::I8(_) => LinearValTypeVariant::I8,
            nodes::LinearValType::U8(_) => LinearValTypeVariant::U8,
            nodes::LinearValType::I32(_) => LinearValTypeVariant::I32,
            nodes::LinearValType::U32(_) => LinearValTypeVariant::U32,
            nodes::LinearValType::Bool(_) => LinearValTypeVariant::Bool,
            nodes::LinearValType::PtrType(ptr_type) => {
                LinearValTypeVariant::Ptr(Box::new(PtrType::from_grammar(src, ptr_type)?))
            }
        };
        Ok(Self { loc, variant })
    }
}

impl IsSubtypeOf for LinearValType {
    fn is_subtype_of(&self, other: &Self) -> bool {
        use LinearValTypeVariant as LVTV;
        match (&self.variant, &other.variant) {
            (LVTV::I8, LVTV::I8) => true,
            (LVTV::U8, LVTV::U8) => true,
            (LVTV::I32, LVTV::I32) => true,
            (LVTV::U32, LVTV::U32) => true,
            (LVTV::Bool, LVTV::Bool) => true,
            (LVTV::Ptr(ptr1), LVTV::Ptr(ptr2)) => ptr1.is_subtype_of(ptr2),
            _ => false,
        }
    }

    fn is_numeric(&self) -> bool {
        match &self.variant {
            LinearValTypeVariant::I8
            | LinearValTypeVariant::U8
            | LinearValTypeVariant::I32
            | LinearValTypeVariant::U32 => true,
            LinearValTypeVariant::Bool => false,
            // No pointer math at the current time. Needs further thought if we want to learn
            // towards C or Rust or what in terms of language semantics.
            LinearValTypeVariant::Ptr(_) => false,
        }
    }

    fn is_signed(&self) -> bool {
        match &self.variant {
            LinearValTypeVariant::I8 | LinearValTypeVariant::I32 => true,
            LinearValTypeVariant::U8
            | LinearValTypeVariant::U32
            | LinearValTypeVariant::Bool
            | LinearValTypeVariant::Ptr(_) => false,
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
            LinearValTypeVariant::I8 | LinearValTypeVariant::U8 | LinearValTypeVariant::Bool => 1,
            LinearValTypeVariant::I32
            | LinearValTypeVariant::U32
            | LinearValTypeVariant::Ptr { .. } => 4,
        }
    }
}

/// Types which can be stored in linear memory. This includes [`LinearValType`] types, plus
/// larger types which cannot be passed around by value.
#[derive(Clone, Debug)]
pub enum LinearStorageTypeVariant {
    LinearValType(Box<LinearValType>),
    LinearRecordType(Box<LinearRecordType>),
}

/// Types which can be stored in linear memory.
#[derive(Clone, Debug)]
pub struct LinearStorageType {
    #[expect(dead_code)]
    loc: Loc,
    variant: LinearStorageTypeVariant,
}

impl fmt::Display for LinearStorageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.variant {
            LinearStorageTypeVariant::LinearValType(ty) => write!(f, "{ty}"),
            LinearStorageTypeVariant::LinearRecordType(rec) => write!(f, "{rec}"),
        }
    }
}

impl FromGrammar for LinearStorageType {
    type Input<'a> = nodes::LinearStorageType<'a>;

    fn from_grammar(src: &Source, ty: nodes::LinearStorageType<'_>) -> Result<Self, ParseError> {
        let loc = src.loc_for(ty.raw());
        let variant = match ty {
            nodes::LinearStorageType::LinearValType(ty) => LinearStorageTypeVariant::LinearValType(
                Box::new(LinearValType::from_grammar(src, ty)?),
            ),
            nodes::LinearStorageType::LinearRecordType(ty) => {
                LinearStorageTypeVariant::LinearRecordType(Box::new(
                    LinearRecordType::from_grammar(src, ty)?,
                ))
            }
        };
        Ok(Self { loc, variant })
    }
}

impl IsSubtypeOf for LinearStorageType {
    fn is_subtype_of(&self, other: &Self) -> bool {
        use LinearStorageTypeVariant as LSTV;
        match (&self.variant, &other.variant) {
            (LSTV::LinearValType(ty1), LSTV::LinearValType(ty2)) => ty1.is_subtype_of(ty2),
            (LSTV::LinearRecordType(rec1), LSTV::LinearRecordType(rec2)) => {
                rec1.is_subtype_of(rec2)
            }
            _ => false,
        }
    }

    fn is_numeric(&self) -> bool {
        match &self.variant {
            LinearStorageTypeVariant::LinearValType(ty) => ty.is_numeric(),
            LinearStorageTypeVariant::LinearRecordType(_) => false,
        }
    }

    fn is_signed(&self) -> bool {
        match &self.variant {
            LinearStorageTypeVariant::LinearValType(ty) => ty.is_signed(),
            LinearStorageTypeVariant::LinearRecordType(_) => false,
        }
    }
}

impl LinearStorable for LinearStorageType {
    fn size_of(&self) -> usize {
        match &self.variant {
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

impl FromGrammar for LinearRecordType {
    type Input<'a> = nodes::LinearRecordType<'a>;

    fn from_grammar(src: &Source, ty: nodes::LinearRecordType<'_>) -> Result<Self, ParseError> {
        let mut rec = Self::empty();
        let mut c = ty.walk();
        for field in ty.fields(&mut c) {
            let field = field.expect_matching();
            let name = Ident::from_grammar(src, field.name().expect_matching())?;
            let field_ty = LinearStorageType::from_grammar(src, field.r#type().expect_matching())?;
            rec.add_field(name, field_ty);
        }
        Ok(rec)
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
    pub fn i8(loc: &Loc) -> Self {
        Self {
            loc: loc.clone(),
            variant: ValTypeVariant::Linear(LinearValType::i8(loc)),
        }
    }

    pub fn u8(loc: &Loc) -> Self {
        Self {
            loc: loc.clone(),
            variant: ValTypeVariant::Linear(LinearValType::u8(loc)),
        }
    }

    pub fn i32(loc: &Loc) -> Self {
        Self {
            loc: loc.clone(),
            variant: ValTypeVariant::Linear(LinearValType::i32(loc)),
        }
    }

    pub fn u32(loc: &Loc) -> Self {
        Self {
            loc: loc.clone(),
            variant: ValTypeVariant::Linear(LinearValType::u32(loc)),
        }
    }

    pub fn bool(loc: &Loc) -> Self {
        Self {
            loc: loc.clone(),
            variant: ValTypeVariant::Linear(LinearValType::bool(loc)),
        }
    }

    #[cfg(test)]
    pub fn i32_for_test() -> Self {
        Self {
            loc: Loc::new_for_test(),
            variant: ValTypeVariant::Linear(LinearValType::i32_for_test()),
        }
    }

    /// Emit any masking operation needed for this type.
    ///
    /// This is a no-op for booleans and other types that never need masks.
    pub fn emit_mask(&self, sink: &mut InstructionSink<'_>) -> Result<()> {
        match &self.variant {
            ValTypeVariant::Linear(ty) => ty.emit_mask(sink),
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

impl FromGrammar for ValType {
    type Input<'a> = nodes::ValType<'a>;

    fn from_grammar(src: &Source, ty: nodes::ValType<'_>) -> Result<Self, ParseError> {
        let loc = src.loc_for(ty.raw());
        let variant = match ty {
            nodes::ValType::LinearValType(ty) => {
                ValTypeVariant::Linear(LinearValType::from_grammar(src, ty)?)
            }
        };
        Ok(Self { loc, variant })
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

    fn is_signed(&self) -> bool {
        match &self.variant {
            ValTypeVariant::Linear(ty) => ty.is_signed(),
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
    /// Construct a empty expression type.
    pub fn void() -> Self {
        ExprType { tys: smallvec![] }
    }

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

    /// Raise an error if this type is not a subtype of `expected`.
    pub fn expecting(&self, loc: &Loc, expected: &ExprType) -> Result<()> {
        if self.is_subtype_of(expected) {
            Ok(())
        } else {
            Err(TypeCheckError::not_expected(loc, self.clone(), expected.clone()).into())
        }
    }

    /// If this is a single value, extract it. Otherwise return and error.
    pub fn expecting_value_count(&self, loc: &Loc, expected: usize) -> Result<&ValType> {
        if self.tys.len() != expected {
            return Err(TypeCheckError::wrong_number_of_values(loc, expected, self.clone()).into());
        }
        Ok(&self.tys[0])
    }

    /// If this [`ExprType`] contains a single [`ValueType`], return it.
    /// Otherwise panic.
    pub fn expect_single(&self) -> &ValType {
        assert_eq!(self.tys.len(), 1, "{self} should contain exactly one type");
        &self.tys[0]
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

    fn is_signed(&self) -> bool {
        self.tys.len() == 1 && self.tys[0].is_signed()
    }
}
