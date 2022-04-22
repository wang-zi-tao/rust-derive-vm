use failure::{format_err, Error, Fallible};
use memfd::MemfdOptions;
use mmap::{MapOption, MemoryMap};
use std::{
    ascii::AsciiExt,
    collections::BTreeSet,
    fs::File,
    io::{stdout, Write},
    os::unix::prelude::{FromRawFd, IntoRawFd},
    ptr::NonNull,
    sync::{Arc, Mutex},
};
pub const VM_PAGE_SIZE: usize = 4096;
pub const VM_ALLOC_START: usize = 0x3000_0000_0000;
pub const MINIMUMALLOCATION_UNIIT_OF_VM: usize = 1 << 12;
pub const TOTAL_VM: usize = 1 << 47;
pub struct Page {}
pub struct VM {
    ptr: NonNull<[u8]>,
}
impl VM {
    pub fn alloc(len: usize) -> Fallible<Self> {
        VIRTUAL_MEMORY_MANAGER.alloc(len)
    }

    pub unsafe fn alloc_unchecked(len: usize) -> Fallible<Self> {
        VIRTUAL_MEMORY_MANAGER.alloc_unchecked(len)
    }

    pub fn new(ptr: NonNull<[u8]>) -> Self {
        Self { ptr }
    }

    pub fn len(&self) -> usize {
        self.ptr.len()
    }

    pub fn as_ptr(&self) -> *mut u8 {
        self.ptr.as_ptr().as_mut_ptr()
    }

    pub fn as_non_null(&self) -> NonNull<u8> {
        self.ptr.as_non_null_ptr()
    }

    pub fn as_non_null_slice_ptr(&self) -> NonNull<[u8]> {
        self.ptr
    }

    pub fn map(&self) -> Fallible<MappedVM> {
        let mmap = MemoryMap::new(self.len(), &[MapOption::MapReadable, MapOption::MapWritable, MapOption::MapExecutable, MapOption::MapAddr(self.as_ptr())])
            .map_err(|e| format_err!("mmap failed,address:{:p},length:{},message:{}", self.as_ptr(), self.len(), e))?;
        Ok(MappedVM::new(mmap))
    }

    pub fn create_shared_memory(&self) -> SharedMemory {
        SharedMemory { offset: self.as_ptr() as usize, len: self.len() }
    }
}
impl Drop for VM {
    fn drop(&mut self) {
        let size = self.len();
        let address = self.as_ptr() as usize;
        let position = address / MINIMUMALLOCATION_UNIIT_OF_VM;
        let unit_count = size / MINIMUMALLOCATION_UNIIT_OF_VM;
        let height = usize::trailing_zeros(unit_count);
        VIRTUAL_MEMORY_MANAGER.buddy.lock().map_err(|_| format_err!("PoisonError")).unwrap().free(height, position);
    }
}
pub struct MappedVM {
    mmap: MemoryMap,
}
impl Drop for MappedVM {
    fn drop(&mut self) {
        let _vm = VM::new(self.as_non_null_slice_ptr());
    }
}
impl MappedVM {
    pub fn new(mmap: MemoryMap) -> Self {
        Self { mmap }
    }

    pub fn as_non_null_slice_ptr(&self) -> NonNull<[u8]> {
        NonNull::slice_from_raw_parts(NonNull::new(self.mmap.data()).unwrap(), self.mmap.len())
    }

    pub fn len(&self) -> usize {
        self.mmap.len()
    }

    pub fn unmap(self) -> VM {
        VM { ptr: self.as_non_null_slice_ptr() }
    }

    pub fn leak(self) -> NonNull<[u8]> {
        let ret = self.as_non_null_slice_ptr();
        std::mem::forget(self);
        ret
    }

    pub unsafe fn clean(&self) -> Fallible<()> {
        Self::clean_raw(self.as_non_null_slice_ptr())
    }

    pub unsafe fn clean_raw(ptr: NonNull<[u8]>) -> Fallible<()> {
        let ret = libc::madvise(ptr.as_non_null_ptr().as_ptr().cast(), ptr.len(), libc::MADV_REMOVE);
        if ret != 0 {
            Err(match std::io::Error::last_os_error().raw_os_error().unwrap_or(-1) {
                libc::EACCES => format_err!("advice is MADV_REMOVE, but the specified address range is not a shared writable mapping."),
                libc::EAGAIN => format_err!("A kernel resource was temporarily unavailable."),
                libc::EBADF => format_err!("The map exists, but the area maps something that isn't a file."),
                libc::EINVAL => format_err!("addr is not page-aligned or length is negative or advice is not a valid."),
                libc::EIO => format_err!("(for MADV_WILLNEED) Paging in this area would exceed the process's maximum resident set size."),
                libc::ENOMEM => format_err!("(for MADV_WILLNEED) Not enough memory: paging in failed."),
                libc::EPERM => format_err!("dvice is MADV_HWPOISON, but the caller does not have the CAP_SYS_ADMIN capability."),
                o => format_err!("unknown error on libc::madvise : return value is {}", o),
            })
        } else {
            Ok(())
        }
    }
}
pub struct SharedMemory {
    offset: usize,
    len: usize,
}
impl SharedMemory {
    pub fn new_uncheckd(offset: usize, len: usize) -> Self {
        Self { offset, len }
    }

    pub fn map(&self, vm: VM) -> Fallible<MappedVM> {
        assert_eq!(vm.len(), self.len);
        let mmap = MemoryMap::new(
            self.len,
            &[
                MapOption::MapNonStandardFlags(libc::MAP_SHARED | libc::MAP_FIXED),
                MapOption::MapReadable,
                MapOption::MapWritable,
                MapOption::MapExecutable,
                MapOption::MapFd(SHARED_MEMORY_MANAGER.fd),
                MapOption::MapOffset(self.offset),
                MapOption::MapAddr(vm.as_ptr()),
            ],
        )
        .map_err(|e| format_err!("mmap failed,address:{:p},length:{},message:{}", vm.as_ptr(), vm.len(), e))?;
        Ok(MappedVM::new(mmap))
    }

    pub unsafe fn unmap(&self, memory: MappedVM) -> Fallible<VM> {
        Ok(VM::new(memory.as_non_null_slice_ptr()))
    }

    pub fn leak(self) -> (usize, usize) {
        (self.offset, self.len)
    }
}
lazy_static! {
    static ref SHARED_MEMORY_MANAGER: SharedMemoryManager = SharedMemoryManager::new();
}
pub struct SharedMemoryManager {
    fd: i32,
}
impl SharedMemoryManager {
    pub fn new() -> Self {
        let mem_fd = MemfdOptions::new().close_on_exec(true).allow_sealing(true).create("").unwrap();
        let mem_file = mem_fd.into_file();
        mem_file.set_len(TOTAL_VM as u64).unwrap();
        Self { fd: mem_file.into_raw_fd() }
    }
}
impl Drop for SharedMemoryManager {
    fn drop(&mut self) {
        unsafe {
            File::from_raw_fd(self.fd);
        }
    }
}
pub struct VirtualMemoryManager {
    buddy: Mutex<Buddy>,
    start: usize,
}
lazy_static! {
    static ref VIRTUAL_MEMORY_MANAGER: Arc<VirtualMemoryManager> = Arc::new(VirtualMemoryManager::new());
}
impl VirtualMemoryManager {
    fn new() -> Self {
        Self { buddy: Mutex::new(Buddy::new()), start: VM_ALLOC_START }
    }

    fn alloc(&self, minimum_size: usize) -> Result<VM, Error> {
        let size = VirtualMemoryManager::align_size(minimum_size);
        unsafe { self.alloc_unchecked(size) }
    }

    unsafe fn alloc_unchecked(&self, size: usize) -> Result<VM, Error> {
        let unit_count = size / MINIMUMALLOCATION_UNIIT_OF_VM;
        let height = usize::trailing_zeros(unit_count);
        let position = self.buddy.lock().map_err(|_| format_err!("PoisonError"))?.alloc(height).ok_or_else(|| format_err!("out of memory"))?;
        let address = NonNull::new_unchecked((position * MINIMUMALLOCATION_UNIIT_OF_VM + self.start) as *mut u8);
        Ok(VM::new(NonNull::slice_from_raw_parts(address, size)))
    }

    fn align_size(size: usize) -> usize {
        let mut align = (size + (MINIMUMALLOCATION_UNIIT_OF_VM - 1)) & !(MINIMUMALLOCATION_UNIIT_OF_VM - 1);
        align |= align >> 1;
        align |= align >> 2;
        align |= align >> 4;
        align |= align >> 8;
        align |= align >> 16;
        align |= align >> 32;
        (align + 1) * MINIMUMALLOCATION_UNIIT_OF_VM
    }
}
const ALLOC_TREE_HEIGH: u32 = usize::trailing_zeros(TOTAL_VM / MINIMUMALLOCATION_UNIIT_OF_VM);
#[derive(Default)]
struct Buddy {
    levels: Vec<BTreeSet<usize>>,
}
impl Buddy {
    pub fn new() -> Buddy {
        let mut levels: Vec<_> = (0..ALLOC_TREE_HEIGH).map(|_| BTreeSet::new()).collect();
        levels.last_mut().map(|t| t.insert(0));
        Self { levels }
    }

    pub fn alloc(&mut self, height: u32) -> Option<usize> {
        if height >= ALLOC_TREE_HEIGH {
            return None;
        }
        let level = &mut self.levels[height as usize];
        if let Some(&first) = level.iter().next() {
            level.remove(&first);
            return Some(first);
        }
        let high_level_position = self.alloc(height + 1)?;
        (&mut self.levels[height as usize]).insert(high_level_position | (1 << height));
        return Some(high_level_position);
    }

    pub fn free(&mut self, height: u32, position: usize) {
        if height < ALLOC_TREE_HEIGH {
            let level = &mut self.levels[height as usize];
            if let Some(_) = level.take(&(position ^ (1 << height))) {
                self.free(height + 1, position & !(1 << height))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{stderr, Write},
        ptr,
        time::Duration,
    };

    use super::*;
    use failure::Fallible;
    #[test]
    pub fn alloc_vm() -> Fallible<()> {
        let vm = VM::alloc(1 * 4096)?;
        assert_ne!(vm.as_ptr(), ptr::null_mut());
        assert_eq!(vm.len(), 1 * 4096);
        Ok(())
    }
    #[test]
    pub fn alloc_vm_multiply() -> Fallible<()> {
        for l in [1, 6, 3, 7, 34, 6, 23, 7] {
            let vm = VM::alloc(l * 4096)?;
            assert_eq!(usize::leading_ones(vm.len()), usize::leading_ones(l * 4096));
        }
        let vm = VM::alloc(15 * 4096)?;
        assert_ne!(vm.as_ptr(), ptr::null_mut());
        assert_eq!(vm.len(), 16 * 4096);
        Ok(())
    }
    #[test]
    pub fn alloc() -> Fallible<()> {
        let vm = VM::alloc(4 * 4096)?;
        let mem = vm.map()?;
        let ptr = mem.as_non_null_slice_ptr();
        unsafe {
            ptr.as_non_null_ptr().as_ptr().write(127);
        }
        assert_eq!(unsafe { ptr.as_non_null_ptr().as_ptr().read() }, 127);
        Ok(())
    }
    #[test]
    pub fn free() -> Fallible<()> {
        let vm = VM::alloc(4 * 4096)?;
        let mem = vm.map()?;
        let _vm = mem.unmap();
        Ok(())
    }
    #[test]
    pub fn multiple_mmap() -> Fallible<()> {
        let len: usize = 4 * 1024 * 1024 * 1024;
        let vm = VM::alloc(len)?;
        let shared = vm.create_shared_memory();
        let mut v = Vec::new();
        for _ in 0..16 {
            let vm = VM::alloc(len)?;
            assert_eq!(vm.len(), len);
            v.push(shared.map(vm)?);
        }
        let mut meter = self_meter::Meter::new(Duration::new(0, 1000)).unwrap();
        meter.track_current_thread("main");
        meter.scan().map_err(|e| writeln!(&mut stderr(), "Scan error: {}", e)).ok();
        let mut mem_usage = || -> Fallible<u64> {
            meter.scan().map_err(|e| writeln!(&mut stderr(), "Scan error: {}", e)).ok();
            println!("Report: {:#?}", meter.report());
            Ok(meter.report().unwrap().memory_rss)
        };
        let mem1 = mem_usage()?;
        let ptr = v[0].as_non_null_slice_ptr();
        unsafe {
            ptr.as_non_null_ptr().as_ptr().write(101);
            // v[0].data().offset(len as isize - 64).write(102);
            for mmap in &v {
                assert_eq!(mmap.as_non_null_slice_ptr().as_non_null_ptr().as_ptr().read(), 101);
            }
            for i in 0..len / 4096 {
                ptr.as_non_null_ptr().as_ptr().offset(4096 * i as isize).write(i as u8);
            }
        }
        let mem2 = mem_usage()?;
        assert!(1024 * 1024 * 4 > i64::abs((mem2 - mem1) as i64 - (len) as i64));
        unsafe {
            let ret = libc::madvise(ptr.as_non_null_ptr().as_ptr().cast(), len / 2, libc::MADV_REMOVE);
            if ret != 0 {
                panic!();
            }
        }
        let mem3 = mem_usage()?;
        assert!(1024 * 1024 * 4 > i64::abs((mem2 - mem3) as i64 - (len / 2) as i64));
        std::mem::drop((v, vm, shared));
        let mem4 = mem_usage()?;
        assert!(1024 * 1024 * 4 > i64::abs(mem4 as i64 - (mem1 as i64)));
        Ok(())
    }
}
