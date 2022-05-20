use std::io::{stdin, Write};

use failure::Fallible;
use log::error;
use vm_lua::LUA_INTERPRETER;

fn main() -> Fallible<()> {
    env_logger::init();
    vm_lua::set_signal_handler();
    let _ = &*LUA_INTERPRETER;
    let vm = vm_lua::new_state()?;
    println!("[ zitao lua 虚拟机 v{} ]", &env!("CARGO_PKG_VERSION"));
    loop {
        print!("");
        std::io::stdout().flush().unwrap();
        let mut code = String::new();
        let len = stdin().read_line(&mut code)?;
        if len == 0 || &code == "\n" {
            break;
        }
        if let Err(e) = vm_lua::run_code(vm.clone(), &code) {
            error!("{e}")
        };
    }
    Ok(())
}
