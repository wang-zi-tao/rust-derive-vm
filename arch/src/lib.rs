#![feature(exclusive_range_pattern)]

mod x86_64;
#[cfg(target_arch = "x86_64")]
pub use crate::x86_64::*;
/// CPU架构
pub trait Architecture {}
/// 基于寄存器的CPU架构
pub trait RegisterBaseArch: Architecture {
    type GenericRegister: GenericRegister;
}
/// 通用寄存器
pub trait GenericRegister {}
