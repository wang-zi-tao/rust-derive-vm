use std::{
    borrow::Borrow,
    fmt::Debug,
    hash::Hash,
    ops::{Deref, DerefMut},
    sync::{Arc, Weak},
};

#[derive(Debug, Eq)]
pub enum CowArc<'l, T: ?Sized> {
    Owned(Arc<T>),
    Ref(&'l T),
}

impl<'l, T: ?Sized> PartialEq for CowArc<'l, T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        &**self == &**other
    }
}

impl<'l, T> CowArc<'l, T> {
    pub fn new(i: T) -> Self {
        Self::Owned(Arc::new(i))
    }
}
impl<'l, T: ?Sized> CowArc<'l, T> {
    pub fn as_ptr(&self) -> *const T {
        &**self as *const T
    }

    pub fn into_raw(self) -> *const T {
        match self {
            CowArc::Owned(o) => Arc::into_raw(o),
            CowArc::Ref(r) => r as *const T,
        }
    }
}

impl<'l, T: ?Sized + Hash> Hash for CowArc<'l, T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.deref().hash(state)
    }
}
impl<'l, T: Clone> CowArc<'l, T> {
    pub fn get_mut(&mut self) -> &mut Arc<T> {
        match self {
            CowArc::Owned(o) => o,
            CowArc::Ref(r) => {
                let v = Arc::new(r.clone());
                *self = Self::Owned(v);
                match self {
                    CowArc::Owned(o) => o,
                    CowArc::Ref(_) => {
                        unreachable!()
                    }
                }
            }
        }
    }

    pub fn downgrade(&self) -> CowWeak<T> {
        match self {
            CowArc::Owned(o) => CowWeak::Weak(Arc::downgrade(o)),
            CowArc::Ref(r) => CowWeak::Ref(r),
        }
    }
}
impl<'l, T: ?Sized> Clone for CowArc<'l, T> {
    fn clone(&self) -> Self {
        match self {
            Self::Owned(arg0) => Self::Owned(arg0.clone()),
            Self::Ref(arg0) => Self::Ref(arg0),
        }
    }
}
impl<'l, T: ?Sized> Deref for CowArc<'l, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            CowArc::Owned(o) => &**o,
            CowArc::Ref(r) => *r,
        }
    }
}
impl<'l, T> Borrow<T> for CowArc<'l, T> {
    fn borrow(&self) -> &T {
        self.deref()
    }
}
#[derive(Debug)]
pub enum CowWeak<'l, T: ?Sized> {
    Weak(Weak<T>),
    Ref(&'l T),
}

impl<'l, T: ?Sized> CowWeak<'l, T> {
    pub fn upgrade(&self) -> Option<CowArc<'l, T>> {
        match self {
            CowWeak::Weak(w) => Weak::upgrade(w).map(CowArc::Owned),
            CowWeak::Ref(r) => Some(CowArc::Ref(r)),
        }
    }
}
#[derive(Clone, Debug, Eq)]
pub enum CowSlice<'l, T> {
    Owned(Vec<T>),
    Ref(&'l [T]),
}

impl<'l, T> PartialOrd for CowSlice<'l, T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (&**self).partial_cmp(&**other)
    }
}

impl<'l, T> Ord for CowSlice<'l, T>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (&**self).cmp(&**other)
    }
}

impl<'l, T> PartialEq for CowSlice<'l, T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        &**self == &**other
    }
}

impl<'l, T> CowSlice<'l, T> {
    pub const fn new() -> Self {
        Self::Owned(Vec::new())
    }
}
impl<'l, T> Default for CowSlice<'l, T> {
    fn default() -> Self {
        Self::Owned(Vec::new())
    }
}
impl<'l, T> From<Vec<T>> for CowSlice<'l, T> {
    fn from(v: Vec<T>) -> Self {
        Self::Owned(v)
    }
}
impl<'l, T: Clone> CowSlice<'l, T> {
    pub fn get_mut(&mut self) -> &mut Vec<T> {
        match self {
            CowSlice::Owned(o) => o,
            CowSlice::Ref(r) => {
                let v = r.to_vec();
                *self = Self::Owned(v);
                match self {
                    CowSlice::Owned(o) => o,
                    CowSlice::Ref(_) => {
                        unreachable!()
                    }
                }
            }
        }
    }
}

impl<'r, 'l, T> IntoIterator for &'r CowSlice<'l, T> {
    type IntoIter = std::slice::Iter<'r, T>;
    type Item = &'r T;

    fn into_iter(self) -> Self::IntoIter {
        (&**self).iter()
    }
}
impl<'l, T> Deref for CowSlice<'l, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        match self {
            CowSlice::Owned(o) => &**o,
            CowSlice::Ref(r) => *r,
        }
    }
}
impl<'l, T: Clone> DerefMut for CowSlice<'l, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}
