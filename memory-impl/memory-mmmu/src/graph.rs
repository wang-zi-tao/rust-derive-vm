use std::hash::Hash;

use failure::Fallible;
use vm_core::{Tuple, Type};

use util::{CacheAlignedVec, CowArc, EmbedGraph, Graph, GraphLocalHandle};

use crate::{
    plan::{EmbedStatistice, ReferenceStatistice},
    RegistedType,
};
pub struct AssignGraph {}
impl Graph for AssignGraph {
    type From = TypeAssignEdge;
    type To = CowArc<'static, RegistedType>;

    fn get_input_edge_set(to: &Self::To) -> Option<&EmbedGraph<Self, Self::From>> {
        Some(&to.assign_from)
    }

    fn get_output_edge_set(_from: &Self::From) -> Option<&EmbedGraph<Self, Self::To>> {
        None
    }
}
pub struct ReferenceGraph {}
impl Graph for ReferenceGraph {
    type From = TypeReferenceEdge;
    type To = CowArc<'static, RegistedType>;

    fn get_input_edge_set(to: &Self::To) -> Option<&EmbedGraph<Self, Self::From>> {
        Some(&to.reference_from)
    }

    fn get_output_edge_set(_from: &Self::From) -> Option<&EmbedGraph<Self, Self::To>> {
        None
    }
}
pub(crate) struct TypeGraphHandle {
    pub(crate) references_graph: GraphLocalHandle<ReferenceGraph>,
    pub(crate) assign_graph: GraphLocalHandle<AssignGraph>,
}

impl TypeGraphHandle {
    fn new() -> Self {
        Self { references_graph: GraphLocalHandle::new(), assign_graph: GraphLocalHandle::new() }
    }
}
thread_local! {
  pub(crate) static GRAPH_HANDLE:TypeGraphHandle=TypeGraphHandle::new();
}

#[derive(Clone)]
pub struct TypeReferenceEdge {
    pub(crate) reference: CowArc<'static, RegistedType>,
    pub(crate) edge_statistice: ReferenceStatistice,
}

impl Eq for TypeReferenceEdge {}

impl PartialEq for TypeReferenceEdge {
    fn eq(&self, other: &Self) -> bool {
        &self.reference == &other.reference
    }
}

impl Hash for TypeReferenceEdge {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.reference.hash(state)
    }
}

impl std::ops::Deref for TypeReferenceEdge {
    type Target = CowArc<'static, RegistedType>;

    fn deref(&self) -> &Self::Target {
        &self.reference
    }
}
#[derive(Clone)]
pub struct TypeAssignEdge {
    pub(crate) assign: CowArc<'static, RegistedType>,
    pub(crate) edge_statistice: EmbedStatistice,
}

impl Eq for TypeAssignEdge {}

impl PartialEq for TypeAssignEdge {
    fn eq(&self, other: &Self) -> bool {
        &self.assign == &other.assign
    }
}

impl Hash for TypeAssignEdge {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.assign.hash(state)
    }
}

impl std::ops::Deref for TypeAssignEdge {
    type Target = CowArc<'static, RegistedType>;

    fn deref(&self) -> &Self::Target {
        &self.assign
    }
}
pub(crate) fn scan_reference(ty: &Type, mut callback: impl FnMut(&RegistedType) -> Fallible<()>) -> Fallible<()> {
    let mut work_stack = CacheAlignedVec::<&Type>::new();
    work_stack.push(ty);
    while let Some(last) = work_stack.pop() {
        match last {
            Type::Tuple(Tuple::Normal(fields)) => {
                work_stack.extend(fields.iter());
            }
            Type::Tuple(Tuple::Compose(fields)) => {
                work_stack.extend(fields.iter().map(|(ty, _small_layout)| ty));
            }
            Type::Enum(t) => {
                work_stack.extend(t.variants.iter());
            }
            Type::Union(t) => {
                work_stack.extend(t.iter());
            }
            Type::Array(layout, _) => {
                work_stack.push(&*layout);
            }
            Type::Reference(r) => {
                r.try_map(|resource| callback(RegistedType::try_downcast(&**resource)?))?;
            }
            _ => {}
        }
    }
    Ok(())
}
pub(crate) fn scan_assign(ty: &Type, mut callback: impl FnMut(&RegistedType) -> Fallible<()>) -> Fallible<()> {
    let mut work_stack = CacheAlignedVec::<&Type>::new();
    work_stack.push(ty);
    while let Some(last) = work_stack.pop() {
        match last {
            Type::Tuple(Tuple::Normal(fields)) => {
                work_stack.extend(fields.iter());
            }
            Type::Tuple(Tuple::Compose(fields)) => {
                work_stack.extend(fields.iter().map(|(ty, _small_layout)| ty));
            }
            Type::Enum(t) => {
                work_stack.extend(t.variants.iter());
            }
            Type::Union(t) => {
                work_stack.extend(t.iter());
            }
            Type::Array(layout, _) => {
                work_stack.push(&*layout);
            }
            Type::Embed(r) => {
                r.try_map(|resource| callback(RegistedType::try_downcast(&**resource)?))?;
            }
            _ => {}
        }
    }
    Ok(())
}
