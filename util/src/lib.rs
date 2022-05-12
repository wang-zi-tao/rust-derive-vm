#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
#[macro_use]
extern crate lazy_static;
mod as_any;
mod atomic_cell;
mod atomic_lazy_arc;
mod cache_aligned_vec;
mod const_map;
mod cow;
mod default_arc;
#[cfg(feature = "derive")]
mod derive;
mod graph;
mod hash;
mod ident;
mod lazy_dyn_ref;
mod linked_list;
mod pooled;
mod string_pool;
mod token_rwlock;

#[cfg(feature = "derive")]
pub use crate::derive::*;
pub use crate::{
    as_any::*, atomic_cell::*, atomic_lazy_arc::*, cache_aligned_vec::*, const_map::*, cow::*, default_arc::*, graph::*, hash::*, ident::*, lazy_dyn_ref::*,
    linked_list::*, pooled::*, string_pool::*, token_rwlock::*,
};
pub use util_derive::inline_const;
