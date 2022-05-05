use async_trait::async_trait;
use crossbeam::atomic::AtomicCell;
use failure::{format_err, Error, Fail, Fallible};
use getset::Getters;
use std::{
    fmt::Debug,
    hash::Hash,
    sync::{Arc, Weak},
};
use tokio::sync::{watch, OnceCell};
use util::CowArc;

use crate::Component;

#[derive(Fail, Debug)]
pub enum ResourceError {
    #[fail(display = "Resource is not loaded")]
    NotLoaded,
    #[fail(display = "Resource is not initialized")]
    NotInitialized,
    #[fail(display = "Resource is dead")]
    Dead,
    #[fail(display = "operation is not supported")]
    Unsupported,
    #[fail(display = "Other error: {:#?}", _0)]
    Other(#[cause] Error),
}
pub trait ResourceFactory<T> {
    type ResourceImpl: Resource<T> + ?Sized;
    fn define(&self) -> Fallible<Arc<Self::ResourceImpl>>;
    fn create(&self, input: T) -> Fallible<Arc<Self::ResourceImpl>> {
        let resource = self.define()?;
        self.upload(&*resource, input)?;
        Ok(resource)
    }
    fn upload(&self, resource: &Self::ResourceImpl, input: T) -> Fallible<()>;
}
#[derive(Debug)]
pub enum ResourceState {
    Defined,
    Loaded,
    Ready,
    Dead,
    Error(Error),
}
impl Clone for ResourceState {
    fn clone(&self) -> Self {
        match self {
            ResourceState::Defined => ResourceState::Defined,
            ResourceState::Loaded => ResourceState::Loaded,
            ResourceState::Ready => ResourceState::Ready,
            ResourceState::Dead => ResourceState::Dead,
            ResourceState::Error(e) => ResourceState::Error(format_err!("resource is in a error state.\nCause by:\n{:#?}", e)),
        }
    }
}
/// 虚拟机的通用抽象资源对象
/// `Resource<T>`代表对`T`类型的数据的后续处理结果的跟踪
/// 可以实现先定义后加载
/// 部分实现支持更新
/// - [] fearure:Resource处理过程异步化
pub trait Resource<T>: Component {
    fn drop(&self) -> Fallible<()> {
        Err(format_err!("not support"))
    }
    fn update_reference(&self) -> Fallible<()> {
        Ok(())
    }
    fn upload(&self, _value: T) -> Fallible<()> {
        Err(format_err!("not support"))?
    }
    fn get_version(&self) -> usize {
        0
    }
    fn ready(&self) -> bool {
        match self.get_state() {
            ResourceState::Ready => true,
            _ => false,
        }
    }
    fn wait_for_ready(&self) -> Fallible<()> {
        Err(format_err!("not support"))?
    }
    fn get_state(&self) -> ResourceState;
}
#[async_trait]
pub trait AsyncResource<T>: Component {
    type SubLifetime: PartialOrd;
    type Action;
    async fn upload(&self, value: T) -> Fallible<()>;
    fn ready(&self) -> bool;
    async fn readyed(&self) -> bool;
}

pub enum AsyncResourceLifetime {
    Defined,
    Loaded,
    Ready,
    Dead,
    Error,
}
pub enum AsyncAction<A, T> {
    ToState(AsyncResourceLifetime),
    Custom(A),
    Upload(Box<T>),
}
#[derive(Builder, Getters)]
#[builder(pattern = "owned")]
#[getset(get = "pub")]
pub struct AsyncResourceFrontEnd<T, S, A, M> {
    state: watch::Receiver<S>,
    last_state: AtomicCell<S>,
    action_sender: watch::Sender<AsyncAction<A, T>>,
    result: OnceCell<Fallible<T>>,
    metadata: M,
}
pub enum MaybeDefinedResource<T: ?Sized + 'static> {
    Defined(CowArc<'static, T>),
    Factory(fn() -> Fallible<CowArc<'static, T>>),
}

impl<T: ?Sized + 'static> Hash for MaybeDefinedResource<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            MaybeDefinedResource::Defined(d) => d.as_ptr().hash(state),
            MaybeDefinedResource::Factory(f) => f.hash(state),
        }
    }
}

impl<T: ?Sized> Eq for MaybeDefinedResource<T> {}

impl<T: ?Sized> PartialEq for MaybeDefinedResource<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Defined(l0), Self::Defined(r0)) => CowArc::as_ptr(l0) == CowArc::as_ptr(r0),
            (Self::Factory(l0), Self::Factory(r0)) => *l0 == *r0,
            _ => false,
        }
    }
}

impl<T: ?Sized> MaybeDefinedResource<T> {
    pub fn map<R>(&self, f: impl FnOnce(&CowArc<T>) -> R) -> Fallible<R> {
        self.try_map(|i| Ok(f(i)))
    }

    pub fn try_map<R>(&self, f: impl FnOnce(&CowArc<T>) -> Fallible<R>) -> Fallible<R> {
        match self {
            MaybeDefinedResource::Defined(r) => f(r),
            MaybeDefinedResource::Factory(factory) => f(&factory()?),
        }
    }
}

impl<T: ?Sized + Debug> Debug for MaybeDefinedResource<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Defined(arg0) => f.debug_tuple("Resource").field(arg0).finish(),
            Self::Factory(arg0) => f.debug_tuple("ResourceFactory").field(arg0).finish(),
        }
    }
}

impl<T: ?Sized> Clone for MaybeDefinedResource<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Defined(arg0) => Self::Defined(arg0.clone()),
            Self::Factory(arg0) => Self::Factory(*arg0),
        }
    }
}

pub struct ResourceReference<T> {
    pub inner: Arc<dyn Resource<T>>,
    pub version: usize,
}
pub struct ResourceWeakReference<T> {
    pub inner: Weak<dyn Resource<T>>,
    pub version: usize,
}
pub trait ResourceConverter<F, T> {
    fn covert(&self, input: ResourceReference<F>) -> Fallible<Arc<dyn Resource<T>>>;
}
pub trait ResourcePipe<F, T> {
    fn receive(&self, input: ResourceReference<F>) -> Fallible<Vec<Arc<dyn Resource<T>>>>;
}
impl ResourceState {
    pub fn is_loaded(&self) -> bool {
        match self {
            Self::Defined => false,
            _ => true,
        }
    }

    pub fn is_ready(&self) -> bool {
        match self {
            Self::Ready => true,
            _ => false,
        }
    }
}
