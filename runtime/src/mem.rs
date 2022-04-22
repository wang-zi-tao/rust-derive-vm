use failure::Fallible;
use util::CowArc;

use crate::instructions::MemoryInstructionSet;

pub trait MemoryInstructionSetProvider {
    fn get_memory_instruction_set() -> Fallible<CowArc<'static, MemoryInstructionSet>>;
}
