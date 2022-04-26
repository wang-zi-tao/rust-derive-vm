use crate as runtime_extra;
use ghost_cell::GhostToken;
use std::mem::{align_of, size_of};
use util::CowArc;
use vm_core::{FloatKind, IntKind, MoveIntoObject, ObjectBuilder, Pointer, Tuple, Type, TypeDeclaration, TypeLayout};

macro_rules! wrap_type {
    ($name:ident,$rust_type:ty,$ty:expr) => {
        #[derive(Clone, Copy, PartialEq, PartialOrd)]
        #[repr(C)]
        pub struct $name(pub $rust_type);
        impl $name {
            pub fn get(&self) -> $rust_type {
                self.0
            }
        }
        impl TypeDeclaration for $name {
            type Impl = Self;

            const LAYOUT: TypeLayout = TypeLayout::of::<$rust_type>();
            const TYPE: Type = $ty;
        }
        impl $name {
            pub const ALIGN: usize = align_of::<Self>();
            pub const SIZE: usize = size_of::<Self>();
            pub const TYPE: Type = $ty;
        }
        impl MoveIntoObject for $name {
            fn set<'l>(self, offset: usize, object_builder: &ObjectBuilder<'l>, token: &mut GhostToken<'l>) {
                object_builder.borrow_mut(token).receive_at(offset).write(self.0);
            }
        }
    };
}

wrap_type!(Unit, (), Type::Tuple(Tuple::Normal(CowArc::Ref(&[]))));

macro_rules! declare_int_type {
    ($name:ident,$int_kind:ident,$rust_type:ident) => {
        wrap_type!($name, $rust_type, Type::Int(IntKind::$int_kind));
    };
}
declare_int_type!(Bool, Bool, bool);
declare_int_type!(U8, U8, u8);
declare_int_type!(U16, U16, u16);
declare_int_type!(U32, U32, u32);
declare_int_type!(U64, U64, u64);
declare_int_type!(Usize, Usize, usize);
declare_int_type!(I8, I8, i8);
declare_int_type!(I16, I16, i16);
declare_int_type!(I32, I32, i32);
declare_int_type!(I64, I64, i64);
declare_int_type!(Isize, Isize, isize);

macro_rules! declare_float_type {
    ($name:ident,$float_kind:ident,$rust_type:ident) => {
        wrap_type!($name, $rust_type, Type::Float(FloatKind::$float_kind));
    };
}
declare_float_type!(F32, F32, f32);
declare_float_type!(F64, F64, f64);

#[derive(TypeDeclaration, Clone)]
#[make_type(make_instruction, tag_start = 0)]

pub enum NullableOption<T: vm_core::TypeDeclaration>
where
    T::Impl: Sized,
{
    Some(T),
    None,
}
impl<T: vm_core::TypeDeclaration> Clone for NullableOptionImpl<T>
where
    [u8; nullable_option_layout::<T>().size()]: Sized,
{
    fn clone(&self) -> Self {
        Self(self.0, self.1)
    }
}

#[derive(TypeDeclaration)]
#[make_type(make_instruction, tag_start = 0)]
pub enum NullablePointer<T: vm_core::TypeDeclaration> {
    Some(Pointer<T>),
    None,
}

impl<T: vm_core::TypeDeclaration> Clone for NullablePointerImpl<T>
where
    [u8; nullable_pointer_layout::<T>().size()]: Sized,
{
    fn clone(&self) -> Self {
        Self(self.0, self.1)
    }
}

#[derive(TypeDeclaration, Clone)]
#[make_type(make_instruction)]
pub enum UnCompressedOption<T: vm_core::TypeDeclaration>
where
    T::Impl: Sized,
{
    Some(T),
    None,
}
