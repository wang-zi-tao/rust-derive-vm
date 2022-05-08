use std::{alloc::Layout, ptr::NonNull};

use e::{Usize, U8};
use runtime::instructions::{bootstrap as b, Instruction, MemoryInstructionSet};
use runtime_derive::{make_instruction, make_native_function, Instruction};
use runtime_extra as e;
use vm_core::{Native, Pointer, TypeDeclaration, TypeResource};

use crate::RegistedType;

make_instruction! {Deref->fn(type_resource: Pointer<U8>,i:Pointer<U8>)->(o:Pointer<U8>){entry:{
    %o=b::Move<Pointer::<U8>::TYPE>(%i);
}}}
make_instruction! {Clone->fn(type_resource: Pointer<U8>,i:Pointer<U8>)->(o:Pointer<U8>){entry:{
    %o=b::Move<Pointer::<U8>::TYPE>(%i);
}}}
make_instruction! {Drop->fn(type_resource: Pointer<U8>,i:Pointer<U8>)->(){entry:{
}}}
#[make_native_function(AllocSized)]
pub unsafe extern "C" fn __memory_mmmu_lib_instruction_set_alloc(type_resource: Pointer<U8>) -> Pointer<U8> {
    let ty = type_resource.as_ptr().cast::<RegistedType>().as_ref().unwrap_unchecked();
    Pointer::new(crate::alloc::try_alloc(ty).unwrap().cast())
}
#[make_native_function(AllocUnsized)]
pub unsafe extern "C" fn __memory_mmmu_lib_instruction_set_alloc_unsized(type_resource: Pointer<U8>, len: Usize) -> Pointer<U8> {
    let ty = type_resource.as_ptr().cast::<RegistedType>().as_ref().unwrap_unchecked();
    Pointer::new(crate::alloc::try_alloc_unsized(ty, len.0).unwrap().cast())
}
#[make_native_function(Free)]
pub unsafe extern "C" fn __memory_mmmu_lib_instruction_set_free(_ty: Pointer<Native<RegistedType>>, _ptr: Pointer<U8>) {
    todo!();
}
#[make_native_function(NonGCAlloc)]
pub unsafe extern "C" fn __memory_mmmu_lib_instruction_set_alloc_in_non_gc_heap(type_resource: Pointer<U8>) -> Pointer<U8> {
    let ty = type_resource.as_ptr().cast::<RegistedType>().as_ref().unwrap_unchecked();
    let layout = ty.get_layout().unwrap();
    assert!(layout.flexible_size() == 0);
    let ptr = std::alloc::alloc(Layout::from_size_align_unchecked(layout.size(), layout.align()));
    Pointer::new(NonNull::new_unchecked(ptr).cast())
}
#[make_native_function(NonGCAllocUnsized)]
pub unsafe extern "C" fn __memory_mmmu_lib_instruction_set_alloc_unsized_in_non_gc_heap(type_resource: Pointer<U8>, len: Usize) -> Pointer<U8> {
    let ty = type_resource.as_ptr().cast::<RegistedType>().as_ref().unwrap_unchecked();
    let layout = ty.get_layout().unwrap();
    assert!(layout.flexible_size() != 0);
    let ptr = std::alloc::alloc(Layout::from_size_align_unchecked(layout.size() + layout.flexible_size() * len.0, layout.align()));
    Pointer::new(NonNull::new_unchecked(ptr).cast())
}
#[make_native_function(NonGCFree)]
pub unsafe extern "C" fn __memory_mmmu_lib_instruction_set_free_in_non_gc_heap(type_resource: Pointer<U8>, ptr: Pointer<U8>) {
    let ty = type_resource.as_ptr().cast::<RegistedType>().as_ref().unwrap_unchecked();
    let layout = ty.get_layout().unwrap();
    if layout.flexible_size() != 0 {
        let len_offset = ty.get_len_offset().unwrap().unwrap();
        let len = ptr.as_ptr_mut().cast::<u8>().add(len_offset).cast::<usize>().read();
        std::alloc::dealloc(ptr.as_ptr_mut().cast(), Layout::from_size_align_unchecked(layout.size() + len * layout.flexible_size(), layout.align()));
    } else {
        std::alloc::dealloc(ptr.as_ptr_mut().cast(), Layout::from_size_align_unchecked(layout.size(), layout.align()));
    }
}
pub(crate) const MEMORY_INSTRUCTION_SET: MemoryInstructionSet = MemoryInstructionSet {
    clone: Clone::INSTRUCTION_TYPE,
    drop: Drop::INSTRUCTION_TYPE,
    deref: Deref::INSTRUCTION_TYPE,
    alloc: AllocSized::INSTRUCTION_TYPE,
    alloc_unsized: AllocUnsized::INSTRUCTION_TYPE,
    free: Free::INSTRUCTION_TYPE,
    non_gc_alloc: NonGCAlloc::INSTRUCTION_TYPE,
    non_gc_alloc_unsized: NonGCAllocUnsized::INSTRUCTION_TYPE,
    non_gc_free: NonGCFree::INSTRUCTION_TYPE,
};
