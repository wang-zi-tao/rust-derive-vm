#![feature(nonnull_slice_from_raw_parts)]
#![feature(int_roundings)]
#![feature(slice_ptr_len)]
#![feature(negative_impls)]
#![feature(slice_ptr_get)]
#![feature(ptr_internals)]
#![allow(incomplete_features)]
#![feature(inherent_associated_types)]
extern crate dashmap;
extern crate failure;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate getset;
#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate util_derive;
pub(crate) mod alloc;
pub(crate) mod gc;
pub(crate) mod graph;
pub(crate) mod heap;
pub(crate) mod layout;
pub(crate) mod mark;
pub(crate) mod metadata;
pub(crate) mod object;
pub(crate) mod plan;
pub(crate) mod references;
pub(crate) mod runtime_instruction_set;
pub(crate) mod scanner;
use std::{
    fmt::{Debug, Formatter},
    hash::Hash,
    mem::size_of,
    ptr::NonNull,
    sync::Arc,
};

use alloc::{GlobalHeap, GlobalSingleTypeHeapPool};
use crossbeam::atomic::AtomicCell;
use dashmap::{DashSet, SharedValue};
use failure::{format_err, Fallible};
use graph::{scan_assign, scan_reference, AssignGraph, ReferenceGraph, TypeAssignEdge, TypeReferenceEdge, GRAPH_HANDLE};
use heap::{AllocationStrategy, HEAP_LARGE_SEGMENT_SIZE, HEAP_PAGE_SIZE, HEAP_SEGMENT_SIZE};
use mark::GlobalMarkSet;
use metadata::MetadataList;
use plan::TypeStatistice;
use runtime::mem::MemoryInstructionSetProvider;
use runtime_instruction_set::MEMORY_INSTRUCTION_SET;
use util::{AtomicLazyArc, CowArc, CowWeak, DefaultArc, EmbedGraph};
use vm_core::{
    Component, MemoryTrait, Module, OOPTrait, Resource, ResourceConverter, ResourceError, ResourceFactory, ResourceState, Singleton, SingletonDyn, Tuple, Type,
    TypeLayout, TypeResource,
};

#[derive(Default, Getters)]
#[getset(get = "pub(crate)")]
pub struct MemoryMMMU {
    heap: GlobalHeap,
    types: DashSet<CowArc<'static, RegistedType>>,
    markset: GlobalMarkSet,
}

impl MemoryMMMU {
    fn new() -> Self {
        Default::default()
    }

    #[inline(always)]
    pub fn get_instance<'l>() -> &'l Self {
        &MMMU
    }
}
impl Debug for MemoryMMMU {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
lazy_static! {
    static ref MMMU: MemoryMMMU = MemoryMMMU::new();
}
impl Singleton for MemoryMMMU {
    fn get_instance<'l>() -> &'l Self {
        &MMMU
    }
}

impl Module for MemoryMMMU {}
impl ResourceConverter<Type, RegistedType> for MemoryMMMU {
    fn define(&self) -> failure::Fallible<Arc<RegistedType>> {
        Ok(RegistedType::new())
    }

    fn upload(&self, resource: &RegistedType, ty: Type) -> failure::Fallible<()> {
        resource.upload(ty)
    }
}
impl ResourceFactory<Type> for MemoryMMMU {
    type ResourceImpl = RegistedType;
}
impl MemoryTrait for MemoryMMMU {
    fn alloc(&self, ty: &Arc<(dyn TypeResource + 'static)>) -> Fallible<NonNull<u8>> {
        Ok(crate::alloc::try_alloc(RegistedType::try_downcast(&**ty)?)?)
    }

    fn alloc_unsized(&self, _type_layout: &Arc<dyn TypeResource>, _size: usize) -> failure::Fallible<Box<dyn OOPTrait>> {
        todo!()
    }
}
impl MemoryInstructionSetProvider for MemoryMMMU {
    fn get_memory_instruction_set() -> failure::Fallible<CowArc<'static, runtime::instructions::MemoryInstructionSet>> {
        Ok(CowArc::new(MEMORY_INSTRUCTION_SET))
    }
}
impl MemoryMMMU {}

#[derive(AsAny, Getters)]
#[getset(get = "pub(crate)")]
pub struct RegistedType {
    pub(crate) weak_self: CowWeak<'static, Self>,
    pub(crate) ty: AtomicLazyArc<(Type, TypeLayout)>,
    pub(crate) assign_from: EmbedGraph<AssignGraph, TypeAssignEdge>,
    pub(crate) reference_from: EmbedGraph<ReferenceGraph, TypeReferenceEdge>,
    pub(crate) memory_pool: GlobalSingleTypeHeapPool,
    pub(crate) heap: GlobalSingleTypeHeapPool,
    pub(crate) allocation_strategy: AtomicCell<AllocationStrategy>,
    pub(crate) statistice: TypeStatistice,
    pub(crate) metas: MetadataList,
}
impl PartialOrd for RegistedType {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some((self as *const Self as usize).cmp(&(other as *const Self as usize)))
    }
}
impl Ord for RegistedType {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self as *const Self as usize).cmp(&(other as *const Self as usize))
    }
}
impl DefaultArc for RegistedType {
    fn default_arc() -> Arc<Self> {
        Self::new()
    }
}
impl Hash for RegistedType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (self as *const Self).hash(state)
    }
}

impl RegistedType {
    pub fn new() -> Arc<Self> {
        Arc::new_cyclic(|weak| Self {
            ty: AtomicLazyArc::new_uninitalized(),
            assign_from: EmbedGraph::new(),
            reference_from: EmbedGraph::new(),
            memory_pool: Default::default(),
            weak_self: CowWeak::Weak(weak.clone()),
            heap: Default::default(),
            allocation_strategy: AtomicCell::new(AllocationStrategy::new()),
            statistice: Default::default(),
            metas: Default::default(),
        })
    }

    pub fn try_downcast(r: &dyn TypeResource) -> Fallible<&Self> {
        r.as_any().downcast_ref().ok_or_else(|| format_err!("wrone type"))
    }

    pub fn try_get(&self) -> Fallible<&Type> {
        Ok(self.ty.load_option().map(|(ty, _layout)| ty).ok_or(ResourceError::NotInitialized)?)
    }

    pub(crate) fn after_alloc(&self) {
        self.statistice.alloc_count.fetch_add(1);
    }

    pub(crate) fn after_creat_heap(&self, size: usize) {
        self.statistice.large_heap_size.fetch_add(size);
    }

    pub(crate) fn get_len_offset(&self) -> Fallible<Option<usize>> {
        Ok(match self.get_type()? {
            Type::Tuple(Tuple::Normal(_t)) => Some(self.get_layout()?.size() - size_of::<usize>()),
            Type::Array(_, None) => Some(0),
            Type::Embed(e) => e.try_map(|t| RegistedType::try_downcast(&**t)?.get_len_offset())?,
            _ => None,
        })
    }
}
impl RegistedType {
    pub fn from_dyn_arc(arc: Arc<dyn TypeResource>) -> Fallible<Arc<RegistedType>> {
        arc.as_any_arc().downcast().map_err(|e| format_err!("not a {}:{:#?}", stringify!(Self), e))
    }

    pub fn to_ready_state(&self) -> Fallible<()> {
        if self.get_state().is_ready() {
            return Ok(());
        };
        if !self.get_state().is_loaded() {
            return Err(ResourceError::NotInitialized.into());
        };
        let ty = self.get_type()?;
        let this_arc = self.weak_self.upgrade().unwrap();
        let set = MemoryMMMU::get_instance().types();
        let sub_set = &set.shards()[set.determine_map(self)];
        {
            let mut sub_set_guard = sub_set.write();
            if self.get_state().is_ready() {
                return Ok(());
            };
            GRAPH_HANDLE.with(|graph_handle| {
                let mut references_write_handle = graph_handle.references_graph.write();
                scan_reference(ty, |r| {
                    references_write_handle.add_edge(
                        TypeReferenceEdge { reference: this_arc.clone(), edge_statistice: Default::default() },
                        r.weak_self.upgrade().ok_or(ResourceError::Dead)?,
                    );
                    Ok(())
                })?;
                let mut assign_write_handle = graph_handle.assign_graph.write();
                scan_assign(ty, |r| {
                    assign_write_handle.add_edge(
                        TypeAssignEdge { assign: this_arc.clone(), edge_statistice: Default::default() },
                        r.weak_self.upgrade().ok_or(ResourceError::Dead)?,
                    );
                    Ok(())
                })?;
                references_write_handle.flush();
                assign_write_handle.flush();
                Fallible::<()>::Ok(())
            })?;
            sub_set_guard.insert(this_arc, SharedValue::new(()));
        }
        Ok(())
    }
}
impl PartialEq for RegistedType {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for RegistedType {
    fn assert_receiver_is_total_eq(&self) {}
}
impl Debug for RegistedType {
    fn fmt(&self, _: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        todo!()
    }
}
impl Resource<Type> for RegistedType {
    fn get_state(&self) -> vm_core::ResourceState {
        if self.ty.is_loaded() {
            ResourceState::Ready
        } else {
            ResourceState::Defined
        }
    }

    fn upload(&self, ty: Type) -> Fallible<()> {
        let layout = ty.get_layout()?;
        if self.ty.is_loaded() {
            Err(format_err!("Resource is aready initiated"))?;
        }
        self.ty.init(Arc::new((ty, layout)));
        self.allocation_strategy.store(if layout.size() > HEAP_PAGE_SIZE / 8 {
            AllocationStrategy::Large
        } else if layout.flexible_size() == 0 {
            AllocationStrategy::Small
        } else {
            AllocationStrategy::SmallUnsized
        });
        Ok(())
    }
}
impl TypeResource for RegistedType {
    fn alloc(&self) -> Fallible<NonNull<u8>> {
        Ok(crate::alloc::try_alloc(self)?)
    }

    fn alloc_unsized(&self, len: usize) -> Fallible<NonNull<u8>> {
        Ok(crate::alloc::try_alloc_unsized(self, len)?)
    }

    unsafe fn free(&self, _oop: vm_core::OOPRef) -> Fallible<()> {
        self.to_ready_state()?;
        todo!()
    }

    fn get_type(&self) -> Fallible<&Type> {
        Ok(&self.ty.load_option().ok_or(ResourceError::NotInitialized)?.0)
    }

    fn get_layout(&self) -> Fallible<vm_core::TypeLayout> {
        Ok(self.ty.load_option().ok_or(ResourceError::NotInitialized)?.1)
    }

    fn page_size(&self) -> Fallible<usize> {
        self.ty.load_option().ok_or(ResourceError::NotInitialized)?;
        Ok(HEAP_PAGE_SIZE)
    }

    fn segment_size(&self) -> Fallible<usize> {
        self.ty.load_option().ok_or(ResourceError::NotInitialized)?;
        Ok(match self.allocation_strategy.load() {
            AllocationStrategy::Small | AllocationStrategy::SmallUnsized => HEAP_SEGMENT_SIZE,
            AllocationStrategy::Large => HEAP_LARGE_SEGMENT_SIZE,
        })
    }
}
impl Component for RegistedType {}
