use std::sync::Arc;

use crate::{Module, ObjectRef, Resource, ResourceConverter, ResourceFactory};
use failure::{format_err, Fallible};

pub trait ExecutableResourceTrait<T>: Resource<T> {
    fn get_object(&self) -> Fallible<ObjectRef>;
}
/// 特定语言或中间码的运行时系统
pub trait RuntimeTrait<M, I>: ResourceConverter<M, I> + Module
where
    I: ExecutableResourceTrait<M>,
    M: 'static,
{
}
pub trait DynRuntimeTrait<M> {
    fn define_dyn(&self) -> Fallible<Arc<dyn ExecutableResourceTrait<M>>>;
    fn create_dyn(&self, input: M) -> Fallible<Arc<dyn ExecutableResourceTrait<M>>>;

    fn upload_dyn(&self, resource: &dyn ExecutableResourceTrait<M>, input: M) -> Fallible<()>;
}
