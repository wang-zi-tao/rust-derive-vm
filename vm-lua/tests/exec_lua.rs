use failure::Fallible;
use llvm_runtime::Interpreter;
use llvm_runtime::JITCompiler;
use log::debug;
use memory_mmmu::MemoryMMMU;
use scan_dir::ScanDir;

use std::path::PathBuf;
use std::sync::Arc;

use util::set_signal_handler;

use vm_lua::ir::LuaInstructionSet;

pub type LuaInterpreter = Interpreter<LuaInstructionSet, MemoryMMMU>;
pub type LuaJIT = JITCompiler<LuaInstructionSet, MemoryMMMU>;
#[test]
fn run_lua_script() -> Fallible<()> {
    env_logger::init();
    set_signal_handler();
    let state = vm_lua::new_state(Arc::new(LuaInterpreter::new()?))?;
    let code = "a=1+1 print(a)";
    if let Err(e) = vm_lua::run_code(state, code) {
        println!("{:?}", &e);
        return Err(e);
    };
    Ok(())
}
// #[test]
fn run_scipts_in_tests_dir() -> Fallible<()> {
    env_logger::init();
    set_signal_handler();
    let mut index = 0;
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push("tests");
    let runtime = Arc::new(LuaJIT::new()?);
    ScanDir::files().read(d, |iter| {
        for (entry, name) in iter {
            match (|| {
                if name.ends_with(".lua") {
                    debug!(target : "test_scripts", "loading:{}, index:{}\n", &name, &index);
                    let code = std::fs::read_to_string(entry.path())?;
                    let state = vm_lua::new_state(runtime.clone())?;
                    vm_lua::run_code(state, &code)?;
                    debug!(target : "test_scripts", "finish:{}\n", &name);
                }
                Fallible::Ok(())
            })() {
                Ok(o) => o,
                Err(e) => panic!("{}\n{:?}", &e, e),
            };
            index += 1;
        }
    })?;
    Ok(())
}
