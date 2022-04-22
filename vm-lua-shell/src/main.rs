use std::io::{stdin, Write};

use failure::Fallible;
use log::{error, info, trace};
use vm_lua::{
    mem::{LuaStateReference, LuaValueImpl},
    LUA_INTERPRETER,
};

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
        match vm_lua::run_code(lua_state.clone(), &code) {
            Ok(_) => {}
            Err(e) => {
                error!("{}", e);
                trace!("{:?}", e);
            }
        };
    }
    Ok(())
}
