use std::{
    fmt::Debug,
    mem::{align_of, size_of},
    ptr::NonNull,
};

pub struct UnsafeBuffer {
    data: NonNull<[u8]>,
    len: usize,
}
impl UnsafeBuffer {
    pub fn new() -> Self {
        let vec = Vec::<u8>::new();
        let l = vec.leak();
        Self { data: l.into(), len: 0 }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        unsafe { Self::from_vec(Vec::with_capacity(capacity)) }
    }

    pub unsafe fn get_ptr<T: Copy>(&self, offset: usize) -> NonNull<T> {
        assert!(self.len() >= offset + size_of::<T>(), "UnsafeBuffer offset out of bounds, len:{}, offset:{}, size:{}", self.len(), offset, size_of::<T>());
        NonNull::new_unchecked(self.data.as_ptr().cast::<u8>().add(offset).cast())
    }

    unsafe fn get_tail_ptr_and_grow<T: Copy>(&mut self) -> NonNull<T> {
        let offset = self.len();
        let new_len = self.len() + size_of::<T>();
        if new_len > self.capacity() {
            self.grow(new_len);
        }
        self.len = new_len;
        self.get_ptr(offset)
    }

    pub unsafe fn slice(&self, start: usize, len: usize) -> *const [u8] {
        &self.data.as_ref()[start..start + len] as *const [u8]
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn capacity(&self) -> usize {
        unsafe { self.data.as_ref().len() }
    }

    pub unsafe fn into_vec(self) -> Vec<u8> {
        Vec::from_raw_parts(self.get_ptr(0).as_ptr(), self.len, self.capacity())
    }

    pub unsafe fn from_vec(vec: Vec<u8>) -> Self {
        let cap = vec.capacity();
        let p = Vec::leak(vec);
        let ptr = p.as_mut_ptr();
        Self { len: p.len(), data: NonNull::new_unchecked(std::ptr::slice_from_raw_parts_mut(ptr, cap)) }
    }

    pub unsafe fn grow(&mut self, new_size: usize) {
        let new_size = 1 << (usize::BITS - new_size.leading_zeros());
        let mut vec = Vec::<u8>::from_raw_parts(self.data.as_ptr().cast::<u8>(), self.len, self.capacity());
        vec.reserve(new_size);
        *self = Self::from_vec(vec);
    }

    pub unsafe fn borrow_mut(&mut self) -> &mut [u8] {
        &mut self.data.as_mut()[0..self.len]
    }

    pub unsafe fn borrow(&self) -> &[u8] {
        &self.data.as_ref()[0..self.len]
    }

    pub unsafe fn push_slice(&mut self, value: &[u8]) {
        let start = self.len();
        let new_len = self.len() + value.len();
        if new_len > self.capacity() {
            self.grow(new_len);
        }
        self.data.as_mut()[start..new_len].copy_from_slice(value);
        self.len = new_len;
    }

    pub unsafe fn set_slice(&mut self, start: usize, value: &[u8]) {
        self.data.as_mut()[start..start + value.len()].copy_from_slice(value);
    }

    pub unsafe fn align(&mut self, align: usize) {
        let target_len = (self.len() + (align - 1)) & !(align - 1);
        for _ in self.len()..target_len {
            self.push(0u8);
        }
    }

    pub unsafe fn push<T: Copy>(&mut self, value: T) {
        self.align(align_of::<T>());
        self.get_tail_ptr_and_grow::<T>().as_ptr().write(value)
    }

    pub unsafe fn set<T>(&mut self, offset: usize, value: T) {
        self.get_ptr::<u8>(offset).cast::<T>().as_ptr().write(value)
    }
}
impl Debug for UnsafeBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe { f.write_fmt(format_args!("UnsafeBuffer(size:{},capacity:{},{:X?})", self.len(), self.capacity(), self.borrow())) }
    }
}
