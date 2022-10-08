use std::io::{stdin, Write};

use failure::Fallible;
use log::error;
use vm_lua::LUA_INTERPRETER;

fn main() -> Fallible<()> {
    env_logger::init();
    vm_lua::util::set_signal_handler();
    let _ = &*LUA_INTERPRETER;
    let vm = vm_wenyan::创建虚拟机()?;
    vm_wenyan::打招呼();
    loop {
        print!("");
        std::io::stdout().flush().unwrap();
        let mut code = String::new();
        let len = stdin().read_line(&mut code)?;
        if len == 0 || &code == "\n" {
            break;
        }
        if let Err(e) = vm_wenyan::运行代码(vm.clone(), &code, &*LUA_INTERPRETER) {
            error!("{e}")
        };
    }
    Ok(())
}
