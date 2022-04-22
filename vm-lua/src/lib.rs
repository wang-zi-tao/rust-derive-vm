//! Lua 5.4
//! https://www.lua.org/manual/5.4/
#![feature(iterator_try_collect)]
#![feature(concat_idents)]
#![feature(slice_ptr_get)]
#![feature(new_uninit)]
#![feature(more_qualified_paths)]

use log::debug;

use std::{cell::UnsafeCell, collections::HashMap};

use failure::Fallible;
use interpreter::Interpreter;
use jvm_core::{ExecutableResourceTrait, ObjectRef, ResourceFactory};
use lazy_static::lazy_static;
use lexical::Lexical;
use lua_lexical::LuaLexical;
use mem::*;
use memory_mmmu::MemoryMMMU;
use runtime::code::FunctionPack;
use runtime_extra::{Bool, NullableOptionImpl, NullablePointerImpl, U64};

use crate::ir::LuaInstructionSet;
#[macro_use]
extern crate lexical_derive;
#[macro_use]
extern crate lexical;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate runtime_derive;
#[macro_use]
extern crate derive_builder;
#[macro_use]
extern crate static_assertions;
pub(crate) type TypeResourceImpl = memory_mmmu::RegistedType;
mod lua_lexical;
struct LuaModule {}
struct LuaVM {}
pub mod builder;
pub mod error;
pub mod instruction;
pub mod ir;
pub mod mem;
// pub mod shell;
pub mod syntax;
// pub mod syntax {
//     use crate::{ir::LuaInstructionSet, lua_lexical::LuaLexical};
//     use failure::Fallible;
//     use runtime::code::FunctionPack;
//     pub fn parse(_source: Vec<LuaLexical>) -> Fallible<FunctionPack<LuaInstructionSet>> { todo!() }
// }
pub fn new_shape(metas: LuaMetaFunctionsReference, is_owned: bool) -> Fallible<LuaShapeReference> {
    unsafe {
        let shape = LuaShapeReference(LuaShapeReference::get()?.alloc()?.cast());
        let mut shape_ptr = shape.as_pointer();
        let shape_ref = shape_ptr.as_ref_mut();
        shape_ref.set_fields(UnsafeCell::new(HashMap::new()));
        shape_ref.set_meta_functions(metas.as_pointer());
        shape_ref.set_as_meta_table(NullableOptionImpl::encode_none(()));
        shape_ref.set_max_int_index(U64(0));
        shape_ref.set_is_owned(Bool(is_owned));
        shape_ref.set_action_of_field(UnsafeCell::new(HashMap::new()));
        shape_ref.set_action_of_metatable(UnsafeCell::new(HashMap::new()));
        let invalid = BoolReference(BoolReference::get()?.alloc()?.cast());
        *invalid.as_pointer().as_ref_mut() = Bool(false);
        shape_ref.set_invalid(invalid.as_pointer());
        Ok(shape)
    }
}
pub fn new_table(metas: LuaMetaFunctionsReference, cap: usize, use_owned_shape: bool) -> Fallible<LuaTableReference> {
    unsafe {
        let table = LuaTableReference(LuaTableReference::get()?.alloc_unsized(cap)?.cast());
        let mut table_ptr = table.as_pointer();
        let table_ref = table_ptr.as_ref_mut();
        table_ref.set_shape(new_shape(metas, use_owned_shape)?.as_pointer());
        table_ref.set_slow_fields(NullablePointerImpl::encode_none(()));
        table_ref.ref_fast_fields_mut().0 = cap;
        for i in 0..cap {
            table_ref.ref_fast_fields_mut().as_slice_mut()[i] = LuaValueImpl::encode_nil(());
        }
        Ok(table)
    }
}
pub fn new_meta_functions() -> Fallible<LuaMetaFunctionsReference> {
    unsafe {
        let meta_functions = LuaMetaFunctionsReference(LuaMetaFunctionsReference::get()?.alloc()?.cast());
        let mut meta_functions_ptr = meta_functions.as_pointer();
        let meta_functions_ref = meta_functions_ptr.as_ref_mut();
        meta_functions_ref.set_valid(Bool(true));
        meta_functions_ref.set_meta_table(new_table(meta_functions.clone(), 0, false)?.as_pointer());
        meta_functions_ref.set_parent(NullableOptionImpl::encode_none(()));
        meta_functions_ref.set_sub_metatable(Vec::new());
        meta_functions_ref.set_add(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_sub(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_mul(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_div(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_mod_(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_pow(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_unm(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_idiv(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_band(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_bor(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_bxor(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_bnot(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_shl(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_shr(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_concat(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_len(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_eq(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_lt(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_le(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_index(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_newindex(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_call(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_metadata(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_gc(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_mode(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_name(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_tostring(LuaValueImpl::encode_nil(()));
        meta_functions_ref.set_pairs(LuaValueImpl::encode_nil(()));
        Ok(meta_functions)
    }
}
pub fn new_state() -> Fallible<LuaStateReference> {
    unsafe {
        let string_meta_functions = new_meta_functions()?;
        let state = LuaStateReference(LuaStateReference::get()?.alloc()?.cast());
        let mut state_ptr = state.as_pointer();
        let state_ref = state_ptr.as_ref_mut();
        state_ref.set_string_meta_functions(string_meta_functions.as_pointer());
        state_ref.set_gc_mark(Bool(false));
        state_ref.set_table_shape(new_shape(new_meta_functions()?, false)?.as_pointer());
        state_ref.set_global(new_table(new_meta_functions()?, 64, true)?.as_pointer());
        Ok(state)
    }
}
lazy_static! {
    pub static ref LUA_INTERPRETER: Interpreter<LuaInstructionSet, MemoryMMMU> = {
        match Interpreter::new() {
            Ok(o) => o,
            Err(e) => panic!("{}", e),
        }
    };
}
pub fn pack_code(code: &str) -> Fallible<FunctionPack<LuaInstructionSet>> {
    debug!(target:"vm_lua::pack_code","code: {:?}", code);
    let lexical = LuaLexical::parse(code)?;
    debug!(target:"vm_lua::pack_code","lexical: {:?}", lexical);
    let pack = crate::syntax::parse(lexical)?;
    debug!(target:"vm_lua::pack_code","function pack: {:?}", pack);
    Ok(pack)
}
pub fn load_code(code: &str) -> Fallible<ObjectRef> {
    let pack = pack_code(code)?;
    let resource = LUA_INTERPRETER.create(pack)?;
    ExecutableResourceTrait::<FunctionPack<LuaInstructionSet>>::get_object(&*resource)
}
pub fn run_code(lua_state: LuaStateReference, code: &str) -> Fallible<()> {
    let pack = pack_code(code)?;
    let resource = LUA_INTERPRETER.create(pack)?;
    unsafe {
        let function: *const LuaFunctionRustType = resource.get_address();
        let args = &[];
        (*function)(lua_state, args);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use failure::Fallible;
    use interpreter::Interpreter;
    use log::debug;
    use memory_mmmu::MemoryMMMU;
    use scan_dir::ScanDir;
    use std::io::{stderr, Write};
    use std::path::PathBuf;
    // #[test]
    fn check_ir() -> Fallible<()> {
        let _ = crate::new_state()?;
        match Interpreter::<crate::ir::LuaInstructionSet, MemoryMMMU>::new() {
            Ok(o) => o,
            Err(e) => panic!("{}", e),
        };
        Ok(())
    }
    #[test]
    fn run_scipts_in_tests_dir() -> Fallible<()> {
        env_logger::init();
        set_signal_handler();
        let mut index = 0;
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("tests");
        ScanDir::files().read(d, |iter| {
            for (entry, name) in iter {
                match (|| {
                    if name.ends_with(".lua") {
                        debug!("loading:{}, index:{}\n", &name, &index);
                        let code = std::fs::read_to_string(entry.path())?;
                        let state = crate::new_state()?;
                        let pack = crate::pack_code(&*code)?;
                        debug!("packed:{:?}\n", &pack);
                        crate::run_code(state, &*code)?;
                        debug!("finish:{}\n", &name);
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
    fn set_signal_handler() {
        use nix::sys::signal;
        extern "C" fn handle_sigsegv(_: i32) {
            panic!("signal::SIGSEGV");
        }
        extern "C" fn handle_sig(s: i32) {
            panic!("signal {}", s);
        }
        unsafe {
            signal::sigaction(
                signal::SIGILL,
                &signal::SigAction::new(
                    signal::SigHandler::Handler(handle_sig),
                    signal::SaFlags::SA_NODEFER,
                    signal::SigSet::all(),
                ),
            )
            .unwrap();
            signal::sigaction(
                signal::SIGSEGV,
                &signal::SigAction::new(
                    signal::SigHandler::Handler(handle_sigsegv),
                    signal::SaFlags::SA_NODEFER,
                    signal::SigSet::empty(),
                ),
            )
            .unwrap();
        }
    }
}
