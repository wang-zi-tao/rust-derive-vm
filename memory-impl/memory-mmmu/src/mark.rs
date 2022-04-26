use std::{
    ptr::NonNull,
    sync::atomic::{AtomicU8, Ordering},
};

use crossbeam::atomic::AtomicCell;
use failure::Fallible;
use os::mem::{MappedVM, VM};
use vm_core::OOPTrait;

pub struct GCHeapFrameMarkSet {
    mem: MappedVM,
}
impl GCHeapFrameMarkSet {
    pub unsafe fn new(start: NonNull<u8>, vm: NonNull<[u8]>) -> Fallible<Self> {
        let marks_ptr =
            NonNull::slice_from_raw_parts(NonNull::new_unchecked((start.as_ptr() as usize + vm.as_ptr().as_mut_ptr() as usize / 64) as *mut u8), vm.len() / 64);
        Ok(Self { mem: VM::new(marks_ptr).map()? })
    }
}
#[derive(Clone)]
pub struct GlobalMarkSet {
    ptr: NonNull<[u8]>,
}
impl Default for GlobalMarkSet {
    fn default() -> Self {
        Self { ptr: NonNull::slice_from_raw_parts(NonNull::dangling(), 0) }
    }
}
unsafe impl Send for GlobalMarkSet {}
unsafe impl Sync for GlobalMarkSet {}

impl GlobalMarkSet {
    pub unsafe fn mark(&self, ptr: NonNull<u8>) {
        let ptr_usize = ptr.as_ptr() as usize;
        let bit_offset = ptr_usize & (u8::BITS as usize - 1);
        let offset = ptr_usize / (u8::BITS as usize);
        NonNull::new_unchecked(self.ptr.as_non_null_ptr().as_ptr().add(offset).cast::<AtomicU8>()).as_mut().fetch_or(1 << bit_offset, Ordering::Relaxed);
    }

    pub unsafe fn is_marked(&self, ptr: NonNull<u8>) -> bool {
        let ptr_usize = ptr.as_ptr() as usize;
        let bit_offset = ptr_usize & (u8::BITS as usize - 1);
        let offset = ptr_usize / (u8::BITS as usize);
        NonNull::new_unchecked(self.ptr.as_non_null_ptr().as_ptr().add(offset).cast::<AtomicCell<u8>>()).as_ref().load() & (1 << bit_offset) != 0
    }
}
