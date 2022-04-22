use failure::Fallible;
use jvm_core::TypeLayout;


use crate::{metadata::Metadata, RegistedType};

use crate::object::Object;

use std::{
    mem::{size_of, MaybeUninit},
    ptr::{self, null_mut, NonNull},
};
pub const HEAP_PAGE_SIZE: usize = 1 << 12;
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AllocationStrategy {
    Small,
    SmallUnsized,
    Large,
}

impl AllocationStrategy {
    #[inline(always)]
    pub fn new() -> Self {
        Self::SmallUnsized
    }
}
pub(crate) enum AllocatorKind {
    SmallLinkList(LinkedListAllocator),
    Mask(Mask),
}
pub struct Allocator {
    kind: AllocatorKind,
}
pub struct SingleTypeHeapRef(NonNull<[u8]>);
impl SingleTypeHeapRef {
    #[inline(always)]
    pub(crate) unsafe fn alloc(&self, layout: TypeLayout) -> Option<NonNull<u8>> {
        self.0.as_non_null_ptr().cast::<Allocator>().as_mut().alloc(layout)
    }

    #[inline(always)]
    pub(crate) unsafe fn alloc_unsized(&self, layout: TypeLayout, len: usize) -> Option<NonNull<u8>> {
        self.0.as_non_null_ptr().cast::<Allocator>().as_mut().alloc_unsized(layout, len)
    }

    #[inline(always)]
    pub(crate) fn is_full(&self) -> bool {
        unsafe { NonNull::from(self).cast::<Allocator>().as_ref().is_full() }
    }

    #[inline(always)]
    pub(crate) unsafe fn new(p: NonNull<[u8]>, layout: TypeLayout, ty: &RegistedType, strategy: AllocationStrategy) -> SingleTypeHeapRef {
        Allocator::new(p.as_non_null_ptr(), layout, strategy);
        p.cast::<Allocator>().as_mut().init(layout, strategy);
        ty.after_creat_heap(p.len());
        Self(p)
    }

    #[inline(always)]
    pub(crate) unsafe fn scan(&self, layout: TypeLayout, callback: impl FnMut(NonNull<u8>) -> Fallible<()>, len_offset: Option<usize>) -> Fallible<()> {
        self.0.as_non_null_ptr().cast::<Allocator>().as_ref().scan(layout, callback, len_offset)
    }
}
impl !Unpin for Allocator {}
unsafe impl Send for Allocator {}
unsafe impl Sync for Allocator {}
impl Drop for Allocator {
    fn drop(&mut self) {
        panic!("should not call `drop` on `SingleTypeHeap`!")
    }
}
impl Allocator {
    #[inline(always)]
    unsafe fn new(ptr: NonNull<u8>, _layout: TypeLayout, strategy: AllocationStrategy) {
        let kind = match strategy {
            AllocationStrategy::SmallUnsized => AllocatorKind::Mask(Mask::default()),
            AllocationStrategy::Small | AllocationStrategy::Large => AllocatorKind::SmallLinkList(LinkedListAllocator::default()),
        };
        let this = Self { kind };
        ptr.cast::<Allocator>().as_ptr().write(this);
    }

    #[inline(always)]
    unsafe fn alloc(&mut self, layout: TypeLayout) -> Option<NonNull<u8>> {
        match &mut self.kind {
            AllocatorKind::SmallLinkList(a) => a.alloc(layout),
            AllocatorKind::Mask(a) => a.alloc(layout),
        }
    }

    #[inline(always)]
    unsafe fn alloc_unsized(&mut self, layout: TypeLayout, len: usize) -> Option<NonNull<u8>> {
        match &mut self.kind {
            AllocatorKind::SmallLinkList(a) => a.alloc_unsized(layout, len),
            AllocatorKind::Mask(a) => a.alloc_unsized(layout, len),
        }
    }

    #[inline(always)]
    fn is_full(&self) -> bool {
        match &self.kind {
            AllocatorKind::SmallLinkList(l) => l.is_full(),
            AllocatorKind::Mask(a) => a.is_full(),
        }
    }

    #[inline(always)]
    unsafe fn init(&mut self, layout: TypeLayout, _strategy: AllocationStrategy) {
        match &mut self.kind {
            AllocatorKind::SmallLinkList(a) => a.init(layout),
            AllocatorKind::Mask(a) => a.init(layout),
        }
    }

    unsafe fn scan(&self, layout: TypeLayout, callback: impl FnMut(NonNull<u8>) -> Fallible<()>, len_offset: Option<usize>) -> Fallible<()> {
        match &self.kind {
            AllocatorKind::SmallLinkList(a) => a.scan(layout, callback, len_offset),
            AllocatorKind::Mask(a) => a.scan(layout, callback, len_offset),
        }
    }
}
pub(crate) struct Mask(usize);

impl Mask {
    #[inline(always)]
    pub unsafe fn init(&mut self, layout: TypeLayout) {
        let start_offset = (size_of::<AllocatorKind>() + (layout.align() - 1)) & !(layout.align() - 1);
        let count = (HEAP_SEGMENT_SIZE - start_offset) / Self::cell_size(layout);
        let mask = (-1isize >> (usize::BITS as usize - count)) as usize;
        *self = Self(mask);
    }

    #[inline(always)]
    pub unsafe fn cell_size(layout: TypeLayout) -> usize {
        (layout.size() + (layout.align() - 1)) & !(layout.align() - 1)
    }

    #[inline(always)]
    pub unsafe fn start_ptr(this: NonNull<Self>, layout: TypeLayout) -> NonNull<u8> {
        let ptr_usize = this.as_ptr().cast::<AllocatorKind>().offset(1) as usize;
        let ptr_usize = (ptr_usize + (layout.align() - 1)) & !(layout.align() - 1);
        NonNull::new_unchecked(ptr_usize as *mut u8)
    }

    #[inline(always)]
    pub fn is_full(&self) -> bool {
        self.0 == 0
    }

    #[inline(always)]
    pub unsafe fn alloc(&mut self, layout: TypeLayout) -> Option<NonNull<u8>> {
        if self.0 == 0 {
            None
        } else {
            let cell_num = self.0.trailing_zeros();
            self.0 &= !(1 << cell_num);
            let size = (layout.size() + layout.align() - 1) & !(layout.align() - 1);
            Some(NonNull::new_unchecked(Self::start_ptr(NonNull::from(self), layout).as_ptr().add(cell_num as usize * size)))
        }
    }

    #[inline(always)]
    pub unsafe fn alloc_unsized(&mut self, layout: TypeLayout, flexible_len: usize) -> Option<NonNull<u8>> {
        if self.0 == 0 {
            None
        } else {
            let size = layout.size() + layout.flexible_size() * flexible_len;
            let cell_size = Self::cell_size(layout);
            let cell_count = (size + cell_size - 1) / cell_size;
            let mut mask = self.0;
            if cell_count & 1 != 0 {
                mask &= mask >> 1;
            };
            if cell_count & 2 != 0 {
                mask &= mask >> 1;
                mask &= mask >> 2;
            };
            if cell_count & 4 != 0 {
                mask &= mask >> 1;
                mask &= mask >> 2;
                mask &= mask >> 4;
            };
            if cell_count & 8 != 0 {
                mask &= mask >> 1;
                mask &= mask >> 2;
                mask &= mask >> 4;
                mask &= mask >> 8;
            };
            if cell_count & 16 != 0 {
                mask &= mask >> 1;
                mask &= mask >> 2;
                mask &= mask >> 4;
                mask &= mask >> 8;
                mask &= mask >> 16;
            };
            if cell_count & 32 != 0 {
                mask &= mask >> 1;
                mask &= mask >> 2;
                mask &= mask >> 4;
                mask &= mask >> 8;
                mask &= mask >> 16;
                mask &= mask >> 32;
            };
            let cell_num = mask.trailing_zeros();
            Some(NonNull::new_unchecked(Self::start_ptr(NonNull::from(self), layout).as_ptr().add(cell_num as usize * cell_size)))
        }
    }

    unsafe fn scan(
        &self,
        layout: TypeLayout,
        mut callback: impl FnMut(NonNull<u8>) -> Result<(), failure::Error>,
        len_offset: Option<usize>,
    ) -> Result<(), failure::Error> {
        let this = NonNull::from(self);
        let start = Self::start_ptr(this, layout);
        let base_len = layout.into_flexible_array().flexible_size();
        let mut ptr = start;
        let mut mask = self.0;
        while mask != 0 {
            let len;
            if mask & 1 != 0 {
                callback(ptr)?;
                if let Some(len_offset) = len_offset {
                    len = layout.size() + layout.flexible_size() * (ptr.as_ptr().add(len_offset).cast::<usize>().read())
                } else {
                    len = base_len;
                }
            } else {
                len = base_len;
            }
            mask >>= len / base_len;
            ptr = NonNull::new_unchecked(ptr.as_ptr().add(len));
        }
        Ok(())
    }
}
impl Default for Mask {
    #[inline(always)]
    fn default() -> Mask {
        Mask(0)
    }
}
pub(crate) struct LinkedListAllocator {
    left_cell_count: usize,
    head: *mut FreeLinkedListNode,
}
impl LinkedListAllocator {
    #[inline(always)]
    pub unsafe fn init(&mut self, layout: TypeLayout) {
        let cell_size = Self::cell_size(layout);
        let end_addr = self.as_ptr() as usize + HeapPage::size();
        let start_addr = self.as_ptr() as usize + size_of::<Allocator>();
        let start_addr = (start_addr + (layout.align() - 1)) & !(layout.align() - 1);
        let cell_count = (end_addr - start_addr) / cell_size;
        let node_ptr = start_addr as *mut FreeLinkedListNode;
        node_ptr.write(FreeLinkedListNode { next: ptr::null_mut::<FreeLinkedListNode>(), available_cell: cell_count });
        *self = LinkedListAllocator { left_cell_count: cell_count, head: node_ptr };
    }

    #[inline(always)]
    fn as_ptr(&self) -> *const u8 {
        (self as *const Self).cast()
    }

    #[inline(always)]
    pub unsafe fn alloc<'a>(&mut self, layout: TypeLayout) -> Option<NonNull<u8>> {
        self.alloc_cell(layout, 0)
    }

    #[inline(always)]
    pub unsafe fn cell_size(layout: TypeLayout) -> usize {
        (layout.size() + (layout.align() - 1)) & !(layout.align() - 1)
    }

    pub unsafe fn alloc_unsized<'a>(&mut self, layout: TypeLayout, flexible_len: usize) -> Option<NonNull<u8>> {
        let size = layout.size() + layout.flexible_size() * flexible_len;
        let size = (size + layout.align() - 1) & !(layout.align() - 1);
        let cell_count = size / Self::cell_size(layout);
        self.alloc_cell(layout, cell_count)
    }

    #[inline(always)]
    pub unsafe fn alloc_cell<'a>(&mut self, layout: TypeLayout, cell_count: usize) -> Option<NonNull<u8>> {
        let node_handle = &mut self.head;
        if self.left_cell_count < cell_count {
            return None;
        }
        loop {
            if let Some(mut node) = (*node_handle).as_mut() {
                if node.available_cell == cell_count {
                    let ptr = NonNull::from(&node).cast();
                    *node_handle = node.next;
                    self.left_cell_count -= cell_count;
                    return Some(ptr);
                } else if node.available_cell > cell_count {
                    node.available_cell -= cell_count;
                    let cell_size = Self::cell_size(layout);
                    let ptr = node.offset(node.available_cell * cell_size).cast();
                    self.left_cell_count -= cell_count;
                    return Some(ptr);
                }
            } else {
                return None;
            }
        }
    }

    #[inline(always)]
    fn is_full(&self) -> bool {
        self.left_cell_count == 0
    }

    unsafe fn scan(
        &self,
        layout: TypeLayout,
        mut callback: impl FnMut(NonNull<u8>) -> Result<(), failure::Error>,
        len_offset: Option<usize>,
    ) -> Result<(), failure::Error> {
        let mut node = self.head;
        let base_len = layout.into_flexible_array().flexible_size();
        while !node.is_null() {
            let node_ref = node.as_mut().unwrap();
            let mut iter = node_ref.offset(node_ref.available_cell).cast::<u8>();
            while iter.as_ptr() < node_ref.next.cast() {
                let ptr = NonNull::new_unchecked(iter.as_ptr());
                callback(ptr)?;
                let len = if let Some(len_offset) = len_offset {
                    layout.size() + layout.flexible_size() * (ptr.as_ptr().add(len_offset).cast::<usize>().read())
                } else {
                    base_len
                };
                iter = NonNull::new(iter.as_ptr().add(len)).unwrap();
            }
            node = node_ref.next;
        }
        Ok(())
    }
}
impl Default for LinkedListAllocator {
    fn default() -> LinkedListAllocator {
        Self { left_cell_count: 0, head: null_mut() }
    }
}
impl !Unpin for HeapPage {}
pub struct HeapPage {
    padding: MaybeUninit<[u8; HEAP_PAGE_SIZE]>,
}
impl Drop for HeapPage {
    fn drop(&mut self) {
        panic!("should not call `drop` on `HeapPage`!")
    }
}
impl HeapPage {
    #[inline(always)]
    pub fn size() -> usize {
        HEAP_PAGE_SIZE - size_of::<FreeLinkedListNode>()
    }

    #[inline(always)]
    pub unsafe fn from_oop(object: &Object) -> &HeapPage {
        (((object as *const Object as usize) & (!(HEAP_PAGE_SIZE - 1))) as *mut HeapPage).as_ref().unwrap()
    }

    #[inline(always)]
    pub fn as_ptr(&self) -> *const HeapPage {
        self as *const HeapPage
    }

    #[inline(always)]
    pub fn get_mate_data(self: &HeapPage) -> &Metadata {
        unsafe { ptr::read((self as *const HeapPage as usize / HEAP_PAGE_SIZE * size_of::<*const Metadata>()) as *const *mut Metadata).as_ref().unwrap() }
    }
}
pub struct FreeLinkedListNode {
    pub next: *mut FreeLinkedListNode,
    pub available_cell: usize,
}
impl FreeLinkedListNode {
    #[inline(always)]
    unsafe fn try_alloc_on_self(&mut self, size: usize) -> Option<NonNull<u8>> {
        if self.available_cell < size {
            self.available_cell -= size;
            let ptr = self.offset(self.available_cell);
            Some(ptr)
        } else {
            None
        }
    }

    #[inline(always)]
    unsafe fn offset(&self, offset: usize) -> NonNull<u8> {
        NonNull::new_unchecked(NonNull::from(self).as_ptr().add(1).cast::<u8>().add(offset))
    }
}
pub const HEAP_SEGMENT_SIZE: usize = 1 << 21;
pub const HEAP_LARGE_SEGMENT_SIZE: usize = 1 << 40;
pub const PAGES_PRE_SEGMENT: usize = HEAP_SEGMENT_SIZE / HEAP_PAGE_SIZE;
pub const OBJECT_MIN_ALIAS: usize = 16;
// pub const GC_MASK: usize = 1 << 46;
pub struct HeapSegment {
    pointer: *const u8,
    segment_length: usize,
}
#[derive(PartialEq, Eq)]
pub struct HeapFrameRef {
    start: NonNull<[u8]>,
    segment_length: usize,
    tire: usize,
}
impl HeapFrameRef {
    #[inline(always)]
    pub fn into_first_segment_pages(self) -> Vec<NonNull<HeapPage>> {
        (0..self.segment_length / HEAP_PAGE_SIZE)
            .map(|i| unsafe { NonNull::new_unchecked(self.start.as_non_null_ptr().as_ptr().add(i * HEAP_PAGE_SIZE).cast()) })
            .collect()
    }

    #[inline(always)]
    pub(crate) unsafe fn new(as_slice_ptr: NonNull<[u8]>, heap_segment_size: usize, tire: usize) -> HeapFrameRef {
        Self { start: as_slice_ptr, segment_length: heap_segment_size, tire }
    }

    #[inline(always)]
    pub(crate) fn as_ptr(&self) -> NonNull<[u8]> {
        self.start
    }
}
