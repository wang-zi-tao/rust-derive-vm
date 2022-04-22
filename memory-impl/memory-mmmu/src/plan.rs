use std::{
    collections::{BinaryHeap, HashSet},
};

use crossbeam::atomic::AtomicCell;

use rand::Rng;
use smallvec::SmallVec;
use util::{CowArc, ToNode};

use crate::{gc::GCPlan, graph::GRAPH_HANDLE, MemoryMMMU, RegistedType};

#[derive(Default)]
pub struct TypeStatistice {
    pub(crate) live: AtomicCell<usize>,
    pub(crate) live_rate: AtomicCell<f64>,
    pub(crate) alloc_count: AtomicCell<usize>,
    pub(crate) large_heap_size: AtomicCell<usize>,
    pub(crate) small_heap_size: AtomicCell<usize>,
    pub(crate) walk_count: AtomicCell<usize>,
}
pub struct ReferenceStatistice {
    rate: AtomicCell<f64>,
}

impl Default for ReferenceStatistice {
    fn default() -> Self {
        Self { rate: AtomicCell::new(1.0) }
    }
}

impl Clone for ReferenceStatistice {
    fn clone(&self) -> Self {
        Self { rate: AtomicCell::new(self.rate.load()) }
    }
}

pub struct EmbedStatistice {
    rate: AtomicCell<f64>,
}
impl Default for EmbedStatistice {
    fn default() -> Self {
        Self { rate: AtomicCell::new(1.0) }
    }
}
impl Clone for EmbedStatistice {
    fn clone(&self) -> Self {
        Self { rate: AtomicCell::new(self.rate.load()) }
    }
}
pub const RECYCLE_TYPE_COUNT: usize = 16;
pub const WALK_STEP_COUNT: usize = 16;
pub const HISTORY_WEIGHT: f64 = 0.5;
pub fn make_plan() -> GCPlan {
    GRAPH_HANDLE.with(|graph_handle| {
        let reference_graph = graph_handle.references_graph.read();
        let _assign_graph = graph_handle.assign_graph.read();
        let memory = MemoryMMMU::get_instance();
        let mut clean_types = HashSet::with_capacity(RECYCLE_TYPE_COUNT);
        let mut scan_types = HashSet::with_capacity(RECYCLE_TYPE_COUNT);
        if memory.types().len() <= RECYCLE_TYPE_COUNT {
            clean_types.extend(memory.types().iter().map(|t| t.clone()));
            scan_types.extend(clean_types.iter().cloned());
        } else {
            let mut rng = rand::thread_rng();
            let start_type = if let Some(start_type) = memory
                .types()
                .iter()
                .map(|t| {
                    t.statistice.walk_count.store((t.statistice.walk_count.load() as f64 * HISTORY_WEIGHT) as usize);
                    t
                })
                .max_by_key(|n| (n.statistice.live_rate.load() * n.statistice.live.load() as f64) as usize)
            {
                start_type
            } else {
                return GCPlan::default();
            };
            let mut candidate_clean_types = BinaryHeap::new();
            candidate_clean_types.push(TypeWrapper(start_type.clone()));
            for _ in 1..RECYCLE_TYPE_COUNT {
                let mut node = start_type.clone();
                for _ in 0..WALK_STEP_COUNT {
                    node.statistice.walk_count.fetch_add(1);
                    let mut weight_sum = 0.0;
                    let mut weight_vec = SmallVec::<[f64; 8]>::new();
                    for target in node.input(&reference_graph).iter() {
                        let weight = target.edge_statistice.weight(&node.statistice, &target.statistice);
                        weight_vec.push(weight);
                        weight_sum += weight;
                    }
                    let random: f64 = rng.gen();
                    let random = weight_sum * random;
                    let mut weight_sum = 0.0;
                    for (target, weight) in node.input(&reference_graph).iter().zip(&weight_vec) {
                        weight_sum += weight;
                        if weight_sum > random {
                            node = target.reference.clone();
                            break;
                        }
                    }
                    candidate_clean_types.push(TypeWrapper(node.clone()));
                }
            }
            for _i in 0..RECYCLE_TYPE_COUNT {
                if let Some(recycle_type) = candidate_clean_types.pop() {
                    clean_types.insert(recycle_type.0);
                }
            }
            for recycle_type in clean_types.iter() {
                scan_types.extend(recycle_type.input(&reference_graph).iter().map(|f| f.reference.clone()));
            }
        }
        GCPlan { clean_types, scan_types }
    })
}
impl ReferenceStatistice {
    fn weight(&self, _to: &TypeStatistice, from: &TypeStatistice) -> f64 {
        from.walk_count() as f64 / (from.alloc_count() as f64 * from.live_rate() * self.rate())
    }

    fn rate(&self) -> f64 {
        self.rate.load()
    }
}
impl TypeStatistice {
    fn alloc_count(&self) -> usize {
        self.alloc_count.load()
    }

    fn walk_count(&self) -> usize {
        self.walk_count.load()
    }

    fn live_rate(&self) -> f64 {
        self.live_rate.load()
    }
}
struct TypeWrapper(CowArc<'static, RegistedType>);
impl PartialEq for TypeWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl Eq for TypeWrapper {}
impl PartialOrd for TypeWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for TypeWrapper {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.statistice.walk_count().cmp(&other.0.statistice.walk_count()).then(self.0.cmp(&other.0))
    }
}
