#![feature(io_read_to_string)]
use std::{
    io::{stdin, Write},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use failure::Fallible;
use log::{error, info, trace};
use structopt::StructOpt;
use vm_lua::{LuaJIT, LUA_INTERPRETER};

use crate::cli::Opt;
mod cli;

fn main() -> Fallible<()> {
    env_logger::init();
    vm_lua::set_signal_handler();
    let opt = cli::Opt::from_args();
    let lua_state = vm_lua::new_state()?;
    info!("wangzi lua vm v1.0.0");
    let run = move |lua_state: vm_lua::mem::LuaStateReference, code: String, opt: &Opt| {
        let bench = opt.bench;
        match std::thread::spawn(move || {
            let resource = match vm_lua::load_code(lua_state.clone(), &code) {
                Ok(r) => r,
                Err(e) => {
                    error!("{}", e);
                    trace!("{:?}", e);
                    panic!();
                }
            };
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
            }
        };
    };
    for code in opt.command.iter().cloned() {
        run(lua_state.clone(), code, &opt);
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
#[cfg(test)]
mod tests {
    use failure::Fallible;
    use log::{debug, error, info, trace};
    use runtime::instructions::{Instruction, InstructionType};

    #[test]
    fn run_lua_script() -> Fallible<()> {
        let code = "local a, b, n = 1, 1, 1000000000 while a < n do
  a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b
  a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b
  a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b a = a + b
  a = a + b a = a + b a = a + b a = a + b end
";
        env_logger::init();
        vm_lua::set_signal_handler();
        let lua_state = vm_lua::new_state()?;
        match vm_lua::run_code(lua_state, &code) {
            Ok(_) => {}
            Err(e) => {
                error!("{}", e);
                trace!("{:?}", e);
                return Err(e);
            }
        };
        Ok(())
    }
}
