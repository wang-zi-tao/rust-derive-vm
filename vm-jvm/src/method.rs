use jvm_core::SymbolRef;

use crate::{metadata::Metadata, Object};
use std::{
    ptr::Unique,
    sync::{Arc, RwLock},
};
#[derive(Debug)]
pub enum InvocationKind {
    Direct(Unique<u8>),
    Virtual(i32),
}
impl InvocationKind {
    // pub unsafe fn transform<T>(&self, object: &Object) -> *const T {
    // match self {
    // InvocationKind::Virtual(offset) => (object.mate_data() as *const Metadata)
    // .cast::<u8>()
    // .offset(*offset as isize)
    // .cast(),
    // InvocationKind::Direct(position) => position.as_ptr().cast(),
    // }
    // }

    // pub unsafe fn get_static<T>(&self) -> *const T {
    // match self {
    // InvocationKind::Virtual(_offset) => panic!(),
    // InvocationKind::Direct(position) => position.as_ptr().cast(),
    // }
    // }
}
#[derive(Debug)]
pub struct MethodLayout {
    invocation_kind: InvocationKind,
    code_associate_stub_pool: Option<SymbolRef>,
}
impl MethodLayout {}
