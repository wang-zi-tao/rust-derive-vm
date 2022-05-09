use std::cell::UnsafeCell;
use std::ptr::NonNull;

use failure::Fallible;
use vm_core::{Direct, Pointer, UnsizedArray};

use crate::mem::{LuaFunctionRustType, LuaValue, LuaValueImpl};
use crate::{instruction::extend_to_buffer, mem::LuaStateReference};
static EMPTY_RETURN_INNER: UnsizedArray<LuaValue> = UnsizedArray::empty();
pub fn empty_return() -> Pointer<UnsizedArray<LuaValue>> { Pointer::new(NonNull::from(&EMPTY_RETURN_INNER)) }

pub extern "C" fn print(state: LuaStateReference, args: &[LuaValueImpl]) -> Pointer<UnsizedArray<LuaValue>> {
    let mut buffer = Vec::new();
    for (arg_index, arg) in args.iter().enumerate() {
        if arg_index != 0 {
            buffer.push(b'\t');
        }
        unsafe {
            extend_to_buffer(&mut buffer, Direct(arg.clone()));
        }
    }
    println!("{}", String::from_utf8_lossy(&buffer));
    empty_return()
}
pub const DEFAULT_BUILT_IN_FUNCTIONS: &'static [(&'static str, &'static LuaFunctionRustType)] =
    &[("print", &(print as LuaFunctionRustType))];
pub fn register_built_in_functions(state: LuaStateReference) -> Fallible<()> {
    for (name, function) in DEFAULT_BUILT_IN_FUNCTIONS {
        crate::add_global_function(state.clone(), name, function)?;
    }
    Ok(())
}
