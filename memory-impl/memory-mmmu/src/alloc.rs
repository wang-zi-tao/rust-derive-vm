

use dashmap::{mapref::entry::Entry, DashMap};
use failure::{format_err, Error};
use jvm_core::{TypeLayout, TypeResource};
use os::mem::{VM};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    convert::TryInto,
    ptr::NonNull,
    sync::{Arc, Mutex},
};
use util::{CacheAlignedVec, CowArc};

use super::heap::{HeapFrameRef, HeapPage, HEAP_SEGMENT_SIZE, PAGES_PRE_SEGMENT};
use crate::{
    heap::{AllocationStrategy, SingleTypeHeapRef, HEAP_PAGE_SIZE},
    MemoryMMMU, RegistedType,
};
#[derive(Fail, Debug)]
pub enum AllocError {
    #[fail(display = "no space left")]
    NoSpaceLeft(),
    #[fail(display = "other allocation error :{}", _0)]
    OtherError(#[cause] Error),
    #[fail(display = "heap frame allocation failed!")]
    HeapFrameAllocFailed(),
    #[fail(display = "all retry failed!:\n{:#?}", _0)]
    AllRetryFailed(Vec<AllocError>),
}
use AllocError::*;
impl From<Error> for AllocError {
    fn from(e: Error) -> Self {
        Self::OtherError(e)
    }
}
pub type AllocResult<R> = Result<R, AllocError>;
pub const VM_ALLOC_RETRY: usize = 64;
#[derive(Default)]
pub struct GlobalHeapFrameAllocator {}
impl GlobalHeapFrameAllocator {
    #[inline(always)]
    pub fn alloc(&self, tire: usize) -> AllocResult<HeapFrameRef> {
        for i in 0..VM_ALLOC_RETRY {
            let ret = self.try_alloc(tire);
            match ret {
                Ok(v) => return Ok(v),
                Err(e) => log::warn!("heap frame allocation failed ,retry:{},message:\n{:#?}", VM_ALLOC_RETRY - i, e),
            }
        }
        log::error!("heap frame allocation failed!");
        Err(HeapFrameAllocFailed())
    }

    #[inline(always)]
    pub fn try_alloc(&self, tire: usize) -> AllocResult<HeapFrameRef> {
        let vm = VM::alloc(HEAP_SEGMENT_SIZE * tire as usize)?;
        let memory = VM::new(vm.as_non_null_slice_ptr()).create_shared_memory();
        let mut mems = Vec::new(); // 实现错误时自动unmap
        for m in 0..tire {
            unsafe {
                let seg = VM::new(NonNull::slice_from_raw_parts(
                    NonNull::new_unchecked(vm.as_ptr().offset((HEAP_SEGMENT_SIZE * m as usize).try_into().unwrap())),
                    vm.len(),
                ));
                mems.push(memory.map(seg)?);
            }
        }
        let heap_frame = unsafe {
            HeapFrameRef::new(NonNull::slice_from_raw_parts(vm.as_non_null_slice_ptr().as_non_null_ptr(), HEAP_SEGMENT_SIZE), HEAP_SEGMENT_SIZE, tire)
        };
        for m in mems {
            m.leak();
        }
        Ok(heap_frame)
    }

    #[inline(always)]
    pub fn try_alloc_large(&self, size: usize, segment_length: usize, tire: usize) -> AllocResult<NonNull<[u8]>> {
        let vm = VM::alloc(segment_length * size * tire as usize)?;
        let memory = VM::new(vm.as_non_null_slice_ptr()).create_shared_memory();
        let mut mems = Vec::new(); // 实现错误时自动unmap
        for m in 0..tire {
            unsafe {
                mems.push(memory.map(VM::new(NonNull::slice_from_raw_parts(
                    NonNull::new_unchecked(vm.as_ptr().offset((segment_length * size * m as usize).try_into().unwrap())),
                    segment_length,
                )))?);
            }
        }
        Ok(vm.as_non_null_slice_ptr())
    }

    #[inline(always)]
    pub fn alloc_multiple(&self, smallest_page_count: usize, tire: usize) -> AllocResult<Vec<HeapFrameRef>> {
        let frame_count = usize::saturating_sub(smallest_page_count, 1) / PAGES_PRE_SEGMENT + 1;
        let mut vec = Vec::with_capacity(frame_count);
        for _ in 0..frame_count {
            vec.push(self.alloc(tire)?);
        }
        Ok(vec)
    }
}
#[derive(Default)]
pub struct GlobalHeap {
    pools: DashMap<Arc<RegistedType>, Arc<GlobalSingleTypeHeapPool>>,
    heap_pages: DashMap<usize, Vec<NonNull<HeapPage>>>,
    large_frames: DashMap<(usize, usize), BTreeMap<usize, Vec<HeapFrameRef>>>,
    allocator: GlobalHeapFrameAllocator,
}

unsafe impl Sync for GlobalHeap {}
unsafe impl Send for GlobalHeap {}

impl GlobalHeap {
    #[inline(always)]
    pub fn get_heap_pages(&self, count: usize, tire: usize) -> AllocResult<Vec<NonNull<HeapPage>>> {
        let tire = tire.max(1);
        let (alloc_count, mut vec) = if let Entry::Occupied(mut entity) = self.heap_pages.entry(tire) {
            let cached = entity.get_mut();
            let len = cached.len();
            let split_off_count = usize::max(0, len - count);
            (count - split_off_count, cached.split_off(split_off_count))
        } else {
            (count, Vec::with_capacity(count))
        };
        if alloc_count > 0 {
            let frames = self.allocator.alloc_multiple(alloc_count, tire)?;
            let len = frames.len();
            let mut alloced_pages_iter = frames.into_iter().flat_map(|f| f.into_first_segment_pages());
            if len > alloc_count {
                match self.heap_pages.entry(tire) {
                    Entry::Occupied(mut entry) => {
                        let pages = entry.get_mut();
                        let insert_len = len - alloc_count;
                        for _ in 0..insert_len {
                            pages.push(alloced_pages_iter.next().unwrap());
                        }
                    }
                    Entry::Vacant(entry) => {
                        let insert_len = len - alloc_count;
                        let mut needless = Vec::with_capacity(insert_len);
                        for _ in 0..insert_len {
                            needless.push(alloced_pages_iter.next().unwrap());
                        }
                        entry.insert(needless);
                    }
                }
            }
            vec.extend(alloced_pages_iter);
        }
        Ok(vec)
    }

    fn get_large_heap(&self, size: usize, segment_length: usize, tire: usize) -> AllocResult<HeapFrameRef> {
        let mut pool = self.large_frames.entry((segment_length, tire)).or_insert_with(Default::default);
        for (_size, frames) in pool.range_mut(size..usize::MAX) {
            if let Some(frame) = frames.pop() {
                return Ok(frame);
            }
        }
        let frame = self.allocator.try_alloc_large(size, segment_length, tire)?;
        Ok(unsafe { HeapFrameRef::new(frame, segment_length, tire) })
    }
}
#[derive(Default)]
pub struct GlobalSingleTypeHeapPool {
    pub(crate) small_heaps: Mutex<SmallHeap>,
    pub(crate) large_heaps: Mutex<LargeHeap>,
}
#[derive(Default)]
pub struct SmallHeap {
    pub(crate) allocable: Vec<SingleTypeHeapRef>,
    pub(crate) full: Vec<SingleTypeHeapRef>,
}

#[derive(Default)]
pub struct LargeHeap {
    pub(crate) allocable: Vec<SingleTypeHeapRef>,
    pub(crate) full: Vec<SingleTypeHeapRef>,
}
unsafe impl Send for GlobalSingleTypeHeapPool {}
unsafe impl Sync for GlobalSingleTypeHeapPool {}
impl GlobalSingleTypeHeapPool {
    #[inline(always)]
    pub fn new_local(&self, layout: TypeLayout, ty: &RegistedType) -> AllocResult<LocalSingleTypeHeapPool> {
        let heaps = self.get(layout, ty, 1);
        Ok(LocalSingleTypeHeapPool::from_vec(heaps?))
    }

    #[inline(always)]
    pub fn get(&self, layout: TypeLayout, ty: &RegistedType, count: usize) -> AllocResult<Vec<SingleTypeHeapRef>> {
        assert_ne!(count, 0);
        let mut guard = self.small_heaps.lock().map_err(|_e| format_err!("PoisonError"))?;
        let free_page = &mut guard.allocable;
        let available_count = free_page.len();
        if available_count < count {
            ty.to_ready_state()?;
            free_page.extend(
                MemoryMMMU::get_instance()
                    .heap()
                    .get_heap_pages(count - available_count, layout.tire())?
                    .into_iter()
                    .map(|p| unsafe { SingleTypeHeapRef::new(NonNull::slice_from_raw_parts(p.cast(), HEAP_PAGE_SIZE), layout, ty, AllocationStrategy::Small) }),
            ); // TODO: 优化动态分配数量
        }
        let len = free_page.len();
        // let mut used_page = & guard.1;
        // used_page.extend(guard.0[..len-count].iter().cloned());
        let result = free_page.split_off(len - count);
        assert_eq!(result.len(), count);
        Ok(result)
    }

    #[inline(always)]
    fn alloc(&self, layout: TypeLayout, ty: &RegistedType, flexible_len: usize) -> AllocResult<NonNull<u8>> {
        let size = layout.size() + layout.flexible_size() * flexible_len;
        let size = (size + (HEAP_SEGMENT_SIZE - 1)) / HEAP_SEGMENT_SIZE * HEAP_SEGMENT_SIZE;
        let mut large_heap = self.large_heaps.lock().unwrap();
        if large_heap.allocable.is_empty() {
            ty.to_ready_state()?;
            let global = MemoryMMMU::get_instance().heap();
            let heap_ptr = global.get_large_heap(size, ty.segment_size()?, layout.tire())?;
            unsafe {
                large_heap.allocable.push(SingleTypeHeapRef::new(heap_ptr.as_ptr(), layout, ty, AllocationStrategy::Large));
            }
        }
        for i in large_heap.allocable.iter().rev() {
            if let Some(r) = unsafe { i.alloc(layout) } {
                return Ok(r);
            }
        }
        Err(AllocError::NoSpaceLeft())
    }
}
pub struct LocalHeapPool {
    pools: HashMap<CowArc<'static, RegistedType>, LocalSingleTypeHeapPool>,
}
impl LocalHeapPool {
    #[inline(always)]
    fn get_single_type_memory_pools(&mut self, ty: &RegistedType, layout: TypeLayout) -> AllocResult<&mut LocalSingleTypeHeapPool> {
        if !self.pools.contains_key(ty) {
            let pool = ty.heap().new_local(layout, ty)?;
            self.pools.insert(ty.weak_self.upgrade().unwrap(), pool);
        }
        Ok(self.pools.get_mut(ty).unwrap())
    }
}
pub struct LocalSingleTypeHeapPool {
    available: CacheAlignedVec<SingleTypeHeapRef>,
    full: Vec<SingleTypeHeapRef>,
}
impl LocalSingleTypeHeapPool {
    #[inline(always)]
    pub fn from_vec(heaps: Vec<SingleTypeHeapRef>) -> Self {
        let mut available = CacheAlignedVec::<SingleTypeHeapRef>::new();
        available.extend(heaps);
        LocalSingleTypeHeapPool { available, full: Vec::new() }
    }

    #[inline(always)]
    pub unsafe fn get_one_available(&mut self, ty: &RegistedType, layout: TypeLayout) -> AllocResult<&mut SingleTypeHeapRef> {
        if self.available.is_empty() {
            let global = ty.heap();
            let heaps = global.get(layout, ty, 1 + (self.full.len() >> 3))?; // TODO: 优化动态分配数量
            self.available.extend(heaps);
        }
        Ok(self.available.last_mut().unwrap())
    }

    pub fn mark_one_full(&mut self) {
        let pool = self.available.pop().unwrap();
        self.full.push(pool);
    }
}
#[inline(always)]
fn new_local_heap_pool() -> LocalHeapPool {
    LocalHeapPool { pools: HashMap::new() }
}
#[inline(always)]
fn new_global_heap() -> GlobalHeap {
    GlobalHeap { pools: DashMap::new(), heap_pages: DashMap::new(), allocator: Default::default(), large_frames: DashMap::new() }
}
thread_local! {
    static LOCAL_HEAP_POOL:RefCell<LocalHeapPool>=RefCell::new(new_local_heap_pool());
}
lazy_static! {
    static ref GLOBALH_EAP: Arc<GlobalHeap> = Arc::new(new_global_heap());
}
// pub(crate) static GLOBALH_EAP: GlobalHeap = new_global_heap();
#[inline(always)]
pub fn try_alloc<'a>(ty: &RegistedType) -> AllocResult<NonNull<u8>> {
    unsafe {
        match ty.allocation_strategy.load() {
            AllocationStrategy::Small | AllocationStrategy::SmallUnsized => LOCAL_HEAP_POOL.with(|this| {
                let mut this = this.borrow_mut();
                let layout = ty.get_layout()?;
                let pools = this.get_single_type_memory_pools(ty, layout)?;
                let pool = pools.get_one_available(ty, layout)?;
                if let Some(oop) = pool.alloc(layout) {
                    if pool.is_full() {
                        pools.mark_one_full();
                    }
                    Ok(oop)
                } else {
                    Err(NoSpaceLeft())
                }
            }),
            AllocationStrategy::Large => {
                let pool = &ty.heap;
                let layout = ty.get_layout()?;
                pool.alloc(layout, ty, 0)
            }
        }
    }
}
#[inline(always)]
pub fn try_alloc_unsized<'a>(ty: &RegistedType, len: usize) -> AllocResult<NonNull<u8>> {
    unsafe {
        match ty.allocation_strategy.load() {
            AllocationStrategy::Small | AllocationStrategy::SmallUnsized => LOCAL_HEAP_POOL.with(|this| {
                let mut this = this.borrow_mut();
                let layout = ty.get_layout()?;
                let pools = this.get_single_type_memory_pools(ty, layout)?;
                loop {
                    let pool = pools.get_one_available(ty, layout)?;
                    if let Some(oop) = pool.alloc_unsized(layout, len) {
                        if pool.is_full() {
                            pools.mark_one_full();
                        }
                        return Ok(oop);
                    } else {
                        pools.mark_one_full();
                    }
                }
            }),
            AllocationStrategy::Large => {
                let pool = &ty.heap;
                let layout = ty.get_layout()?;
                pool.alloc(layout, ty, len)
            }
        }
    }
}
