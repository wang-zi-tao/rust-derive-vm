#![feature(inline_const)]
#![feature(generic_const_exprs)]
#![feature(const_fn_trait_bound)]
#![feature(inherent_associated_types)]
use runtime::instructions::bootstrap as b;
use runtime_extra as e;
use std::ptr::NonNull;

use vm_core::*;
#[macro_use]
extern crate runtime_derive;
#[derive(TypeDeclaration)]
pub enum TestEnum {
    Variant1,
    Variant2(e::F32),
    Variant3(e::U64),
    Variant4 { f: e::Bool },
}
type TestEnumRef = NonNull<TestEnumImpl>;
#[derive(TypeDeclaration)]
pub struct Test<T: TypeDeclaration> {
    pub inner1: Pointer<T>,
    pub inner2: Pointer<T>,
}
#[derive(TypeDeclaration)]
pub struct Vec4 {
    pub x: e::F32,
    pub y: e::F32,
    pub z: e::F32,
    pub w: e::F32,
}
type Vec4Ref = NonNull<Vec4Impl>;

impl Vec4 {
    type LocateX = Vec4LocateX;
}
make_instruction! {
    Vec4LocateX->fn(this:Pointer<Vec4>)->(field:Pointer<e::F32>){
        entry:{
            %field = b::LocateField<Vec4::TYPE,0>(%this);
        }
    }
}
#[derive(TypeDeclaration)]
pub enum TestOption<T: TypeDeclaration>
where
    T::Impl: Sized,
{
    Some(T),
    None,
}
struct Str {
    hash: usize,
    data: [u8],
}
type BoxStr = Box<Str>;
