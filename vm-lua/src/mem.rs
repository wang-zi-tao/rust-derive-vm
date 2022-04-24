use crate::TypeResourceImpl;

use jvm_core::{make_reference, Aligned, FunctionType, MoveIntoObject, Native, ObjectBuilder, Pointer, Reference, Resource, SymbolRef, Type, TypeDeclaration, TypeLayout, UnsizedArray};
use lexical::_lazy_static::lazy_static;

use runtime_extra::ty::*;
use std::cell::UnsafeCell;
use std::{collections::HashMap, hash::Hash};
use util::{inline_const, CowArc, CowSlice, PooledStr};
#[derive(TypeDeclaration)]
#[make_type(make_instruction)]
pub struct LuaState {
    pub string_meta_functions: LuaMetaFunctionsReference,
    pub table_shape: LuaShapeReference,
    pub global: LuaTableReference,
    pub gc_mark: Bool,
}
make_reference!(LuaStateReference, LuaState, TypeResourceImpl);
#[derive(TypeDeclaration)]
#[make_type(make_instruction)]
#[allow(non_snake_case)]
pub struct LuaMetaFunctions {
    pub valid: Bool,
    pub meta_table: LuaTableReference,
    pub parent: NullableOption<LuaMetaFunctionsReference>,
    pub sub_metatable: Native<Vec<LuaMetaFunctionsReference>>,
    pub add: LuaValue,
    pub sub: LuaValue,
    pub mul: LuaValue,
    pub div: LuaValue,
    pub mod_: LuaValue,
    pub pow: LuaValue,
    pub unm: LuaValue,
    pub idiv: LuaValue,
    pub band: LuaValue,
    pub bor: LuaValue,
    pub bxor: LuaValue,
    pub bnot: LuaValue,
    pub shl: LuaValue,
    pub shr: LuaValue,
    pub concat: LuaValue,
    pub len: LuaValue,
    pub eq: LuaValue,
    pub lt: LuaValue,
    pub le: LuaValue,
    pub index: LuaValue,
    pub newindex: LuaValue,
    pub call: LuaValue,
    pub metadata: LuaValue,
    pub gc: LuaValue,
    pub mode: LuaValue,
    pub name: LuaValue,
    pub tostring: LuaValue,
    pub pairs: LuaValue,
}
make_reference!(LuaMetaFunctionsReference, LuaMetaFunctions, TypeResourceImpl);
#[derive(TypeDeclaration)]
#[make_type(make_instruction)]
pub enum Lifetime {
    Live,
    Dead,
}
#[derive(TypeDeclaration)]
#[make_type(make_instruction)]
pub struct LuaString {
    pub align: Aligned<16>,
    pub lua_state: LuaStateReference,
    pub pooled: Native<Option<PooledStr>>,
    #[make_type(unsized)]
    pub data: UnsizedArray<U8>,
}
make_reference!(LuaStringReference, LuaString, TypeResourceImpl);
#[derive(TypeDeclaration)]
#[make_type(make_instruction)]
pub struct LuaFunction {
    pub align: Aligned<16>,
    pub function: LuaFunctionType,
    pub object_ref: Native<SymbolRef>,
    pub state: LuaStateReference,
}
make_reference!(LuaFunctionReference, LuaFunction, TypeResourceImpl);
#[derive(TypeDeclaration)]
#[make_type(make_instruction)]
pub struct LuaClosure {
    pub align: Aligned<16>,
    pub function: LuaClosureFunctionType,
    pub object_ref: Native<SymbolRef>,
    pub state: LuaStateReference,
    #[make_type(unsized)]
    pub up_values: UnsizedArray<LuaUpValueReference>,
}
make_reference!(LuaClosureReference, LuaClosure, TypeResourceImpl);
#[derive(TypeDeclaration)]
#[make_type(make_instruction)]
pub struct LuaUpValue {
    pub owned: NullablePointer<UnsizedArray<LuaValue>>,
    #[make_type(unsized)]
    pub pointers: UnsizedArray<NullablePointer<LuaValue>>,
}
make_reference!(LuaUpValueReference, LuaUpValue, TypeResourceImpl);
#[derive(TypeDeclaration)]
#[make_type(make_instruction)]
pub struct LuaSlotMetadata {
    pub slot: Usize,
}
impl Clone for LuaSlotMetadataImpl {
    fn clone(&self) -> Self { Self(self.0) }
}
#[derive(TypeDeclaration)]
#[make_type(make_instruction)]
pub struct LuaShape {
    pub fields: Native<UnsafeCell<HashMap<LuaValueImpl, LuaSlotMetadataImpl>>>,
    pub meta_functions: LuaMetaFunctionsReference,
    pub as_meta_table: NullableOption<LuaMetaFunctionsReference>,
    pub max_int_index: U64,
    pub is_owned: Bool,
    pub action_of_field: Native<UnsafeCell<HashMap<LuaValueImpl, (LuaShapeReference, usize)>>>,
    pub action_of_metatable: Native<UnsafeCell<HashMap<LuaTableReference, LuaShapeReference>>>,
    pub invalid: BoolReference,
}
make_reference!(LuaShapeReference, LuaShape, TypeResourceImpl);
make_reference!(BoolReference, Bool, TypeResourceImpl);
#[derive(TypeDeclaration)]
#[make_type(make_instruction)]
pub struct LuaTable {
    pub align: Aligned<16>,
    pub shape: LuaShapeReference,
    pub slow_fields: NullablePointer<UnsizedArray<LuaValue>>,
    #[make_type(unsized)]
    pub fast_fields: UnsizedArray<LuaValue>,
}
make_reference!(LuaTableReference, LuaTable, TypeResourceImpl);
make_reference!(LuaValueArrayReference, UnsizedArray<LuaValue>, TypeResourceImpl);
#[derive(TypeDeclaration)]
#[make_type(make_instruction)]
pub struct LuaI64 {
    pub value: I64,
    pub align: Aligned<16>,
}
pub type LuaI64Reference = Reference<LuaI64, TypeResourceImpl>;
#[derive(TypeDeclaration)]
#[make_type(make_instruction)]
pub struct LuaF64 {
    pub value: F64,
    pub align: Aligned<16>,
}
pub type LuaF64Reference = Reference<LuaF64, TypeResourceImpl>;
pub type I64Reference = Reference<I64, TypeResourceImpl>;
pub type F64Reference = Reference<F64, TypeResourceImpl>;
#[derive(TypeDeclaration)]
#[make_type(make_instruction,tag_mask=[0..4])]
pub enum LuaValue {
    Integer(I64),
    BigInt(LuaI64Reference),
    Float(I64),
    BigFloat(LuaF64Reference),
    Boolean(Bool),
    String(LuaStringReference),
    Nil,
    Table(LuaTableReference),
    Function(LuaFunctionReference),
    Closure(LuaClosureReference),
}
impl MoveIntoObject for LuaValueImpl {
    fn set<'l>(self, offset: usize, object_builder: &ObjectBuilder<'l>, token: &mut ghost_cell::GhostToken<'l>) {
        object_builder.borrow_mut(token).receive_at(offset).write(self.0);
    }
}
impl Clone for LuaValueImpl {
    fn clone(&self) -> Self { Self(self.0, self.1) }
}
impl Hash for LuaValueImpl {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
        self.1.hash(state);
    }
}
impl PartialEq for LuaValueImpl {
    fn eq(&self, other: &Self) -> bool { self.0 == other.0 && self.1 == other.1 }
}
impl Eq for LuaValueImpl {}

pub enum LuaFunctionType {}
impl TypeDeclaration for LuaFunctionType {
    type Impl = LuaFunctionRustType;

    const LAYOUT: jvm_core::TypeLayout = TypeLayout::of::<LuaFunctionRustType>();

    const TYPE: jvm_core::Type = Type::Function(CowArc::Ref(inline_const!(
        [&'static FunctionType]
        &FunctionType {
            dispatch: CowSlice::new(),
            return_type: Some(Pointer::<UnsizedArray<LuaValue>>::TYPE),
            args: CowSlice::Ref(inline_const!([&'static [Type]]&[LuaStateReference::TYPE])),
            va_arg: Some(LuaValue::TYPE),
    })));
}
pub enum LuaClosureFunctionType {}
impl TypeDeclaration for LuaClosureFunctionType {
    type Impl = LuaClosureRustType;

    const LAYOUT: jvm_core::TypeLayout = TypeLayout::of::<LuaClosureRustType>();

    const TYPE: jvm_core::Type = Type::Function(CowArc::Ref(inline_const!(
        [&'static FunctionType]
        &FunctionType {
            dispatch: CowSlice::new(),
            return_type: Some(Pointer::<UnsizedArray<LuaValue>>::TYPE),
            args: CowSlice::Ref(inline_const!(
                    [&'static [Type]]
                    &[LuaStateReference::TYPE,LuaClosureReference::TYPE])),
            va_arg: Some(LuaValue::TYPE),
    })));
}
pub type LuaFunctionRustType = fn(state: LuaStateReference, args: &[LuaValueImpl]) -> Pointer<UnsizedArray<LuaValue>>;
pub type LuaClosureRustType = fn(
    state: LuaStateReference,
    closure: LuaClosureReference,
    args: &[LuaValueImpl],
) -> Pointer<UnsizedArray<LuaValue>>;
impl LuaState {}
