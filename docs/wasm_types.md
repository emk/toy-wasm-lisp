# Semi-basic WASM types

```txt
pub enum ValType {
    I32,
    I64,
    F32,
    F64,
    V128,
    (GC) Ref(RefType),
}

pub enum StorageType {
    I8,
    I16,
    Val(ValType),
}

CoreTypeEncoder 
- func_type FucType
- (GC) array StorageType
- (GC) struct FieldType..
- (GC) cont ContType
- (GC) subtype SubType
- (GC) rec SubType..

(GC) FieldType -> mutable?, StorageType

(Components) pub enum PrimitiveValType {
    Bool,
    S8,
    U8,
    S16,
    U16,
    S32,
    U32,
    S64,
    U64,
    F32,
    F64,
    Char,
    String,
    ErrorContext,
}

Memory loading:
- i32_load8_s
- i32_load8_u
- i32_load16_s
- i32_load16_u
- i32 supports load/store when loading to/storing from i64
- i64 is always signed on load/store

Comparison:
- Always has signed and unsigned variants for int types

Vectorized types (not currently relevant)
```

## WABI component types

For eventual versions of WASI (post-0.3), [the goal](https://github.com/WebAssembly/component-model/issues/525) is apparently to keep WABI _independent_ of the difference between linear types and GC types, and provide a canonical transformation from the WASI components to either linear types or GC types. So this is not a 100% helpful and clarifying direction to look for our work, which currently attempts to support mixed linear and GC types.
