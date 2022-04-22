use dashmap::DashMap;
use left_right::{Absorb, ReadGuard, ReadHandle, WriteHandle};
use std::{
    any::{Any, TypeId},
    cell::UnsafeCell,
    collections::HashSet,
    hash::Hash,
    marker::PhantomData,
    sync::{Arc, Mutex, MutexGuard},
};

// pub trait Node<G: Graph>: Send + Sync {
// type From: Clone + Hash + Eq;
// type To: Clone + Hash + Eq;
// fn get_input_edge_set(&self) -> Option<&EmbedGraph<G, Self::From>>;
// fn get_output_edge_set(&self) -> Option<&EmbedGraph<G, Self::To>>;
// }
#[derive(Default)]
pub struct EmbedGraph<G, T>
where
    T: Hash + Eq,
{
    edge_set: [UnsafeCell<HashSet<T>>; 2],
    _phantom_data: PhantomData<G>,
}

impl<G, T> EmbedGraph<G, T>
where
    T: Hash + Eq,
{
    pub fn new() -> Self {
        Self { edge_set: [UnsafeCell::new(HashSet::new()), UnsafeCell::new(HashSet::new())], _phantom_data: PhantomData }
    }
}
impl<G, T> EmbedGraph<G, T>
where
    T: Hash + Eq,
{
    fn get(&self, version: &GraphVersion) -> &HashSet<T> {
        unsafe { self.edge_set[version.0 & 1].get().as_ref().unwrap() }
    }

    #[allow(clippy::mut_from_ref)]
    unsafe fn get_mut<'l>(&'l self, version: &'l GraphVersion) -> &'l mut HashSet<T> {
        self.edge_set[version.0 & 1].get().as_mut().unwrap()
    }
}
impl<G, T> EmbedGraph<G, T> where T: Hash + Eq {}
unsafe impl<G, T> Send for EmbedGraph<G, T> where T: Hash + Eq {}
unsafe impl<G, T> Sync for EmbedGraph<G, T> where T: Hash + Eq {}
#[derive(Default)]
pub struct GraphVersion(usize);
impl Clone for GraphVersion {
    fn clone(&self) -> Self {
        GraphVersion(self.0 ^ 1)
    }
}
pub struct GraphLocalHandle<G: Graph> {
    read_handle: ReadHandle<GraphVersion>,
    graph_global_handle: Arc<GraphGlobalHandle<G>>,
}
impl<G: Graph> GraphLocalHandle<G> {
    pub fn new() -> Self {
        let graph_global_handle = GraphGlobalHandle::<G>::get_instant();
        let read_handle = graph_global_handle.read_handle.lock().unwrap().clone();
        Self { read_handle, graph_global_handle }
    }

    pub fn write<'l>(&'l self) -> GraphWriteHandle<'l, G> {
        GraphWriteHandle::new(&self.graph_global_handle)
    }

    pub fn read<'l>(&'l self) -> GraphReadGuard<'l, G> {
        GraphReadGuard(self.read_handle.enter().unwrap(), PhantomData)
    }
}
pub struct GraphWriteHandle<'l, G: Graph> {
    guard: MutexGuard<'l, (WriteHandle<GraphVersion, GraphOperation<G>>, ReadHandle<GraphVersion>)>,
    // read_guard:ReadGuard<'l,GraphVersion>,
}
impl<'l, G: Graph> GraphWriteHandle<'l, G> {
    fn new(stub: &'l GraphGlobalHandle<G>) -> Self {
        Self { guard: stub.write_handle.lock().unwrap() }
    }

    fn get_wirte_handle(&mut self) -> &mut WriteHandle<GraphVersion, GraphOperation<G>> {
        &mut self.guard.0
    }

    pub fn get_read_handle(&self) -> &ReadHandle<GraphVersion> {
        &self.guard.1
    }

    pub fn flush(&mut self) {
        self.get_wirte_handle().flush()
    }

    pub fn add_operation(&mut self, op: GraphOperation<G>) {
        self.get_wirte_handle().append(op);
    }

    pub fn add_edge(&mut self, from: G::From, to: G::To) {
        self.add_operation(GraphOperation::AddEdge { from, to })
    }

    pub fn remove_edge(&mut self, from: G::From, to: G::To) {
        self.add_operation(GraphOperation::RemoveEdge { from, to })
    }
}

pub struct GraphGlobalHandle<G: Graph> {
    read_handle: Mutex<ReadHandle<GraphVersion>>,
    write_handle: Mutex<(WriteHandle<GraphVersion, GraphOperation<G>>, ReadHandle<GraphVersion>)>,
    _phantom_data: PhantomData<G>,
}
lazy_static! {
    static ref STUBS: DashMap<TypeId, Arc<dyn Any + Send + Sync>> = DashMap::new();
}
impl<G: Graph> GraphGlobalHandle<G> {
    pub fn get_instant() -> Arc<Self> {
        let b = STUBS.entry(TypeId::of::<G>()).or_insert_with(|| Arc::new(Self::new()));
        Arc::downcast(b.clone()).unwrap()
    }
}
impl<G: Graph> GraphGlobalHandle<G> {
    fn new() -> Self {
        let (w, r) = left_right::new_from_empty(GraphVersion::default());
        Self { read_handle: Mutex::new(r.clone()), write_handle: Mutex::new((w, r)), _phantom_data: PhantomData }
    }
}
pub struct GraphReadGuard<'l, G: Graph>(ReadGuard<'l, GraphVersion>, PhantomData<G>);
impl<'l, G: Graph> GraphReadGuard<'l, G> {}
pub trait GraphStubProvider: Graph {
    fn get_global_stub() -> Arc<GraphGlobalHandle<Self>> {
        GraphGlobalHandle::<Self>::get_instant()
    }
    fn read<'l>() -> GraphReadGuard<'l, Self>;
}
pub trait Multigraph {}
pub trait MultigraphStubProvider: Multigraph {
    fn read<'l, G: Graph + MultigraphMember<Multigraph = Self>>() -> GraphReadGuard<'l, G>;
}
pub trait MultigraphMember {
    type Multigraph: Multigraph;
}
pub trait Graph: Send + Sync + Sized + 'static {
    type From: Clone + Sync + Send + Hash + Eq;
    type To: Clone + Sync + Send + Hash + Eq;
    fn get_input_edge_set(to: &Self::To) -> Option<&EmbedGraph<Self, Self::From>>;
    fn get_output_edge_set(from: &Self::From) -> Option<&EmbedGraph<Self, Self::To>>;
}
impl<G: Graph> Absorb<GraphOperation<G>> for GraphVersion {
    fn absorb_first(&mut self, operation: &mut GraphOperation<G>, _other: &Self) {
        unsafe {
            match operation {
                GraphOperation::AddEdge { from, to } => {
                    G::get_output_edge_set(from).map(|edge_set| edge_set.get_mut(self).insert(to.clone()));
                    G::get_input_edge_set(to).map(|edge_set| edge_set.get_mut(self).insert(from.clone()));
                }
                GraphOperation::RemoveEdge { from, to } => {
                    G::get_output_edge_set(from).map(|edge_set| edge_set.get_mut(self).remove(to));
                    G::get_input_edge_set(to).map(|edge_set| edge_set.get_mut(self).remove(from));
                }
            }
        }
    }

    fn sync_with(&mut self, first: &Self) {
        self.0 = (2 + first.0) ^ 1;
    }

    fn absorb_second(&mut self, operation: GraphOperation<G>, _other: &Self) {
        match operation {
            GraphOperation::AddEdge { from, to } => {
                let from_node_edge_set = G::get_output_edge_set(&from);
                let to_node_edge_set = G::get_input_edge_set(&to);
                unsafe {
                    match (from_node_edge_set, to_node_edge_set) {
                        (None, None) => {}
                        (Some(from_node_edge_set), None) => {
                            from_node_edge_set.get_mut(self).insert(to);
                        }
                        (None, Some(to_node_edge_set)) => {
                            to_node_edge_set.get_mut(self).insert(from);
                        }
                        (Some(from_node_edge_set), Some(to_node_edge_set)) => {
                            from_node_edge_set.get_mut(self).insert(to.clone());
                            to_node_edge_set.get_mut(self).insert(from);
                        }
                    }
                }
            }
            GraphOperation::RemoveEdge { from, to } => unsafe {
                G::get_output_edge_set(&from).map(|edge_set| edge_set.get_mut(self).remove(&to));
                G::get_input_edge_set(&to).map(|edge_set| edge_set.get_mut(self).remove(&from));
            },
        }
    }

    fn drop_first(self: Box<Self>) {}

    fn drop_second(self: Box<Self>) {}
}
pub enum GraphOperation<G: Graph> {
    AddEdge { from: G::From, to: G::To },
    RemoveEdge { from: G::From, to: G::To },
    // RemoveNodeOutput(G::From),
    // RemoveNodeInput(G::To),
}

pub trait FromNode<G: Graph> {
    fn output(&self, guard: &GraphReadGuard<G>) -> &HashSet<G::To>;
}
impl<T, G: Graph<From = T>> FromNode<G> for T {
    fn output(&self, guard: &GraphReadGuard<'_, G>) -> &HashSet<<G as Graph>::To> {
        G::get_output_edge_set(self).unwrap().get(&*guard.0)
    }
}
pub trait ToNode<G: Graph> {
    fn input(&self, guard: &GraphReadGuard<G>) -> &HashSet<G::From>;
}
impl<T, G: Graph<To = T>> ToNode<G> for T {
    fn input(&self, guard: &GraphReadGuard<'_, G>) -> &HashSet<<G as Graph>::From> {
        G::get_input_edge_set(self).unwrap().get(&*guard.0)
    }
}
