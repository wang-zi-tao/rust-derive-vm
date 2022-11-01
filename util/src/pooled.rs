use crate::{BKDRHash, BuildBKDRHash};
use core::ops::Deref;
use dashmap::{mapref::entry::Entry, DashMap};
use std::{
    borrow::Borrow,
    fmt::{Debug, Display, Formatter},
    hash::{Hash, Hasher},
    ptr::NonNull,
    sync::{
        atomic::{AtomicIsize, Ordering},
        Arc, Weak,
    },
};
struct Inner<T: Hash + Eq + Sync + Send> {
    pub rc: AtomicIsize,
    pub data: T,
    pub hash: u64,
    pub pool: Weak<RawPool<T>>,
}
impl<T: Hash + Eq + Sync + Send> Inner<T> {
    fn new(data: T, pool: &Pool<T>) -> Self {
        let mut hasher = BKDRHash::new();
        data.hash(&mut hasher);
        Inner { data, rc: AtomicIsize::new(1), hash: hasher.finish(), pool: Arc::downgrade(pool) }
    }
}
impl<T: Hash + Eq + Sync + Send + Debug> Debug for Inner<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Inner").field("rc", &self.rc).field("hash", &self.hash).field("data", &self.data).finish()
    }
}
impl<T: Hash + Eq + Sync + Send> PartialEq for Inner<T> {
    fn eq(&self, other: &Self) -> bool {
        other.data == self.data
    }
}
impl<T: Hash + Eq + Sync + Send> Eq for Inner<T> {}
impl<T: Hash + Eq + Sync + Send> Hash for Inner<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}
impl<T: Hash + Eq + Sync + Send> Deref for Inner<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
unsafe impl<T: Hash + Eq + Sync + Send> Sync for Inner<T> {}
pub struct Pooled<T: Hash + Eq + Sync + Send> {
    inner: NonNull<Inner<T>>,
}
impl<T: Hash + Eq + Sync + Send> Pooled<T> {
    unsafe fn new(inner: &Inner<T>) -> Pooled<T> {
        inner.rc.fetch_add(1, Ordering::Relaxed);
        Pooled { inner: NonNull::from(inner) }
    }

    unsafe fn from_raw(ptr: NonNull<Inner<T>>) -> Pooled<T> {
        Pooled { inner: ptr }
    }

    fn get_inner(&self) -> &Inner<T> {
        unsafe { self.inner.as_ref() }
    }
}
impl<T: Hash + Eq + Sync + Send> Hash for Pooled<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.get_inner().hash);
    }
}
impl<T: Hash + Eq + Sync + Send> PartialEq for Pooled<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}
unsafe impl<T: Hash + Eq + Sync + Send> Sync for Pooled<T> {}
unsafe impl<T: Hash + Eq + Sync + Send> Send for Pooled<T> {}
impl<T: Hash + Eq + Sync + Send> Eq for Pooled<T> {}
impl<T: Hash + Eq + Sync + Send + PartialOrd> PartialOrd for Pooled<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        T::partial_cmp(self, other)
    }
}
impl<T: Hash + Eq + Sync + Send + Ord> Ord for Pooled<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        T::cmp(self, other)
    }
}
impl<T: Hash + Eq + Sync + Send + Debug> Debug for Pooled<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PooledStr").field("inner_ptr", &self.inner).field("inner", self.get_inner()).finish()
    }
}
impl<T: Hash + Eq + Sync + Send> Drop for Pooled<T> {
    fn drop(&mut self) {
        let inner = self.get_inner();
        let old_rc = inner.rc.fetch_sub(1, Ordering::Relaxed);
        if old_rc == 1 {
            inner.pool.upgrade().map(|pool| pool.pool.remove_if(inner, |k, _| k.rc.load(Ordering::Acquire) == 0));
        }
    }
}
impl<T: Hash + Eq + Sync + Send> Clone for Pooled<T> {
    fn clone(&self) -> Self {
        unsafe {
            let inner = self.get_inner();
            Pooled::new(inner)
        }
    }
}
impl<T: Hash + Eq + Sync + Send> Borrow<T> for Inner<T> {
    fn borrow(&self) -> &T {
        &self.data
    }
}
impl<T: Hash + Eq + Sync + Send> Deref for Pooled<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.inner.as_ref() }
    }
}
impl<T: Hash + Eq + Sync + Send> Borrow<T> for Pooled<T> {
    fn borrow(&self) -> &T {
        unsafe { self.inner.as_ref() }
    }
}
impl<T: Hash + Eq + Sync + Send + Display> Display for Pooled<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        (**self).fmt(f)
    }
}
pub struct RawPool<T: Hash + Eq + Sync + Send> {
    pool: DashMap<Inner<T>, (), BuildBKDRHash>,
}
impl<T: Hash + Eq + Sync + Send> RawPool<T> {
    pub fn new() -> Self {
        Self { pool: DashMap::with_hasher(BuildBKDRHash {}) }
    }

    pub fn insert(this: &Pool<T>, data: T) -> Pooled<T> {
        if let Some(ref_inner) = this.pool.get(&data) {
            return unsafe { Pooled::new(ref_inner.key()) };
        }
        let inner = Inner::new(data, this);
        match this.pool.entry(inner) {
            Entry::Occupied(o) => unsafe { Pooled::new(o.key()) },
            Entry::Vacant(v) => unsafe {
                let ptr = NonNull::from(v.key());
                v.insert(());
                Pooled::from_raw(ptr)
            },
        }
    }
}
pub type Pool<T> = Arc<RawPool<T>>;
#[macro_export]
macro_rules! static_pool {
    ($name:ident:$type:ty) => {
        lazy_static! {
            static ref $name: Pool<&type>=Pool::new();
        }
    };
}
