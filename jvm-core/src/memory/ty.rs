use crate::{context::Scope, resources::Resource, Component, MaybeDefinedResource, MoveIntoObject};
use dashmap::DashMap;
use failure::{format_err, Error, Fallible};
use getset::{CopyGetters, Getters};
use lazy_static::lazy_static;
use smallvec::SmallVec;
use std::{
    alloc::Layout,
    any::{Any, TypeId},
    array::TryFromSliceError,
    cell::UnsafeCell,
    convert::TryInto,
    fmt::Debug,
    marker::PhantomData,
    mem::{align_of, size_of},
    ops::{Deref, DerefMut},
    ptr::NonNull,
    sync::{atomic::Ordering, Arc},
};
use util::{inline_const, AsAny, CowArc, CowSlice, DefaultArc};
pub trait TypeLayoutTrait: AsAny + Debug {}
pub trait ImplementLayoutTrait: Sync + Send + Debug {}
pub trait ExecutableLayoutTrait: Sync + Send + Debug {}

pub enum AccessMode {
    Noraml,
    Ordering(Ordering),
}
impl Default for AccessMode {
    fn default() -> Self {
        AccessMode::Noraml
    }
}
pub trait FieldLayoutTrait: Sync + Send + Debug {
    unsafe fn get_value(&self, object: Option<&dyn OOPTrait>, access_mode: AccessMode, value: *const ()) -> Fallible<()>;
    unsafe fn set_value(&self, object: Option<&dyn OOPTrait>, value: *const (), access_mode: AccessMode) -> Fallible<()>;
}
pub trait OOPTrait: Any + Send + Sync + Debug {
    fn as_ptr(&self) -> *const ();
    fn as_non_null(&self) -> NonNull<()>;
}
impl PartialEq for dyn OOPTrait {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for dyn OOPTrait {}
pub trait SyncOOPTrait: OOPTrait + Sync + Send {}
pub type OOPRef = Arc<dyn OOPTrait>;
pub struct RawOOP {
    pub pointer: *const (),
    pub matedata: *const (),
}
pub trait ReferenceKind: Component {
    fn optional(&self) -> bool;
    fn size(&self) -> u32;
    fn align(&self) -> u32;
}
impl PartialEq for dyn ReferenceKind {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for dyn ReferenceKind {}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntKind {
    Bool = 0,
    I8 = 1,
    U8 = 2,
    I16 = 3,
    U16 = 4,
    I32 = 5,
    U32 = 6,
    I64 = 7,
    U64 = 8,
    I128 = 9,
    U128 = 10,
    Isize = 11,
    Usize = 12,
}
impl IntKind {
    pub fn get_layout(self) -> TypeLayout {
        use IntKind::*;
        match self {
            Bool | I8 | U8 => TypeLayout::of::<i8>(),
            I16 | U16 => TypeLayout::of::<i16>(),
            I32 | U32 => TypeLayout::of::<i32>(),
            I64 | U64 => TypeLayout::of::<i64>(),
            I128 | U128 => TypeLayout::of::<i128>(),
            Isize | Usize => TypeLayout::of::<isize>(),
        }
    }

    pub fn get_width(self) -> usize {
        use IntKind::*;
        match self {
            Bool => 1,
            I8 | U8 => 8,
            I16 | U16 => 16,
            I32 | U32 => 32,
            I64 | U64 => 64,
            I128 | U128 => 128,
            Isize | Usize => isize::BITS as usize,
        }
    }

    pub fn size(self) -> usize {
        use IntKind::*;
        match self {
            Bool | I8 | U8 => 1,
            I16 | U16 => 2,
            I32 | U32 => 4,
            I64 | U64 => 8,
            I128 | U128 => 16,
            Isize | Usize => (isize::BITS >> 3) as usize,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatKind {
    F32 = 32,
    F64 = 64,
}
impl FloatKind {
    pub fn get_layout(self) -> TypeLayout {
        use FloatKind::*;
        match self {
            F32 => TypeLayout::of::<f32>(),
            F64 => TypeLayout::of::<f64>(),
        }
    }

    pub fn size(self) -> usize {
        use FloatKind::*;
        match self {
            F32 => 4,
            F64 => 8,
        }
    }
}
#[derive(Clone, PartialEq, Eq)]
pub enum Type {
    Float(FloatKind),
    Int(IntKind),
    MetaData(CowArc<'static, [(OOPRef, CowArc<'static, dyn TypeResource>)]>),
    Const(CowArc<'static, [u8]>, CowArc<'static, Type>),
    Tuple(Tuple),
    Enum(CowArc<'static, Enum>),
    Union(CowArc<'static, [Type]>),
    Pointer(CowArc<'static, Type>),
    Array(CowArc<'static, Type>, Option<usize>),
    Reference(MaybeDefinedResource<dyn TypeResource>),
    Embed(MaybeDefinedResource<dyn TypeResource>),
    Native(Layout),
    Function(CowArc<'static, FunctionType>),
}

impl Debug for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Float(arg0) => arg0.fmt(f),
            Self::Int(arg0) => arg0.fmt(f),
            Self::MetaData(arg0) => f.debug_tuple("MetaData").field(&*arg0).finish(),
            Self::Const(arg0, arg1) => f.debug_tuple("Const").field(&*arg0).field(&*arg1).finish(),
            Self::Tuple(arg0) => arg0.fmt(f),
            Self::Enum(arg0) => arg0.fmt(f),
            Self::Union(arg0) => f.debug_tuple("Union").field(&*arg0).finish(),
            Self::Pointer(arg0) => f.write_str("*").and_then(|_| (&**arg0).fmt(f)),
            Self::Array(arg0, arg1) => {
                f.write_str("[")?;
                (&**arg0).fmt(f)?;
                if let Some(s) = arg1 {
                    f.write_str(";")?;
                    s.fmt(f)?;
                };
                f.write_str("]")
            }
            Self::Reference(arg0) => f.write_str("&").and_then(|_| arg0.fmt(f)),
            Self::Embed(arg0) => f.debug_tuple("Embed").field(arg0).finish(),
            Self::Native(arg0) => f.debug_tuple("Native").field(arg0).finish(),
            Self::Function(arg0) => f.debug_tuple("Function").field(&**arg0).finish(),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnumTagLayout {
    UndefinedValue { end: usize, start: usize },
    UnusedBytes { offset: usize, size: u8 },
    SmallField(SmallElementLayout),
    AppendTag { offset: usize, size: u8 },
}
impl EnumTagLayout {
    pub unsafe fn encode(&self, tag: usize, data: *mut u8) {
        match self {
            Self::AppendTag { offset, size } | Self::UnusedBytes { offset, size } => {
                let tag_ptr = data.add(*offset);
                match size {
                    1 => tag_ptr.cast::<u8>().write(tag as u8),
                    2 => tag_ptr.cast::<u16>().write(tag as u16),
                    4 => tag_ptr.cast::<u32>().write(tag as u32),
                    o => panic!("illegal tag size , except one of `1` `2` `4`,got {} ", o),
                }
            }
            Self::UndefinedValue { start, end: _ } => {
                if tag > 0 {
                    data.cast::<usize>().write(*start + tag - 1)
                }
            }
            Self::SmallField(s) => s.encode(tag, data.cast()),
        }
    }

    pub unsafe fn decode(&self, data: *mut u8) -> usize {
        match self {
            Self::AppendTag { offset, size } | Self::UnusedBytes { offset, size } => {
                let tag_ptr = data.add(*offset);
                match *size {
                    1 => tag_ptr.cast::<u8>().read() as usize,
                    2 => tag_ptr.cast::<u16>().read() as usize,
                    4 => tag_ptr.cast::<u32>().read() as usize,
                    o => panic!("illegal tag size , except one of `1` `2` `4`,got {} ", o),
                }
            }
            Self::UndefinedValue { start, end } => {
                let value = data.cast::<usize>().read();
                if end.overflowing_sub(value).0 > *start {
                    0
                } else {
                    end - value
                }
            }
            Self::SmallField(s) => s.decode(data.cast()),
        }
    }

    pub unsafe fn earse(&self, data: *mut u8) {
        self.encode(0, data)
    }
}
#[derive(Debug, Clone, Builder)]
pub struct MetaData {
    pub value: Value,
    pub ty: Option<CowArc<'static, dyn TypeResource>>,
}
#[derive(Clone, PartialEq, Eq)]
pub enum Tuple {
    Normal(CowArc<'static, [Type]>),
    Compose(CowArc<'static, [(Type, SmallElementLayout)]>),
}

impl Debug for Tuple {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("(")?;
        match self {
            Self::Normal(arg0) => (&**arg0).iter().try_for_each(|e| {
                e.fmt(f)?;
                f.write_str(",")?;
                Ok(())
            })?,
            Self::Compose(arg0) => (&**arg0).iter().try_for_each(|(t, l)| {
                t.fmt(f)?;
                f.write_str("@")?;
                l.fmt(f)?;
                f.write_str(",")?;
                Ok(())
            })?,
        }
        f.write_str(")")?;
        Ok(())
    }
}
#[derive(Clone, PartialEq, Eq)]
pub struct Enum {
    pub variants: CowArc<'static, [Type]>,
    pub tag_layout: EnumTagLayout,
}

impl Debug for Enum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("enum ")?;
        self.tag_layout.fmt(f)?;
        f.write_str("{")?;
        (&*self.variants).iter().try_for_each(|e| {
            e.fmt(f)?;
            f.write_str(",")?;
            Ok(())
        })?;
        f.write_str("}")?;
        Ok(())
    }
}
impl Enum {
    pub fn new(variants: CowArc<'static, [Type]>, tag_layout: EnumTagLayout) -> Self {
        Self { variants, tag_layout }
    }

    pub fn tag_bytes(&self) -> Fallible<Option<usize>> {
        if let EnumTagLayout::AppendTag { .. } = self.tag_layout {
            Ok(Some(match self.variants.len() {
                0..=0xff => 1,
                0x100..=0xffff => 2,
                0x10000..=0xffffffff => 4,
                _ => return Err(format_err!("Too many variants")),
            }))
        } else {
            Ok(None)
        }
    }
}
#[derive(Debug, Clone, Copy, Builder, CopyGetters, PartialEq, Eq)]
pub struct SmallElementLayout {
    #[getset(get_copy = "pub")]
    pub mask: usize,
    #[getset(get_copy = "pub")]
    pub bit_offset: i8,
}
impl SmallElementLayout {
    pub unsafe fn encode(&self, data: usize, ptr: *mut usize) {
        if self.bit_offset > 0 {
            ptr.write((ptr.read() & !self.mask) | self.mask & (data << self.bit_offset))
        } else {
            ptr.write((ptr.read() & !self.mask) | self.mask & (data >> -self.bit_offset))
        }
    }

    pub unsafe fn decode(&self, ptr: *mut usize) -> usize {
        if self.bit_offset > 0 {
            (ptr.read() & self.mask) >> self.bit_offset
        } else {
            (ptr.read() & self.mask) << self.bit_offset
        }
    }
}
impl Default for SmallElementLayout {
    fn default() -> Self {
        Self { mask: usize::MAX, bit_offset: 0 }
    }
}

pub enum FieldKind {
    Parents,
    Element(u32),
    Deref,
}
pub type FieldPath = [FieldKind];
#[derive(Debug, Clone)]
pub struct Value(pub SmallVec<[u8; 16]>);
impl Value {
    pub fn new() -> Self {
        Value(SmallVec::new())
    }

    pub fn int_extend(&mut self, len: usize, sign: bool) {
        let extend_byte = if sign {
            #[cfg(target_endian = "little")]
            {
                (self.last().cloned().unwrap_or(0) & 0x80) as i8 >> 7
            }
            #[cfg(not(target_endian = "little"))]
            {
                (self.first().cloned().unwrap_or(0) & 0x80) as i8 >> 7
            }
        } else {
            0
        };
        #[cfg(target_endian = "little")]
        {
            self.0.insert_many(0, (self.len()..len).map(|_| extend_byte as u8));
        }
        #[cfg(not(target_endian = "little"))]
        {
            self.0.extend((self.len()..len).map(|_| extend_byte as u8));
        }
    }
}
impl Deref for Value {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0.as_slice()
    }
}
impl<const L: usize> From<[u8; L]> for Value {
    fn from(i: [u8; L]) -> Self {
        Self(SmallVec::from_slice(&i))
    }
}
macro_rules! value_conversion {
    ($t:ty) => {
        impl From<$t> for Value {
            fn from(i: $t) -> Self {
                i.to_ne_bytes().into()
            }
        }
        impl TryInto<$t> for Value {
            type Error = TryFromSliceError;

            fn try_into(self) -> Result<$t, Self::Error> {
                Ok(<$t>::from_ne_bytes(self[..].try_into()?))
            }
        }
    };
}
value_conversion!(f32);
value_conversion!(f64);
value_conversion!(usize);
value_conversion!(isize);
value_conversion!(u128);
value_conversion!(i128);
value_conversion!(u64);
value_conversion!(i64);
value_conversion!(u32);
value_conversion!(i32);
value_conversion!(u16);
value_conversion!(i16);
value_conversion!(u8);
value_conversion!(i8);
impl From<bool> for Value {
    fn from(i: bool) -> Self {
        (i as u8).into()
    }
}
impl TryInto<bool> for Value {
    type Error = Error;

    fn try_into(self) -> Result<bool, Self::Error> {
        let value_u8: u8 = self.try_into()?;
        match value_u8 {
            1 => Ok(true),
            0 => Ok(false),
            other => Err(format_err!("invalid bool value:{:X}", other)),
        }
    }
}

impl Type {
    pub fn is_basic_type(&self) -> bool {
        use Type::*;
        match self {
            Int(_) | Float(_) => true,
            _ => false,
        }
    }

    pub fn get_layout(&self) -> Fallible<TypeLayout> {
        Ok(match self {
            Type::Int(kind) => kind.get_layout(),
            Type::Float(kind) => kind.get_layout(),
            Type::Native(layout) => TypeLayout::native(*layout),
            Type::MetaData(_) => TypeLayout { tire: 1, ..Default::default() },
            Type::Const(_, _) => TypeLayout::new(),
            Type::Tuple(Tuple::Normal(fields)) => {
                fields.iter().fold(Fallible::Ok(StructLayoutBuilder::new()), |builder, ty| Ok(builder?.extend(ty.get_layout()?)))?.build()
            }
            Type::Tuple(Tuple::Compose(fields)) => fields
                .iter()
                .fold(Fallible::Ok(StructLayoutBuilder::new()), |builder, (ty, layout)| {
                    Ok(builder?.extend_compose(layout.bit_offset as usize, layout.mask, ty.get_layout()?))
                })?
                .build(),
            Type::Enum(e) => {
                let layout = e.variants.iter().fold(Fallible::Ok(TypeLayout::new()), |s, n| Ok(s?.union(n.get_layout()?)))?;
                if let EnumTagLayout::AppendTag { offset: _, size: _ } = &e.tag_layout {
                    let tag_size = e.tag_bytes()?.unwrap();
                    StructLayoutBuilder::from(layout).extend(TypeLayout { size: tag_size, align: tag_size, ..Default::default() }).build()
                } else {
                    layout
                }
            }
            Type::Union(fields) => fields.iter().fold(Fallible::Ok(TypeLayout::new()), |s, n| Ok(s?.union(n.get_layout()?)))?,
            Type::Pointer(_) => TypeLayout::of::<*const u8>(),
            Type::Array(element, option_len) => {
                if let Some(len) = option_len {
                    element.get_layout()?.repeat(*len)
                } else {
                    element.get_layout()?.into_flexible_array()
                }
            }
            Type::Reference(_) => TypeLayout::of::<*const u8>(),
            Type::Embed(type_resource) => type_resource.try_map(|r| r.get_layout())?,
            Type::Function(_) => TypeLayout::of::<&fn()>(),
        })
    }
}
#[derive(Clone, Copy, Builder)]
pub struct TypeLayout {
    size: usize,
    align: usize,
    tire: usize,
    flexible_size: usize,
}

impl TypeLayout {
    pub const fn default() -> Self {
        Self { size: 0, align: 1, tire: 0, flexible_size: 0 }
    }

    pub const fn set_size(self, size: usize) -> Self {
        Self { size, ..self }
    }

    pub const fn set_align(self, align: usize) -> Self {
        Self { align, ..self }
    }

    pub const fn set_tire(self, tire: usize) -> Self {
        Self { tire, ..self }
    }

    pub const fn set_flexible_flexible_size(self, flexible_size: usize) -> Self {
        Self { flexible_size, ..self }
    }
}
impl Into<Layout> for TypeLayout {
    fn into(self) -> Layout {
        Layout::from_size_align(self.size(), self.align()).unwrap()
    }
}
impl Default for TypeLayout {
    fn default() -> Self {
        Self::new()
    }
}
impl TypeLayout {
    pub const fn new() -> Self {
        Self { size: 0, align: 1, tire: 0, flexible_size: 0 }
    }

    pub const fn of<T: Sized>() -> Self {
        Self { size: size_of::<T>(), align: align_of::<T>(), tire: 0, flexible_size: 0 }
    }

    pub const fn repeat(&self, count: usize) -> Self {
        assert!(self.flexible_size == 0);
        let element_size = self.size.wrapping_add(self.align.wrapping_sub(1)) & !(self.align.wrapping_sub(1));
        Self { size: element_size * count, tire: if self.tire > 1 { self.tire } else { 1 }, ..*self }
    }

    pub const fn builder(&self) -> StructLayoutBuilder {
        StructLayoutBuilder { offset: 0, tire: self.tire, turple_size: self.size, align: self.align, flexible_size: self.flexible_size }
    }

    pub const fn union(&self, other: TypeLayout) -> Self {
        Self {
            size: if self.size > other.size { self.size } else { other.size },
            align: if self.align > other.align { self.align } else { other.align },
            tire: if self.tire > other.tire { self.tire } else { other.tire },
            flexible_size: if self.flexible_size > other.flexible_size { self.flexible_size } else { other.flexible_size },
        }
    }

    pub const fn size(self) -> usize {
        self.size
    }

    pub const fn align(&self) -> usize {
        self.align
    }

    pub const fn tire(&self) -> usize {
        self.tire
    }

    pub const fn flexible_size(&self) -> usize {
        self.flexible_size
    }

    pub const fn native(layout: Layout) -> TypeLayout {
        Self { size: layout.size(), align: layout.align(), ..Self::new() }
    }

    pub const fn into_flexible_array(&self) -> TypeLayout {
        Self {
            flexible_size: (self.size + (self.align - 1)) & !(self.align - 1),
            align: self.align,
            size: size_of::<usize>(),
            tire: if self.tire > 1 { self.tire } else { 1 },
        }
    }
}
#[derive(Clone, Copy, Builder)]
pub struct StructLayoutBuilder {
    pub offset: usize,
    pub tire: usize,
    pub turple_size: usize,
    pub align: usize,
    pub flexible_size: usize,
}
impl Default for StructLayoutBuilder {
    fn default() -> Self {
        Self::new()
    }
}
impl StructLayoutBuilder {
    pub const fn new() -> Self {
        Self { offset: 0, tire: 0, turple_size: 0, align: 1, flexible_size: 0 }
    }

    pub const fn extend(&self, other: TypeLayout) -> Self {
        let offset = self.turple_size.wrapping_add(self.align.wrapping_sub(1)) & !(other.align.wrapping_sub(1));
        assert!(self.flexible_size == 0);
        Self {
            offset,
            turple_size: offset.wrapping_add(other.size),
            align: if self.align > other.align { self.align } else { other.align },
            tire: self.tire + other.tire,
            flexible_size: other.flexible_size,
        }
    }

    pub const fn extend_compose(&self, bit_offset: usize, mask: usize, other: TypeLayout) -> Self {
        assert!(self.flexible_size == 0);
        assert!(other.flexible_size == 0);
        let extend_bits = size_of::<usize>()
            .wrapping_mul(8)
            .wrapping_sub(mask.leading_zeros() as usize)
            .wrapping_add(bit_offset)
            .wrapping_add(self.offset)
            .saturating_sub(self.turple_size);
        let extend_bytes = extend_bits.wrapping_add(7).wrapping_div(8);
        Self {
            offset: (self.offset.wrapping_add(other.size).wrapping_add(self.align.wrapping_sub(1)) & !(other.align.wrapping_sub(1))),
            turple_size: self.turple_size + extend_bytes,
            align: if self.align > other.align { self.align } else { other.align },
            tire: self.tire + other.tire,
            flexible_size: 0,
        }
    }

    pub const fn build(&self) -> TypeLayout {
        TypeLayout { size: self.turple_size, align: self.align, tire: self.tire, flexible_size: self.flexible_size }
    }

    pub const fn offset(&self) -> usize {
        self.offset
    }

    pub const fn tire(&self) -> usize {
        self.tire
    }

    pub const fn size(&self) -> usize {
        self.turple_size
    }

    pub const fn align(&self) -> usize {
        self.align
    }

    pub const fn flexible_size(&self) -> usize {
        self.flexible_size
    }
}
impl From<TypeLayout> for StructLayoutBuilder {
    fn from(layout: TypeLayout) -> Self {
        Self { offset: 0, tire: layout.tire, turple_size: layout.size, align: layout.align, flexible_size: layout.flexible_size }
    }
}
pub trait TypeDeclaration {
    type Impl;
    const LAYOUT: TypeLayout;
    const TYPE: Type;
}
pub trait TypeResource: Resource<Type> {
    fn alloc(&self) -> Fallible<NonNull<u8>>;
    fn alloc_unsized(&self, len: usize) -> Fallible<NonNull<u8>>;
    unsafe fn free(&self, oop: OOPRef) -> Fallible<()>;
    fn get_type(&self) -> Fallible<&Type>;
    fn get_layout(&self) -> Fallible<TypeLayout>;
    fn page_size(&self) -> Fallible<usize>;
    fn segment_size(&self) -> Fallible<usize>;
}
impl PartialEq for dyn TypeResource {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for dyn TypeResource {}
#[derive(Default, Debug, Builder, Getters, CopyGetters, PartialEq, Eq)]
pub struct FunctionType {
    #[getset(get = "pub")]
    #[builder(default)]
    pub dispatch: CowSlice<'static, Type>,
    #[getset(get = "pub")]
    #[builder(default)]
    pub return_type: Option<Type>,
    #[getset(get = "pub")]
    #[builder(default)]
    pub args: CowSlice<'static, Type>,
    #[getset(get = "pub")]
    #[builder(default)]
    pub va_arg: Option<Type>,
}

impl TypeDeclaration for () {
    type Impl = ();

    const LAYOUT: TypeLayout = TypeLayout::of::<()>();
    const TYPE: Type = Type::Tuple(Tuple::Normal(CowArc::Ref(&[])));
}

pub type ObjectFlag = u64;
pub mod flag {
    use crate::ObjectFlag;
    pub const WRITABLE: ObjectFlag = 0x1;
    pub const OPTIONAL: ObjectFlag = 0x2;
    pub const STATIC: ObjectFlag = 0x4;
    pub const CONST: ObjectFlag = 0x4;
    pub const EXTENABLE: ObjectFlag = 0x8;
}
pub struct ObjectField {
    pub field_type: Type,
    pub flag: ObjectFlag,
    pub scope: Arc<Scope>,
    pub const_value: Option<OOPRef>,
    pub offset: Option<usize>,
}
pub struct ObjectType {
    pub fields: Vec<Arc<dyn Resource<ObjectField>>>,
    pub flag: ObjectFlag,
    pub native: Option<Layout>,
}

#[repr(C)]
pub struct Reference<T: TypeDeclaration, R>(pub NonNull<T::Impl>, pub PhantomData<R>);
impl<T: TypeDeclaration + 'static, R: TypeResource + DefaultArc> TypeDeclaration for Reference<T, R> {
    type Impl = Pointer<T>;

    const LAYOUT: TypeLayout = TypeLayout::of::<NonNull<u8>>();
    const TYPE: Type = Type::Reference(MaybeDefinedResource::Factory(Self::init));
}
impl<T: TypeDeclaration + 'static, R: TypeResource + DefaultArc> Reference<T, R> {
    fn init() -> Fallible<CowArc<'static, dyn TypeResource>> {
        lazy_static! {
            static ref MAP: DashMap<TypeId, CowArc<'static, dyn TypeResource>> = DashMap::new();
        }
        let resource = MAP.entry(TypeId::of::<T>()).or_insert_with(|| CowArc::Owned(R::default_arc()));
        if !resource.get_state().is_loaded() {
            resource.upload(T::TYPE)?;
        }
        Ok(resource.clone())
    }
}
#[macro_export]
macro_rules! make_reference {
    ($name:ident,$ty:ty,$resource_type:ty) => {
        #[repr(C)]
        #[derive(Clone)]
        pub struct $name(pub std::ptr::NonNull<<$ty as jvm_core::TypeDeclaration>::Impl>);
        impl TypeDeclaration for $name {
            type Impl = jvm_core::Pointer<$ty>;

            const LAYOUT: jvm_core::TypeLayout = jvm_core::TypeLayout::of::<std::ptr::NonNull<u8>>();
            const TYPE: jvm_core::Type = jvm_core::Type::Reference(jvm_core::MaybeDefinedResource::Factory(Self::get));
        }
        impl $name {
            pub fn get() -> failure::Fallible<util::CowArc<'static, dyn jvm_core::TypeResource>> {
                lazy_static! {
                    static ref RESOURCE: std::sync::Arc<$resource_type> = {
                        let resource = <$resource_type as util::DefaultArc>::default_arc();
                        resource.upload(<$ty as jvm_core::TypeDeclaration>::TYPE).unwrap();
                        resource
                    };
                }
                Ok(util::CowArc::Owned((&*RESOURCE).clone()))
            }

            pub fn as_non_null(&self) -> std::ptr::NonNull<<$ty as jvm_core::TypeDeclaration>::Impl> {
                self.0
            }

            pub fn as_pointer(&self) -> jvm_core::Pointer<$ty> {
                Pointer::new(self.0.cast())
            }
        }
    };
}
#[repr(C)]
pub struct Direct<T: TypeDeclaration>(pub T::Impl);

impl<T: TypeDeclaration> DerefMut for Direct<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: TypeDeclaration> std::ops::Deref for Direct<T> {
    type Target = T::Impl;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: TypeDeclaration> TypeDeclaration for Direct<T> {
    type Impl = T::Impl;

    const LAYOUT: TypeLayout = T::LAYOUT;
    const TYPE: Type = T::TYPE;
}

#[repr(C)]
pub struct Pointer<T: TypeDeclaration>(NonNull<u8>, PhantomData<T::Impl>);

impl<T: TypeDeclaration> Clone for Pointer<T> {
    fn clone(&self) -> Self {
        Self(self.0, self.1)
    }
}

impl<T: TypeDeclaration> MoveIntoObject for Pointer<T> {
    fn set<'l>(self, offset: usize, object_builder: &crate::ObjectBuilder<'l>, token: &mut ghost_cell::GhostToken<'l>) {
        object_builder.borrow_mut(token).receive_at(offset).write(self.0);
    }
}

impl<T: TypeDeclaration> Pointer<T> {
    pub fn new(ptr: NonNull<u8>) -> Self {
        Self(ptr, PhantomData)
    }

    pub fn as_non_null(&self) -> NonNull<T::Impl> {
        self.0.cast()
    }

    pub fn as_ptr(&self) -> *const T::Impl {
        self.as_non_null().as_ptr()
    }

    pub fn as_ptr_mut(&self) -> *mut T::Impl {
        self.as_non_null().as_ptr()
    }

    pub unsafe fn as_ref(&self) -> &T::Impl {
        self.as_non_null().as_ref()
    }

    pub unsafe fn as_ref_mut(&mut self) -> &mut T::Impl {
        self.as_non_null().as_mut()
    }
}

impl<T: TypeDeclaration> TypeDeclaration for Pointer<T> {
    type Impl = NonNull<u8>;

    const LAYOUT: TypeLayout = TypeLayout::of::<NonNull<u8>>();
    const TYPE: Type = Type::Pointer(CowArc::Ref(inline_const! {<T:TypeDeclaration>[&'static Type]&T::TYPE}));
}

#[repr(C)]
pub struct Embed<T: TypeDeclaration>(pub T::Impl);
#[repr(C)]
#[derive(Default, Clone)]
pub struct Native<T>(pub T);

impl<T> Deref for Native<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> TypeDeclaration for Native<T> {
    type Impl = T;

    const LAYOUT: TypeLayout = TypeLayout::of::<T>();
    const TYPE: Type = Type::Native(Layout::new::<T>());
}
#[repr(C)]
#[derive(Default, Clone)]
pub struct Aligned<const ALIGN: usize>();

impl<const ALIGN: usize> TypeDeclaration for Aligned<ALIGN> {
    type Impl = ();

    const LAYOUT: TypeLayout = TypeLayout { size: 0, align: ALIGN, tire: 0, flexible_size: 0 };
    const TYPE: Type = Type::Native(unsafe { Layout::from_size_align_unchecked(0, ALIGN) });
}
#[repr(C)]
pub struct UnsizedArray<T: TypeDeclaration>(pub usize, pub UnsafeCell<PhantomData<[T::Impl]>>);

unsafe impl<T: TypeDeclaration> Send for UnsizedArray<T> {}
unsafe impl<T: TypeDeclaration> Sync for UnsizedArray<T> {}

impl<T: TypeDeclaration> UnsizedArray<T> {
    pub fn len(&self) -> usize {
        self.0
    }

    pub fn as_non_null(&self) -> NonNull<[T::Impl]> {
        NonNull::from(self.as_slice())
    }

    pub fn as_slice(&self) -> &[T::Impl] {
        unsafe { self.as_ptr().as_ref().unwrap() }
    }

    pub fn as_slice_mut(&mut self) -> &mut [T::Impl] {
        unsafe { self.as_ptr_mut().as_mut().unwrap() }
    }

    pub fn as_ptr(&self) -> *const [T::Impl] {
        std::ptr::slice_from_raw_parts((self.1.get()).cast::<T::Impl>(), self.len())
    }

    pub fn as_ptr_mut(&self) -> *mut [T::Impl] {
        std::ptr::slice_from_raw_parts_mut((self.1.get()).cast::<T::Impl>(), self.len())
    }
}

impl<T: TypeDeclaration> TypeDeclaration for UnsizedArray<T> {
    type Impl = Self;

    const LAYOUT: TypeLayout = T::LAYOUT.into_flexible_array();
    const TYPE: Type = Type::Array(CowArc::Ref(inline_const! {<T:TypeDeclaration>[&'static Type]&T::TYPE}), None);
}

#[repr(C)]
pub struct Array<T: TypeDeclaration, const LEN: usize>(pub [T::Impl; LEN]);

impl<T: TypeDeclaration, const LEN: usize> TypeDeclaration for Array<T, LEN> {
    type Impl = [T::Impl; LEN];

    const LAYOUT: TypeLayout = TypeLayout::of::<NonNull<T::Impl>>();
    const TYPE: Type = Type::Array(CowArc::Ref(inline_const! {<T:TypeDeclaration>[&'static Type]&T::TYPE}), Some(LEN));
}
#[repr(C)]
pub struct Slice<T: TypeDeclaration>(pub Pointer<T>, pub usize);

impl<T: TypeDeclaration> TypeDeclaration for Slice<T> {
    type Impl = Self;

    const LAYOUT: TypeLayout = TypeLayout::of::<Self>();
    const TYPE: Type = Type::Tuple(Tuple::Normal(CowArc::Ref(
        inline_const! {<T:TypeDeclaration>[&'static [Type]]&[Type::Pointer(CowArc::Ref(inline_const! {<T:TypeDeclaration>[&'static Type]&T::TYPE})),Type::Int(IntKind::Usize)]},
    )));
}
#[repr(C)]
pub struct Function<T: TypeDeclaration>(pub NonNull<u8>, pub PhantomData<*mut T>);
