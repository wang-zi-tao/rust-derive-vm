use util::{AtomicCell, AtomicLazyArc};

use crate::field::{FieldLayout, FieldLayoutKind};
use std::{
    fmt::Debug,
    hash::Hash,
    sync::{Arc, RwLock},
};
use vm_core::{Component, TypeLayoutTrait};
// #[derive(Debug, AsAny, Getters)]
// pub struct TypeLayout {
// size: u32,
// tire: u32,
// metadata: Arc<RwLock<AssociateStubPool>>,
// metadata_layout: Vec<Arc<TypeLayout>>,
// fields: Vec<Arc<FieldLayout>>,
// implements: Vec<Arc<ImplementLayout>>,
// memory_map: Vec<u32>,
// }
// impl TypeLayout {
// pub fn size(&self) -> u32 {
// self.size
// }
//
// pub fn tire(&self) -> u32 {
// self.tire
// }
//
// pub fn memory_map(&self) -> &[u32] {
// &*self.memory_map
// }
//
// pub fn new_class_layout(_super_layout: Option<&TypeLayout>) -> Result<Self> {
// todo!()
// }
//
// pub fn get_associate_stub_pool_mut(&mut self) -> &mut Arc<RwLock<AssociateStubPool>> {
// &mut self.metadata
// }
//
// pub fn get_associate_stub_pool(&self) -> &Arc<RwLock<AssociateStubPool>> {
// &self.metadata
// }
// }
// impl TypeLayoutTrait for TypeLayout {}
// impl PartialEq<TypeLayout> for TypeLayout {
// fn eq(&self, _: &TypeLayout) -> bool {
// todo!()
// }
// }
// impl Eq for TypeLayout {}
// impl Hash for TypeLayout {
// fn hash<H>(&self, _: &mut H)
// where
// H: std::hash::Hasher,
// {
// todo!()
// }
// }
pub struct JavaExecutableLayout {
    executable: Box<[i8]>,
}
// #[derive(Debug)]
// pub struct ImplementLayout {
// offset: usize,
// tire: u32,
// }
// impl ImplementLayout {
// pub fn new(class: &mut TypeLayout, interface: &TypeLayout) -> Self {
// let tire = class.tire + interface.tire;
// class.tire += interface.tire;
// Self { offset: 0, tire }
// }
// }

// #[derive(Debug)]
// pub enum Type {
// F32,
// F64,
// Int(usize),
// Static(AssociateStubRef),
// MetaData(Box<[Type]>),
// Const(Box<[u8]>, Box<Type>),
// Tuple(Box<[(Type, usize, usize)]>),
// Enum(Box<[(Type, usize, usize)]>),
// Pointer(Box<Type>),
// Array(Box<Type>, usize),
// Reference(Arc<GCObjectType>, Arc<dyn ReferenceKind>),
// ReferenceOrEmbed(Arc<GCObjectType>, Arc<dyn ReferenceKind>),
// Embed(Arc<GCObjectType>),
// }
// pub struct GCObjectType {
// size: u32,
// tire: u32,
// metadata: Arc<RwLock<AssociateStubPool>>,
// metadata_layout: Vec<MetaData>,
//
// raw_type: AtomicLazyArc<Type>,
// usage: Vec<Arc<GCObjectType>>,
// assignable: Vec<Arc<GCObjectType>>,
// stub: EmbedNodeStub,
// }
// impl Debug for GCObjectType {
// fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
// f.debug_struct("GCObjectType")
// .field("size", &self.size)
// .field("tire", &self.tire)
// .finish()
// }
// }
// impl GCObjectType {
// pub fn get_node_stub(&self) -> &EmbedNodeStub {
// &self.stub
// }
// }
// impl Hash for GCObjectType {
// fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
// state.write_usize(self as *const Self as usize)
// }
// }
// impl PartialEq for GCObjectType {
// fn eq(&self, other: &Self) -> bool {
// (self as *const Self) == (other as *const Self)
// }
// }
// impl Eq for GCObjectType {}
// pub struct MetaData {
// layout: Arc<GCObjectType>,
// object: GlobalOOP,
// }
// pub struct ObjectLayout {
//     type_layout: Arc<TypeLayout>,
//     fields: Vec<FieldLayout>,
// }
