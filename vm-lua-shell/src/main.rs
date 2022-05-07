use std::io::{stdin, Write};

use failure::Fallible;
use log::{error, info, trace};
use vm_lua::LUA_INTERPRETER;

fn main() -> Fallible<()> {
    env_logger::init();
    let lua_state = vm_lua::new_state()?;
    let _ = &*LUA_INTERPRETER;
    info!("wangzi lua vm v1.0.0");
    loop {
        print!(">>> ");
        std::io::stdout().flush().unwrap();
        let mut code = String::new();
        let len = stdin().read_line(&mut code)?;
        if len == 0 {
            break;
        }
        let lua_state = lua_state.clone();
        match std::thread::spawn(move || {
            let code = code;
            match vm_lua::run_code(lua_state, &code) {
                Ok(_) => {}
                Err(e) => {
                    error!("{}", e);
                    trace!("{:?}", e);
                }
            };
        })
        .join()
        {
            Ok(_) => {}
            Err(_) => {
                error!("exec thread panic!");
            }
        }
    }
    Ok(())
}
