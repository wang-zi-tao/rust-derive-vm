use std::{
    mem::{size_of},
    ops::Add,
    ptr::NonNull,
    sync::{Arc, RwLock},
};

use crossbeam::atomic::AtomicCell;
use jvm_core::{OOPTrait};

use crate::RegistedType;

#[derive(Debug, Clone)]
pub struct Metadata {
    pub(crate) ty: Arc<RegistedType>,
    pub(crate) object: Box<[u8]>,
}
impl Metadata {
    pub fn new(ty: Arc<RegistedType>, object: Box<[u8]>) -> Self {
        Self { ty, object }
    }

    pub(crate) unsafe fn as_raw(&self) -> NonNull<u8> {
        NonNull::new_unchecked(NonNull::from(&self.object).as_ptr().cast::<u8>().add(size_of::<TypeMetadata>()))
    }
}
#[derive(Default)]
pub(crate) struct MetadataList {
    pub(crate) metas: RwLock<Vec<Metadata>>,
}
pub(crate) struct GCMetadata {
    pub(crate) index: AtomicCell<usize>,
}
pub(crate) struct TypeMetadata {
    pub(crate) ty: Arc<RegistedType>,
    pub(crate) gc: GCMetadata,
    pub(crate) tire: usize,
}

impl TypeMetadata {
    pub unsafe fn from_raw<'l>(ptr: NonNull<u8>) -> NonNull<Self> {
        NonNull::new_unchecked(ptr.cast::<Self>().as_ptr().offset(-1))
    }
}
