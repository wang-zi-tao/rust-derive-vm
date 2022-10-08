use std::{
    io::{stdin, Write},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
extern crate vm_wenyan;

use failure::{format_err, Fallible};
use llvm_runtime::{Interpreter, JITCompiler};
use log::{error, info, trace};
use memory_mmmu::MemoryMMMU;
use structopt::StructOpt;
use vm_lua::LuaInstructionSet;

use crate::cli::Opt;
mod cli;
use lazy_static::lazy_static;

pub type LuaInterpreter = Interpreter<LuaInstructionSet, MemoryMMMU>;
pub type LuaJIT = JITCompiler<LuaInstructionSet, MemoryMMMU>;
lazy_static! {
    pub static ref LUA_INTERPRETER: Interpreter<LuaInstructionSet, MemoryMMMU> = Interpreter::new().unwrap();
}
lazy_static! {
    pub static ref LUA_JIT: JITCompiler<LuaInstructionSet, MemoryMMMU> = { JITCompiler::new().unwrap() };
}

fn main() -> Fallible<()> {
    env_logger::init();
    let opt = cli::Opt::from_args();
    let lua_state = vm_lua::new_state()?;
    vm_wenyan::加入虚拟机(lua_state.clone())?;
    vm_lua::hello();
    vm_wenyan::打招呼();
    println!("<<< zitao [lua,wenyan] 多语言虚拟机 v{} >>>", &env!("CARGO_PKG_VERSION"));
    let run = move |lua_state: vm_lua::mem::LuaStateReference, code: String, opt: &Opt| -> Fallible<()> {
        let run = || {
            let bench = opt.bench;
            let resource = match &*opt.language {
                "lua" => {
                    if opt.jit {
                        vm_lua::load_code(lua_state.clone(), &code, &*LUA_JIT)?
                    } else {
                        vm_lua::load_code(lua_state.clone(), &code, &*LUA_INTERPRETER)?
                    }
                }
                "wenyan" => {
                    if opt.jit {
                        vm_wenyan::加载代码(lua_state.clone(), &code, &*LUA_JIT)?
                    } else {
                        vm_wenyan::加载代码(lua_state.clone(), &code, &*LUA_INTERPRETER)?
                    }
                }
                o => {
                    panic!("unsupport language {}", o);
                }
            };
            match std::thread::spawn(move || {
                let start = SystemTime::now();
                unsafe {
                    let function: vm_lua::mem::LuaFunctionRustType = std::mem::transmute(resource.lock().unwrap().get_export_ptr(0));
                    let args = &[];
                    function(lua_state, args);
                }
                if bench {
                    let end = SystemTime::now();
                    let difference = end.duration_since(start).expect("Clock may have gone backwards");
                    println!("bench: {difference:?}");
                }
            })
            .join()
            {
                Ok(_) => {}
                Err(e) => {
                    error!("exec thread panic");
                    return Err(format_err!("exec thread panic"));
                }
            };
            Fallible::Ok(())
        };
        match run() {
            Ok(r) => Ok(r),
            Err(e) => {
                error!("{}", e);
                trace!("{:?}", e);
                Err(e)
            }
        }
    };
    for code in opt.command.iter().cloned() {
        let r = run(lua_state.clone(), code, &opt);
        if let Err(_) = &r {
            return r;
        }
    }
    for file in opt.file.iter() {
        let code = std::fs::read(file)?;
        run(lua_state.clone(), String::from_utf8_lossy(&code).to_string(), &opt);
    }
    if opt.file.is_empty() && opt.command.is_empty() {
        loop {
            print!(">>> ");
            std::io::stdout().flush().unwrap();
            let mut code = String::new();
            let len = stdin().read_line(&mut code)?;
            if len == 0 {
                break;
            }
            run(lua_state.clone(), code, &opt);
        }
    }
    Ok(())
}
