use std::{fmt::Debug, ops::Deref, ptr::null_mut, sync::Arc};

use crossbeam::atomic::AtomicCell;

pub struct AtomicLazyArc<T>(AtomicCell<*const T>);
impl<T> AtomicLazyArc<T> {
    pub fn new_initialized_arc(value: Arc<T>) -> AtomicLazyArc<T> {
        Self(AtomicCell::new(Arc::into_raw(value)))
    }

    pub fn new_initialized(value: T) -> AtomicLazyArc<T> {
        Self(AtomicCell::new(Arc::into_raw(Arc::new(value))))
    }

    pub fn new_uninitalized() -> Self {
        Self(AtomicCell::new(null_mut()))
    }

    pub fn init(&self, value: Arc<T>) {
        self.0
            .compare_exchange(null_mut(), Arc::into_raw(value))
            .unwrap();
    }

    pub fn is_loaded(&self) -> bool {
        !self.0.load().is_null()
    }

    pub fn load_option(&self) -> Option<&T> {
        unsafe { self.0.load().as_ref() }
    }

    pub fn load(&self) -> &T {
        self.load_option().unwrap()
    }
}
impl<T> AtomicLazyArc<T> {
    pub fn clone_option_arc(&self) -> Option<Arc<T>> {
        unsafe {
            let ptr = self.0.load();
            if ptr.is_null() {
                None
            } else {
                Arc::increment_strong_count(ptr);
                Some(Arc::from_raw(ptr))
            }
        }
    }

    pub fn clone_arc(&self) -> Arc<T> {
        self.clone_option_arc().unwrap()
    }
}
impl<T> Drop for AtomicLazyArc<T> {
    fn drop(&mut self) {
        unsafe { Arc::from_raw(self.0.load()) };
    }
}
impl<T> Deref for AtomicLazyArc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.load()
    }
}
impl<T: Debug> Debug for AtomicLazyArc<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.load_option().fmt(f)
    }
}
impl<T> Default for AtomicLazyArc<T> {
    fn default() -> Self {
        Self::new_uninitalized()
    }
}
impl<T> Clone for AtomicLazyArc<T> {
    fn clone(&self) -> Self {
        match self.clone_option_arc() {
            Some(i) => Self::new_initialized_arc(i),
            None => Self::new_uninitalized(),
        }
    }
}
unsafe impl<T: Send> Send for AtomicLazyArc<T> {}

unsafe impl<T: Sync> Sync for AtomicLazyArc<T> {}
