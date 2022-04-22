use atomic::Ordering;
use crossbeam::epoch::{self, Atomic, Guard, Owned};
use std::{fmt::Debug, ops::Deref};
#[derive(Default)]
pub struct AtomicCell<T>(Atomic<T>);
impl<T> AtomicCell<T> {
    pub fn load(&self) -> AtomicCellGuard<'_, T> {
        AtomicCellGuard::new(self)
    }

    pub fn new(value: T) -> Self {
        Self(Atomic::new(value))
    }

    pub fn null() -> Self {
        Self(Atomic::null())
    }
}
pub struct AtomicCellGuard<'g, T> {
    guard: Guard,
    cell: &'g AtomicCell<T>,
}
impl<'g, T> AtomicCellGuard<'g, T> {
    pub fn new(cell: &'g AtomicCell<T>) -> Self {
        Self {
            guard: epoch::pin(),
            cell,
        }
    }

    pub fn load(&self) -> &T {
        self.load_option().unwrap()
    }

    pub fn load_option(&self) -> Option<&T> {
        unsafe { self.cell.0.load(Ordering::Acquire, &self.guard).as_ref() }
    }

    pub fn store(&self, value: T) {
        let old = self
            .cell
            .0
            .swap(Owned::new(value), Ordering::SeqCst, &self.guard);
        unsafe {
            self.guard.defer_destroy(old);
        }
    }
}
impl<'g, T> Deref for AtomicCellGuard<'g, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.load()
    }
}
impl<T> Debug for AtomicCell<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let guard = self.load();
        guard.load_option().fmt(f)
    }
}
