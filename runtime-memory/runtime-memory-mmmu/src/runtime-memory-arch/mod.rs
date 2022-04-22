#![feature(min_const_generics)]





pub(crate) mod template_interpreter_x86_64;

#[cfg(target_arch = "x86_64")]
pub mod template_interpreter {

    pub use super::template_interpreter_x86_64::*;
}
