pub(crate) mod buffer;
pub(crate) mod object;
pub(crate) mod ty;

use std::{ptr::NonNull, sync::Arc};

use failure::{Fallible};



use crate::{Module, OOPTrait, ResourceFactory, Singleton, Type, TypeResource};
pub trait MemoryTrait: Module + ResourceFactory<Type>
where
    Self::ResourceImpl: Sized + TypeResource,
{
    fn alloc(&self, type_layout: &Arc<dyn TypeResource>) -> Fallible<NonNull<u8>>;
    fn alloc_unsized(&self, type_layout: &Arc<dyn TypeResource>, size: usize) -> Fallible<Box<dyn OOPTrait>>;
}
pub trait StaticMemoryTrait: MemoryTrait + Singleton
where
    Self::ResourceImpl: Sized + TypeResource,
{
}

// pub trait OOPMemoryTrait: MemoryTrait {
//     // fn create_layout_builder(&self) -> Box<dyn LayoutBuilderTrait>;
// }
