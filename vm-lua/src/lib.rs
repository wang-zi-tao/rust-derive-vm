//! Lua 5.4
//! https://www.lua.org/manual/5.4/
#![feature(iterator_try_collect)]
#![feature(concat_idents)]
#![feature(slice_ptr_get)]
#![feature(new_uninit)]
#![feature(more_qualified_paths)]
#![feature(hash_drain_filter)]
#![feature(hash_set_entry)]
#![feature(slice_ptr_len)]
#![feature(nonnull_slice_from_raw_parts)]
#![feature(const_convert)]
pub use core;
use log::debug;

pub use runtime;
pub use util;
use vm_core::DynRuntimeTrait;

use std::sync::Arc;
use std::{cell::UnsafeCell, collections::{HashMap, HashSet}, ptr::NonNull};

use failure::Fallible;

use lexical::Lexical;
use lua_lexical::LuaLexical;
use mem::*;

use runtime::code::FunctionPack;
use runtime_extra::{Bool, NullableOptionImpl, NullablePointerImpl, Usize, U64, U8};
use vm_core::{ObjectRef, Pointer, UnsizedArray};

pub use crate::ir::LuaInstructionSet;
#[macro_use]
extern crate lexical_derive;
extern crate lexical;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate runtime_derive;
#[macro_use]
extern crate derive_builder;
extern crate static_assertions;
pub(crate) type TypeResourceImpl = memory_mmmu::RegistedType;
pub mod builder;
pub mod built_in;
pub mod error;
pub mod instruction;
pub mod ir;
pub mod lua_lexical;
pub mod mem;
pub mod syntax;
pub fn add_global_function(state: LuaStateReference, key: &str, function: &LuaFunctionRustType) -> Fallible<()> {
    add_global(
        state.clone(),
        new_string(state.as_pointer(), key.as_bytes())?,
        new_function(state, function)?,
    )
}
pub fn new_string(state: Pointer<LuaState>, buffer: &[u8]) -> Fallible<LuaValueImpl> {
    unsafe {
        let mut state_ptr_clone = state.clone();
        let strings = state_ptr_clone.as_ref_mut().ref_strings_mut();
        let string = strings.get_or_insert_with(buffer, |buffer| {
            let string = LuaStringReference(
                LuaStringReference::get()
                    .unwrap()
                    .alloc_unsized(buffer.len())
                    .unwrap()
                    .cast(),
            );
            let mut string_ptr = string.as_pointer();
            let string_ref = string_ptr.as_ref_mut();
            string_ref.set_lua_state(state);
            string_ref.ref_data_mut().set_len(buffer.len());
            string_ref
                .ref_data_mut()
                .as_slice_mut()
                .iter_mut()
                .zip(buffer.iter())
                .for_each(|(s, b)| *s = U8(*b));
            LuaStringNativeReference(string)
        });
        Ok(LuaValueImpl::encode_string(string.0.as_pointer()))
    }
}
pub fn add_global(state: LuaStateReference, key: LuaValueImpl, value: LuaValueImpl) -> Fallible<()> {
    unsafe {
        let mut state_ptr = state.as_pointer();
        let state_ref = state_ptr.as_ref_mut();
        let mut global = state_ref.get_global();
        let global_ref = global.as_ref_mut();
        let mut shape = global_ref.get_shape();
        let shape_ref = shape.as_ref_mut();
        let key_map = shape_ref.ref_fields_mut().get_mut();
        let slot = key_map.len();
        let mut slot_impl = LuaSlotMetadataImpl(Default::default());
        slot_impl.set_slot(Usize(slot));
        key_map.insert(key, slot_impl);
        let fast_fields = global_ref.ref_fast_fields_mut().as_slice_mut();
        if fast_fields.len() > slot {
            fast_fields[slot] = value;
        } else {
            let index = slot - fast_fields.len();
            if let Some(slow_fields) = global_ref.get_slow_fields().read_some() {
                let mut slow_fields = Pointer::<UnsizedArray<LuaValue>>::new(slow_fields.cast());
                if slow_fields.as_ref_mut().len() <= index {
                    let slow_fields_slice = slow_fields.as_ref_mut().as_slice();
                    let len = 1 << (usize::BITS - index.leading_zeros());
                    let mut new_slow_fields = Pointer::<UnsizedArray<LuaValue>>::new(
                        LuaValueArrayReference::get()?.alloc_unsized(len)?.cast(),
                    );
                    let (copy_slice, fill_slice) = new_slow_fields.as_ref_mut().as_slice_mut().split_at_mut(index);
                    copy_slice.clone_from_slice(slow_fields_slice);
                    fill_slice.fill(LuaValueImpl::encode_nil(()));
                    global_ref.set_slow_fields(NullablePointerImpl::encode_some(new_slow_fields.as_non_null().cast()));
                }
                slow_fields.as_ref_mut().as_slice_mut()[index] = value;
            } else {
                let len = 7;
                let mut new_slow_fields =
                    Pointer::<UnsizedArray<LuaValue>>::new(LuaValueArrayReference::get()?.alloc_unsized(len)?.cast());
                let new_slow_fields_ptr = new_slow_fields.as_non_null();
                let new_slow_fields_slice = new_slow_fields.as_ref_mut().as_slice_mut();
                new_slow_fields_slice.fill(LuaValueImpl::encode_nil(()));
                global_ref.set_slow_fields(NullablePointerImpl::encode_some(new_slow_fields_ptr.cast()));
                new_slow_fields_slice[index] = value;
            }
        }
    }
    Ok(())
}
pub fn new_function(state: LuaStateReference, native_function: &LuaFunctionRustType) -> Fallible<LuaValueImpl> {
    unsafe {
        let function = LuaFunctionReference(LuaFunctionReference::get()?.alloc()?.cast());
        let mut function_ptr = function.as_pointer();
        let function_ref = function_ptr.as_ref_mut();
        function_ref.set_state(state.as_pointer());
        function_ref.set_function(NonNull::from(native_function).cast());
        let lua_value = LuaValueImpl::encode_function(function.as_pointer());
        Ok(lua_value)
    }
}
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
pub type LuaRuntime = Arc<dyn DynRuntimeTrait<FunctionPack<LuaInstructionSet>>>;
pub fn new_state(runtime: LuaRuntime) -> Fallible<LuaStateReference> {
    unsafe {
        let string_meta_functions = new_meta_functions()?;
        let state = LuaStateReference(LuaStateReference::get()?.alloc()?.cast());
        let mut state_ptr = state.as_pointer();
        let state_ref = state_ptr.as_ref_mut();
        state_ref.set_strings(HashSet::<LuaStringNativeReference>::new());
        state_ref.set_runtime(runtime);
        state_ref.set_string_meta_functions(string_meta_functions.as_pointer());
        state_ref.set_gc_mark(Bool(false));
        state_ref.set_table_shape(new_shape(new_meta_functions()?, false)?.as_pointer());
        let global_table = new_table(new_meta_functions()?, 64, true)?.as_pointer();
        state_ref.set_global(global_table);
        built_in::register_built_in_functions(state.clone())?;
        Ok(state)
    }
}
#[cfg(feature = "runtime")]
mod runtime_feature {
    use crate::LuaInstructionSet;
    use lazy_static::lazy_static;
    use memory_mmmu::MemoryMMMU;
    pub use runtime;
    use std::sync::Arc;
    pub use util;

    pub type LuaInterpreter = Interpreter<LuaInstructionSet, MemoryMMMU>;
    pub type LuaJIT = JITCompiler<LuaInstructionSet, MemoryMMMU>;
    use llvm_runtime::Interpreter;
    use llvm_runtime::JITCompiler;
    lazy_static! {
        pub static ref LUA_INTERPRETER: Arc<Interpreter<LuaInstructionSet, MemoryMMMU>> =
            Arc::new(match Interpreter::new() {
                Ok(o) => o,
                Err(e) => panic!("{}", e),
            });
    }
    lazy_static! {
        pub static ref LUA_JIT: Arc<JITCompiler<LuaInstructionSet, MemoryMMMU>> = Arc::new(match JITCompiler::new() {
            Ok(o) => o,
            Err(e) => panic!("{}", e),
        });
    }
}
#[cfg(feature = "runtime")]
pub use runtime_feature::*;
pub fn pack_code(lua_state: LuaStateReference, code: &str) -> Fallible<Vec<FunctionPack<LuaInstructionSet>>> {
    debug!(target:"vm_lua::pack_code","code: {:?}", code);
    let lexical = LuaLexical::parse(code)?;
    debug!(target:"vm_lua::pack_code","lexical: {:?}", lexical);
    let pack = crate::syntax::parse(lua_state, lexical)?;
    debug!(target:"vm_lua::pack_code","function pack: {:?}", pack);
    Ok(pack)
}
pub fn load_code(lua_state: LuaStateReference, code: &str) -> Fallible<ObjectRef> {
    let mut pack = pack_code(lua_state.clone(), code)?;
    let root_function = pack.pop().unwrap();
    let lua_state_pointer = lua_state.as_pointer();
    let runtime = unsafe { lua_state_pointer.as_ref().ref_runtime() };
    let resource = runtime.create_dyn(root_function)?;
    for closure in pack {
        runtime.create_dyn(closure)?;
    }
    let object = resource.get_object()?;
    Ok(object)
}
pub fn run_code(lua_state: LuaStateReference, code: &str) -> Fallible<()> {
    let resource = load_code(lua_state.clone(), code)?;
    unsafe {
        let function: LuaFunctionRustType = std::mem::transmute(resource.lock().unwrap().get_export_ptr(0));
        let args = &[];
        function(lua_state, args);
    }
    Ok(())
}
pub fn hello() {
    println!("[ zitao lua 虚拟机 v{} ]", &env!("CARGO_PKG_VERSION"));
}
