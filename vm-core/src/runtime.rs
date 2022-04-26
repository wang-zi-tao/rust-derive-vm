use crate::{Module, ObjectRef, Resource, ResourceFactory};
use failure::Fallible;

pub trait ExecutableResourceTrait<T>: Resource<T> {
    fn get_object(&self) -> Fallible<ObjectRef>;
}
/// 特定语言或中间码的运行时系统
pub trait RuntimeTrait<M>: ResourceFactory<M> + Module
where
    Self::ResourceImpl: ExecutableResourceTrait<M>,
{
}
