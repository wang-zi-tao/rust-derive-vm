#![feature(slice_ptr_get)]
#![feature(slice_ptr_len)]
#![feature(nonnull_slice_from_raw_parts)]
extern crate failure_derive;
#[macro_use]
extern crate lazy_static;
mod linux;
#[cfg(target_os = "linux")]
pub use crate::linux::*;
