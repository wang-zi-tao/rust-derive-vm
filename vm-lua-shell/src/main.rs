#![feature(io_read_to_string)]
use std::io::{stdin, Write};

use failure::Fallible;
use log::{error, info, trace};
use structopt::StructOpt;
use vm_lua::LUA_INTERPRETER;
mod cli;

fn main() -> Fallible<()> {
    env_logger::init();
    vm_lua::set_signal_handler();
    let opt = cli::Opt::from_args();
    let lua_state = vm_lua::new_state()?;
    let _ = &*LUA_INTERPRETER;
    info!("wangzi lua vm v1.0.0");
    for code in opt.command.iter().cloned() {
        let _ = vm_lua::spawn(lua_state.clone(), code).join();
    }
    for file in opt.file.iter() {
        let code = std::fs::read(file)?;
        let _ = vm_lua::spawn(lua_state.clone(), String::from_utf8_lossy(&code).to_string()).join();
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
            let _ = vm_lua::spawn(lua_state.clone(), code).join();
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
        let code = "local a=1 while a<16 do a=a+1 end";
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
