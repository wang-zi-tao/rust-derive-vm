pub use ghost_cell as _ghost_cell;

use std::{any::Any, fmt::Debug};
extern crate util_derive;
#[macro_use]
extern crate derive_builder;
use util::AsAny;
mod aot;
mod code;
mod context;
mod memory;
mod resources;
mod runtime;
mod singleton;
mod task;
pub use crate::{
    aot::*,
    context::*,
    memory::{buffer::*, object::*, ty::*, *},
    resources::*,
    runtime::*,
    singleton::*,
    task::*,
};

pub trait Module: Sync + Send + Debug {}
pub trait Component: Any + AsAny + Sync + Send + Debug {}

pub trait JVM: Sync + Send + Debug {}
