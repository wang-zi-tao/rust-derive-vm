#![feature(const_weak_new)]
// pub mod bytecode;
#[macro_use]
extern crate util_derive;
#[macro_use]
extern crate getset;
pub mod code;
pub mod instructions;
pub mod interpreter;
pub mod mem;
pub mod method;

pub use failure as _failure;
pub use util as _util;

use smallvec::SmallVec;
use std::{
    any::{Any, TypeId},
    fmt::{Debug, Formatter},
    marker::PhantomData,
    sync::{Arc, RwLock},
};
use util::AsAny;

use dashmap::{mapref::entry::Entry, DashMap};
use failure::{format_err, Fallible};
use vm_core::{Component, ExecutableResourceTrait, Module, Resource, ResourceFactory, ResourceState, RuntimeTrait, SymbolRef};
pub trait RuntimeFilter: Module {
    fn get_input_type_id(&self) -> TypeId;
    fn consume<'l>(&'l self, esource_input: Arc<dyn Any + Send + Sync>) -> Fallible<SmallVec<[(Arc<dyn Any + Send + Sync>, &'l dyn RuntimeFilter); 1]>>;
}
#[derive(Debug)]
pub struct RuntimePipeGraph {
    enter_point: DashMap<TypeId, Arc<dyn RuntimeFilter>>,
}
#[derive(Debug, AsAny)]
pub struct ExecutableResourceImpl {
    inner: RwLock<ExecutableResourceInner>,
    pub symbol_ref: SymbolRef,
}
impl<T> ExecutableResourceTrait<T> for ExecutableResourceImpl {
    fn get_object(&self) -> Fallible<vm_core::ObjectRef> {
        todo!()
    }
}
#[derive(Debug)]
struct ExecutableResourceInner {
    state: ResourceState,
}
impl ExecutableResourceImpl {
    pub fn new() -> Self {
        Self { inner: RwLock::new(ExecutableResourceInner { state: ResourceState::Defined }), symbol_ref: SymbolRef::new() }
    }

    fn get_state(&self) -> vm_core::ResourceState {
        self.inner.read().unwrap().state.clone()
    }
}
impl Component for ExecutableResourceImpl {}
impl<T> Resource<T> for ExecutableResourceImpl {
    fn get_state(&self) -> ResourceState {
        ExecutableResourceImpl::get_state(self)
    }
}
impl Module for RuntimePipeGraph {}
impl RuntimePipeGraph {
    fn upload_for<'l, T: AsAny + 'static>(&'l self, resource: &dyn ExecutableResourceTrait<T>, input: T, enter_point: &'l dyn RuntimeFilter) -> Fallible<()> {
        let _resource: &ExecutableResourceImpl = resource.as_any().downcast_ref().ok_or_else(|| format_err!("wrong type"))?;
        let mut tasks: Vec<(Arc<dyn Any + Send + Sync>, &'l dyn RuntimeFilter)> = Vec::new();
        tasks.push((Arc::new(input).as_any_arc(), enter_point));
        while let Some((value, pipe)) = tasks.pop() {
            let out = pipe.consume(value)?;
            tasks.extend(out.iter().cloned());
        }
        Ok(())
    }

    pub fn get_enter_point<T: AsAny + Send + Sync + 'static>(&self) -> Option<Arc<dyn RuntimeFilter>> {
        self.enter_point.get(&TypeId::of::<T>()).map(|e| {
            let enter_point: &Arc<dyn RuntimeFilter> = &*e;
            enter_point.clone()
        })
    }

    pub fn add_pipe(&self, pipe: Arc<dyn RuntimeFilter>) -> Fallible<()> {
        match self.enter_point.entry(pipe.get_input_type_id()) {
            Entry::Vacant(v) => {
                v.insert(pipe);
            }
            Entry::Occupied(o) => {
                Err(format_err!("the pipe of type {:?} has aready be registered.\nold:{:#?}\nnew:{:#?}", pipe.get_input_type_id(), o.get(), pipe))?;
            }
        }
        Ok(())
    }
}
impl<T: AsAny + Send + Sync + 'static> ResourceFactory<T> for RuntimePipeGraph {
    type ResourceImpl = dyn ExecutableResourceTrait<T>;

    fn define(&self) -> Fallible<std::sync::Arc<Self::ResourceImpl>> {
        Ok(Arc::new(ExecutableResourceImpl::new()))
    }

    fn upload(&self, resource: &Self::ResourceImpl, input: T) -> Fallible<()> {
        let enter_point = self.enter_point.get(&resource.type_id()).ok_or_else(|| format_err!("the runtime pipe is not defined.resource:{:#?}", resource))?;
        Self::upload_for(self, resource, input, &**enter_point)
    }
}
impl<T: Send + Sync + util::AsAny> RuntimeTrait<T> for RuntimePipeGraph {}
pub struct RuntimePipe<T> {
    graph: Arc<RuntimePipeGraph>,
    enter_point: Arc<dyn RuntimeFilter>,
    phantom_data: PhantomData<T>,
}
impl<T> Debug for RuntimePipe<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("RuntimePipe").field("graph", &self.graph).field("name", &self.enter_point).finish()
    }
}
impl<T: AsAny + Send + Sync + 'static> Module for RuntimePipe<T> {}
impl<T: AsAny + Send + Sync + 'static> ResourceFactory<T> for RuntimePipe<T> {
    type ResourceImpl = dyn ExecutableResourceTrait<T>;

    fn define(&self) -> Fallible<std::sync::Arc<Self::ResourceImpl>> {
        Ok(Arc::new(ExecutableResourceImpl::new()))
    }

    fn upload(&self, resource: &Self::ResourceImpl, input: T) -> Fallible<()> {
        self.graph.upload_for(resource, input, &*self.enter_point)
    }
}
impl<T: Send + Sync + util::AsAny> RuntimeTrait<T> for RuntimePipe<T> {}
