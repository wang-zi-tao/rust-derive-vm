use std::{
    borrow::Borrow,
    cell::UnsafeCell,
    mem::MaybeUninit,
    ops::Deref,
    sync::{atomic::AtomicBool, Mutex},
};

use atomic::Ordering;

trait Module {
    fn name(&self) -> &'static str;
}
lazy_static! {
    static ref GLOBAL_LOCK: Mutex<()> = Mutex::new(());
}
pub struct LazyDynRef<'l, T: ?Sized>(AtomicBool, UnsafeCell<MaybeUninit<&'l T>>);
impl<'l, T: ?Sized> LazyDynRef<'l, T> {
    pub fn uninit() -> Self {
        Self(
            AtomicBool::new(false),
            UnsafeCell::new(MaybeUninit::uninit()),
        )
    }

    pub fn new(value: &'l T) -> Self {
        Self(
            AtomicBool::new(true),
            UnsafeCell::new(MaybeUninit::new(value)),
        )
    }

    pub fn as_ref(&self) -> &T {
        if !self.0.load(Ordering::Relaxed) && !self.0.load(Ordering::Acquire) {
            panic!("LazyDynRef not initialized yet");
        }
        unsafe { self.1.get().as_ref().unwrap().assume_init() }
    }

    pub fn initialized(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }

    pub fn init(&self, value: &'l T) {
        let _global_guard = GLOBAL_LOCK.lock();
        if !self.initialized() {
            unsafe {
                self.1.get().replace(MaybeUninit::new(value));
                self.0.store(true, Ordering::Release);
            }
        } else {
            panic!("LazyDynRef aready initialized");
        }
    }
}
unsafe impl<'l, T: ?Sized> Send for LazyDynRef<'l, T> {}
unsafe impl<'l, T: ?Sized> Sync for LazyDynRef<'l, T> {}
impl<'l, T: ?Sized> Borrow<T> for LazyDynRef<'l, T> {
    fn borrow(&self) -> &T {
        self.as_ref()
    }
}
impl<'l, T: ?Sized> Deref for LazyDynRef<'l, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}
impl<'l, T: ?Sized> Default for LazyDynRef<'l, T> {
    fn default() -> Self {
        Self::uninit()
    }
}
