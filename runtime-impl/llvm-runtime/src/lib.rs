#![feature(ptr_metadata)]
#![feature(iterator_try_collect)]
mod genarator;
mod interpreter;
mod jit;
mod raw_llvm;

pub use interpreter::*;
pub use jit::*;
pub use raw_llvm::*;
