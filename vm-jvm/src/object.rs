use std::{ops::Deref, sync::atomic::AtomicPtr};
pub struct Object {}
pub const ARRAY_LENGTH_OFFSET: i32 = 0;

pub trait GCScanner {
    fn mark(object: &impl GCObject);
    fn prev_read_barrier(object: &impl GCObject);
    fn post_read_barrier(object: &impl GCObject);
    fn prev_write_barrier(object: &impl GCObject, field: &impl GCObject);
    fn post_write_barrier(object: &impl GCObject, field: &impl GCObject);
}
pub trait GCObject {
    fn scan(gc: &mut impl GCScanner);
}
pub struct GCReference<O: GCObject>(AtomicPtr<O>);
// unsafe impl<O: GCObject> Send for GCReference<O> where O: Send {}
// unsafe impl<O: GCObject> Sync for GCReference<O> where O: Sync {}

impl<O: GCObject> GCReference<O> {
    fn read<S: GCScanner>(&self, gc: &S) -> ReadGuard<S, O> {
        gc.post_read_barrier(self);
        ReadGuard { gc, object: self }
    }

    fn write(&self, value: &impl GCObject, gc: &impl GCScanner) {
        gc.prev_write_barrier(self, value);
        self.0
            .store(value as *mut O, std::sync::atomic::Ordering::Relaxed);
        gc.post_write_barrier(self, value);
    }
}
impl<O: GCObject> Drop for GCReference<O> {
    fn drop(&mut self) {
        todo!()
    }
}
pub struct ReadGuard<'l, G: GCScanner, O: GCObject> {
    gc: &'l G,
    object: &'l GCReference<O>,
}
impl<'l, G: GCScanner, O: GCObject> Deref for ReadGuard<'l, G, O> {
    type Target = O;

    fn deref(&self) -> &Self::Target {
        unsafe { self.object.0.as_ref() }
    }
}
impl<'l, G: GCScanner, O: GCObject> Drop for ReadGuard<'l, G, O> {
    fn drop(&mut self) {
        self.gc.post_read_barrier(self.object);
    }
}
fn instruction_set() {
    // [fields] [structure] -> [value.field]
    // [arrays] -> [array.iter]
    // [enums] -> []
    // [references]
}
