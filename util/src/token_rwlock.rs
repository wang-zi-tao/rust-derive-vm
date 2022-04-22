use core::cell::UnsafeCell;
use failure::{format_err, Error};
use std::{
    fmt::Debug,
    hash::Hash,
    ops::{Deref, DerefMut},
    sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard},
};
pub struct ReadToken<'a> {
    lock: &'a Key,
    key: RwLockReadGuard<'a, ()>,
}
pub struct WriteToken<'a> {
    lock: &'a Key,
    key: RwLockWriteGuard<'a, ()>,
}
pub struct TokenLockReadGuard<'a, T> {
    cell: &'a UnsafeCell<T>,
    _key: RwLockReadGuard<'a, ()>,
}
impl<'a, T> Deref for TokenLockReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.cell.get().as_ref().unwrap() }
    }
}
pub struct TokenLockWriteGuard<'a, T> {
    cell: &'a UnsafeCell<T>,
    _key: RwLockWriteGuard<'a, ()>,
}
impl<'a, T> Deref for TokenLockWriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.cell.get().as_ref().unwrap() }
    }
}
impl<'a, T> DerefMut for TokenLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.cell.get().as_mut().unwrap() }
    }
}
#[derive(Default)]
pub struct RawKey {
    lock: RwLock<()>,
}
#[derive(Clone, Default)]
pub struct Key {
    inner: Arc<RawKey>,
}
impl Key {
    pub fn read<'a>(&'a self) -> Result<ReadToken<'a>, PoisonError<RwLockReadGuard<'_, ()>>> {
        self.inner.lock.read().map(|r| ReadToken { lock: self, key: r })
    }

    pub fn write<'a>(&'a self) -> Result<WriteToken<'a>, PoisonError<RwLockWriteGuard<'_, ()>>> {
        self.inner.lock.write().map(|r| WriteToken { lock: self, key: r })
    }
}
pub struct TokenRwLock<T> {
    inner: UnsafeCell<T>,
    key: Key,
}
impl<T> TokenRwLock<T> {
    pub fn read_without_token<'a>(&'a self) -> Result<TokenLockReadGuard<'a, T>, Error> {
        let key = self.key.read().map_err(|e| format_err!("PoisonError{{{:?}}}", e))?.key;
        Ok(TokenLockReadGuard { cell: &self.inner, _key: key })
    }

    pub fn write_without_token<'a>(&'a self) -> Result<TokenLockWriteGuard<'a, T>, Error> {
        let key = self.key.write().map_err(|e| format_err!("PoisonError{{{:?}}}", e))?.key;
        Ok(TokenLockWriteGuard { cell: &self.inner, _key: key })
    }

    pub fn read<'a>(&self, token: &ReadToken<'a>) -> Option<&T> {
        if Arc::as_ptr(&token.lock.inner) == Arc::as_ptr(&self.key.inner) {
            unsafe { Some(self.inner.get().as_ref().unwrap()) }
        } else {
            None
        }
    }

    pub fn write<'a>(&self, token: &WriteToken<'a>) -> Option<&mut T> {
        if Arc::as_ptr(&token.lock.inner) == Arc::as_ptr(&self.key.inner) {
            unsafe { Some(self.inner.get().as_mut().unwrap()) }
        } else {
            None
        }
    }

    pub fn new(key: Key, inner: T) -> Self {
        Self { key, inner: UnsafeCell::new(inner) }
    }
}
unsafe impl<T> Send for TokenRwLock<T> {}
unsafe impl<T> Sync for TokenRwLock<T> {}
impl<T: Debug> Debug for TokenRwLock<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let read_guard = self.read_without_token().unwrap();
        let ref_inner = read_guard.deref();
        ref_inner.fmt(f)
    }
}
impl<T: Hash> Hash for TokenRwLock<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let read_guard = self.read_without_token().unwrap();
        let ref_inner = read_guard.deref();
        ref_inner.hash(state);
    }
}
impl<T: PartialOrd> PartialOrd for TokenRwLock<T> {
    fn partial_cmp(&self, _other: &Self) -> Option<std::cmp::Ordering> {
        todo!()
    }
}
impl<T: Ord> Ord for TokenRwLock<T> {
    fn cmp(&self, _other: &Self) -> std::cmp::Ordering {
        todo!()
    }
}
impl<T: PartialEq> PartialEq for TokenRwLock<T> {
    fn eq(&self, _other: &Self) -> bool {
        todo!()
    }
}
impl<T: Eq> Eq for TokenRwLock<T> {}
