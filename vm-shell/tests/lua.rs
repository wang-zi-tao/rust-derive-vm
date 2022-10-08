use failure::Fallible;
use llvm_runtime::{Interpreter, JITCompiler};
use log::debug;
use memory_mmmu::MemoryMMMU;
use std::{
    io::{stderr, Write},
    path::PathBuf,
};

use vm_lua::util::set_signal_handler;

use vm_lua::ir::LuaInstructionSet;

pub type LuaInterpreter = Interpreter<LuaInstructionSet, MemoryMMMU>;
pub type LuaJIT = JITCompiler<LuaInstructionSet, MemoryMMMU>;
#[test]
fn run_lua_script() -> Fallible<()> {
    env_logger::init();
    set_signal_handler();
    let state = vm_lua::new_state()?;
    let code = "a=print+1";
    vm_lua::run_code(state, code, &LuaInterpreter::new()?)?;
    Ok(())
}
