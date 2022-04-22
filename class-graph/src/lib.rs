#![feature(default_free_fn)]
#![feature(try_trait)]
#![feature(exact_size_is_empty)]
#![feature(iter_order_by)]
#![feature(coerce_unsized)]
extern crate classfile;
use class_loader::{BootstrapClassSet, ClassLoader};
pub mod annotations;
use classfile::ClassFile;
use dashmap::DashSet;
use jvm_core::OOPMemoryTrait;
use util;
#[macro_use]
extern crate util_derive;
#[macro_use]
extern crate getset;
use jvm_core::{Module, RuntimeTrait};
use std::{fmt::Debug, marker::PhantomData, sync::Arc};
use util::Key;

pub(crate) mod class;
pub(crate) mod class_loader;
pub(crate) mod executable;
pub(crate) mod field;
pub(crate) mod flags;
pub(crate) mod java_type;
pub(crate) mod member;
pub(crate) mod modifiers;
pub(crate) mod package;
pub(crate) mod signature;
// pub(crate) mod r#type;
pub(crate) mod generate_type;
pub(crate) mod verification;

use failure::{self, format_err};
use util::LazyDynRef;
#[macro_use]
extern crate failure_derive;
struct JVMContext {}
type JavaByteCodeRuntime = dyn RuntimeTrait<ClassFile, JVMContext>;
pub struct ClassGraph {
    memory: LazyDynRef<'static, dyn OOPMemoryTrait>,
    runtime: LazyDynRef<'static, JavaByteCodeRuntime>,

    bootstrap_class_loader: Arc<ClassLoader>,
    bootstrap_class_set: BootstrapClassSet,

    class_loader_set: DashSet<Arc<ClassLoader>>,
}
impl Debug for ClassGraph {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
impl ClassGraph {
    fn get_bootstrap_class_loader(&self) -> &Arc<ClassLoader> {
        &self.bootstrap_class_loader
    }
}
impl Module for ClassGraph {}
// impl ClassGraphTrait for ClassGraph {
// fn get_bootstrap_class_loader(&self) -> Result<&dyn ClassLoaderTrait> {
// Ok(self.bootstrap_class_loader.as_ref())
// }
//
// fn create_bootstrap_class_loader(&self) -> Result<Arc<dyn jvm_core::ClassLoaderTrait>> {
// ClassLoader::create_bootstrap_class_loader().map(|c| c as Arc<dyn ClassLoaderTrait>)
// }
// }
