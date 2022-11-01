use super::hash::BKDRHash;
use crate::BuildBKDRHash;
use core::ops::Deref;
use dashmap::{mapref::entry::Entry, DashMap};
use std::{
    borrow::Borrow,
    convert::TryFrom,
    fmt::{Debug, Display, Formatter},
    hash::{Hash, Hasher},
    ptr::NonNull,
    string::FromUtf8Error,
    sync::atomic::{AtomicIsize, Ordering},
};
#[derive(Debug)]
struct Inner {
    pub rc: AtomicIsize,
    pub string: String,
    pub hash: u64,
}
impl Inner {
    fn new(s: String) -> Self {
        let mut hasher = BKDRHash::new();
        s.hash(&mut hasher);
        Inner {
            string: s,
            rc: AtomicIsize::new(1),
            hash: hasher.finish(),
        }
    }
}
impl PartialEq for Inner {
    fn eq(&self, other: &Self) -> bool {
        other.string == self.string
    }
}
impl Eq for Inner {}
impl Hash for Inner {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.string.hash(state);
    }
}
impl Deref for Inner {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.string
    }
}
unsafe impl Sync for Inner {}
lazy_static! {
    static ref POOL: DashMap<Box<Inner>, (), BuildBKDRHash> =
        DashMap::with_hasher(BuildBKDRHash {});
}
// #[derive(Hash, Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PooledStr {
    inner: NonNull<Inner>,
}
impl PooledStr {
    unsafe fn new(inner: &Inner) -> PooledStr {
        inner.rc.fetch_add(1, Ordering::Relaxed);
        PooledStr {
            inner: NonNull::from(inner),
        }
    }

    unsafe fn from_raw(ptr: NonNull<Inner>) -> PooledStr {
        PooledStr { inner: ptr }
    }

    fn get_inner(&self) -> &Inner {
        unsafe { self.inner.as_ref() }
    }
}
impl Hash for PooledStr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.get_inner().hash);
    }
}
impl PartialEq for PooledStr {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}
unsafe impl Sync for PooledStr {}
unsafe impl Send for PooledStr {}
impl Eq for PooledStr {}
impl PartialOrd for PooledStr {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (**self).partial_cmp(&**other)
    }
}
impl Ord for PooledStr {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (**self).cmp(&**other)
    }
}
impl Debug for PooledStr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PooledStr")
            .field("inner_ptr", &self.inner)
            .field("inner", self.get_inner())
            .finish()
    }
}
impl From<String> for PooledStr {
    fn from(s: String) -> PooledStr {
        if let Some(ref ref_inner) = POOL.get(&*s) {
            return unsafe { PooledStr::new(ref_inner.key()) };
        }
        let inner = Inner::new(s);
        match POOL.entry(Box::new(inner)) {
            Entry::Occupied(o) => unsafe { PooledStr::new(o.key()) },
            Entry::Vacant(v) => {
                let ptr = NonNull::from(&**v.key());
                v.insert(());
                unsafe { PooledStr::from_raw(ptr) }
            }
        }
    }
}
impl From<&str> for PooledStr {
    fn from(s: &str) -> Self {
        PooledStr::from(String::from(s))
    }
}
impl Clone for PooledStr {
    fn clone(&self) -> Self {
        unsafe {
            let inner = self.get_inner();
            PooledStr::new(inner)
        }
    }
}
impl Drop for PooledStr {
    fn drop(&mut self) {
        let inner = self.get_inner();
        let old_rc = inner.rc.fetch_sub(1, Ordering::Release);
        if old_rc == 1 {
            POOL.remove_if(inner, |k, _| k.rc.load(Ordering::Acquire) == 0);
        }
    }
}
impl Borrow<str> for Inner {
    fn borrow(&self) -> &str {
        &self.string
    }
}
impl Borrow<str> for Box<Inner> {
    fn borrow(&self) -> &str {
        &self.string
    }
}
impl From<&String> for PooledStr {
    fn from(s: &String) -> Self {
        PooledStr::from(s.clone())
    }
}
impl TryFrom<Vec<u8>> for PooledStr {
    type Error = FromUtf8Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(String::from_utf8(value)?.into())
    }
}
impl Deref for PooledStr {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        unsafe { self.inner.as_ref() }
    }
}
impl Borrow<str> for PooledStr {
    fn borrow(&self) -> &str {
        unsafe { self.inner.as_ref() }
    }
}
impl Display for PooledStr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self)
    }
}
