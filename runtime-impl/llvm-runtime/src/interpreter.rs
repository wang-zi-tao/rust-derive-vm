use std::{convert::TryInto, mem::MaybeUninit, ptr::null, sync::Arc};

use failure::Fallible;
use getset::{CopyGetters, Getters};
use inkwell::{
    context::Context,
    module::Module,
    values::{CallableValue, PointerValue},
};
use libffi::middle::{Callback, Cif, Closure, Type};
use util::CowArc;
use vm_core::{
    FunctionType, IntKind, ObjectBuilder, ObjectBuilderImport, ObjectBuilderInner, ObjectRef, RelocationKind, SymbolBuilder, Tuple, _ghost_cell::GhostToken,
};

use std::{any::Any, cell::RefCell, fmt::Debug, marker::PhantomData, rc::Rc, sync::RwLock};

use crate::genarator::{GlobalBuilder, LLVMFunctionBuilder};
use failure::format_err;

use runtime::{
    code::FunctionPack,
    instructions::{InstructionSet, InstructionType, MemoryInstructionSet},
    mem::MemoryInstructionSetProvider,
};
use util::AsAny;
use vm_core::{Component, ExecutableResourceTrait, Resource, ResourceError, ResourceFactory, RuntimeTrait};

#[derive(Getters)]
#[getset(get = "pub")]
pub struct RawInterpreter {
    binder: FunctionBinder,
    context: Arc<Context>,
}
impl RawInterpreter {
    pub fn new(
        instructions: &[(usize, InstructionType)],
        instruction_count: usize,
        memory_instruction_set: &MemoryInstructionSet,
        name: &str,
    ) -> Fallible<Self> {
        let context = Arc::new(Context::create());
        let binder = {
            let context_ref = &*context;
            let module = Arc::new(context_ref.create_module(name));
            let global_builder = Rc::new(RefCell::new(GlobalBuilder {
                symbol_maps: Default::default(),
                module,
                context: context_ref,
                memory_instruction_set: memory_instruction_set.clone(),
            }));
            let instruction_functions =
                LLVMFunctionBuilder::generate_instruction_set_interpreter(instructions, instruction_count, context_ref, global_builder.clone(), name)?;
            let GlobalBuilder { symbol_maps, module, .. } = Rc::try_unwrap(global_builder).unwrap().into_inner();
            FunctionBinder::generate(context_ref, &module, instruction_functions.as_pointer_value(), 12)?;
            module.verify().map_err(|e| format_err!("llvm verify error: {}", e.to_string()))?;
            let execution_engine = module.create_jit_execution_engine(inkwell::OptimizationLevel::Aggressive).map_err(|e| format_err!("llvm error: {}", e))?;
            for (symbol, ptr) in symbol_maps {
                execution_engine.add_global_mapping(&symbol, ptr as usize);
            }
            std::mem::forget(execution_engine.clone());
            FunctionBinder::from_jit(execution_engine, 12)?
        };
        Ok(Self { binder, context })
    }
}
pub fn debug_function(module: &Module, function_name: &str) {
    if let Some(i) = module.get_function(function_name) {
        i.print_to_stderr()
    }
}
#[derive(Getters)]
#[getset(get = "pub")]
pub struct Interpreter<S: InstructionSet, M: MemoryInstructionSetProvider> {
    raw: RawInterpreter,
    _ph: PhantomData<(S, M)>,
}
impl<S: InstructionSet, M: MemoryInstructionSetProvider> Interpreter<S, M> {
    pub fn new() -> Fallible<Self> {
        let raw = RawInterpreter::new(&S::INSTRUCTIONS, S::INSTRUCTION_COUNT, &*M::get_memory_instruction_set()?, stringify!(M))?;
        Ok(Self { raw, _ph: PhantomData })
    }
}
unsafe impl<S: InstructionSet, M: MemoryInstructionSetProvider> Send for Interpreter<S, M> {}
unsafe impl<S: InstructionSet, M: MemoryInstructionSetProvider> Sync for Interpreter<S, M> {}
impl<S: InstructionSet, M: MemoryInstructionSetProvider> Interpreter<S, M> {}
impl<S: InstructionSet, M: MemoryInstructionSetProvider> Debug for Interpreter<S, M> {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
impl<S: InstructionSet + 'static, M: MemoryInstructionSetProvider + 'static> AsAny for Interpreter<S, M> {
    fn as_any(&self) -> &(dyn Any) {
        self
    }

    fn as_any_arc(self: Arc<Self>) -> Arc<dyn Any + Sync + Send + 'static> {
        self
    }
}
impl<S: InstructionSet, M: MemoryInstructionSetProvider> Interpreter<S, M> {}
impl<S: InstructionSet, M: MemoryInstructionSetProvider> vm_core::Module for Interpreter<S, M> {}
impl<S: InstructionSet, M: MemoryInstructionSetProvider> ResourceFactory<FunctionPack<S>> for Interpreter<S, M> {
    type ResourceImpl = InterpreterFunction;

    fn define(&self) -> Fallible<Arc<Self::ResourceImpl>> {
        Ok(Arc::new(Default::default()))
    }

    fn upload(&self, _resource: &Self::ResourceImpl, _input: FunctionPack<S>) -> Fallible<()> {
        Err(ResourceError::Unsupported.into())
    }

    fn create(&self, input: FunctionPack<S>) -> Fallible<Arc<Self::ResourceImpl>> {
        let this = self.define()?;
        let ir = input.byte_code.clone();
        let bind = self
            .raw
            .binder
            .bind(
                ir.clone(),
                &input.function_type,
                input.register_count,
                input
                    .output
                    .or_else(|| {
                        let inner = this.inner.read().unwrap();
                        inner.as_ref().map(|inner| inner.bind.clone())
                    })
                    .unwrap_or_default(),
                self.raw.context.clone(),
            )
            .unwrap();
        *this.inner.write().unwrap() = Some(InterpreterFunctionInner { ir, bind });
        Ok(this)
    }
}
impl<S: InstructionSet, M: MemoryInstructionSetProvider> RuntimeTrait<FunctionPack<S>> for Interpreter<S, M> {}
#[derive(Getters, Default)]
#[getset(get = "pub")]
pub struct InterpreterFunctionInner {
    ir: ObjectRef,
    bind: ObjectRef,
}
#[derive(Getters, Default)]
#[getset(get = "pub")]
pub struct InterpreterFunction {
    inner: RwLock<Option<InterpreterFunctionInner>>,
}
impl<S> ExecutableResourceTrait<FunctionPack<S>> for InterpreterFunction {
    fn get_object(&self) -> Fallible<ObjectRef> {
        let inner = self.inner.read().unwrap();
        Ok(inner.as_ref().ok_or(ResourceError::NotLoaded)?.bind.clone())
    }
}
unsafe impl Send for InterpreterFunction {}
unsafe impl Sync for InterpreterFunction {}
impl AsAny for InterpreterFunction {
    fn as_any(&self) -> &(dyn Any) {
        self
    }

    fn as_any_arc(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }
}
impl Debug for InterpreterFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.read().unwrap();
        f.debug_struct("FunctionBind").field("ir", &inner.as_ref().map(|inner| inner.bind.clone())).finish()
    }
}

impl Component for InterpreterFunction {}
impl<M> Resource<FunctionPack<M>> for InterpreterFunction {
    fn get_state(&self) -> vm_core::ResourceState {
        vm_core::ResourceState::Ready
    }
}

#[repr(C)]
pub struct FunctionMetadata {
    register_count: u16,
    code: *const u8,
    args_count: usize,
    bind: unsafe extern "C" fn(),
    context: Arc<Context>,
    closure: Closure<'static>,
}
fn get_callback<'ctx>(
    context: &'ctx Context,
    instructions: PointerValue<'ctx>,
    optional_arg_count: Option<usize>,
    is_var_args: bool,
    module: &Module<'ctx>,
    name: &str,
) -> Fallible<PointerValue<'ctx>> {
    module.add_function("llvm.stacksave", context.i8_type().ptr_type(inkwell::AddressSpace::Generic).fn_type(&[], false), None);
    module.add_function("llvm.stackrestore", context.void_type().fn_type(&[context.i8_type().ptr_type(inkwell::AddressSpace::Generic).into()], false), None);
    let closure_type =
        context.struct_type(&[context.i16_type().into(), context.i8_type().ptr_type(inkwell::AddressSpace::Global).into(), context.i64_type().into()], false);
    let function_type = context.void_type().fn_type(
        &[
            context.i8_type().ptr_type(inkwell::AddressSpace::Global).into(),
            context.i64_type().ptr_type(inkwell::AddressSpace::Local).into(),
            context.i64_type().ptr_type(inkwell::AddressSpace::Local).ptr_type(inkwell::AddressSpace::Local).into(),
            closure_type.ptr_type(inkwell::AddressSpace::Shared).into(),
        ],
        is_var_args,
    );
    let function = module.add_function(name, function_type, None);
    let basic_block = context.append_basic_block(function, "enter");
    let builder = context.create_builder();
    builder.position_at_end(basic_block);

    let return_ptr = function.get_nth_param(1).unwrap().into_pointer_value();
    let args = function.get_nth_param(2).unwrap().into_pointer_value();
    let metadata = function.get_nth_param(3).unwrap().into_pointer_value();
    let reg_count_ptr = builder.build_struct_gep(metadata, 0, "reg_count_ptr").unwrap();
    let reg_count = builder.build_load(reg_count_ptr, "reg_count").into_int_value();

    let stack_state = builder.build_call(module.get_function("llvm.stacksave").unwrap(), &[], "stack_state").try_as_basic_value().left().unwrap();

    let regs = builder.build_address_space_cast(
        builder.build_array_alloca(context.i64_type(), reg_count, "registers"),
        context.i64_type().ptr_type(inkwell::AddressSpace::Local),
        "registers_local",
    );
    // let arg_size = optional_arg_size
    //     .map(|arg_size| context.i64_type().const_int(arg_size as u64, false))
    //     .unwrap_or_else(|| {
    //         builder
    //             .build_load(
    //                 builder
    //                     .build_struct_gep(metadata, 3, "arg_size_ptr")
    //                     .unwrap(),
    //                 "arg_size",
    //             )
    //             .into_int_value()
    //     });
    // builder.build_memmove(regs, 8, args, 8, arg_size).unwrap();
    if let Some(arg_count) = optional_arg_count {
        for i in 0..arg_count {
            unsafe {
                let index_value = context.i64_type().const_int(i as u64, false);
                let arg_ptr = builder.build_load(builder.build_gep(args, &[index_value], "arg_ptr_ptr"), "arg").into_pointer_value();
                let arg = builder.build_load(arg_ptr, "arg");
                builder.build_store(builder.build_gep(regs, &[index_value], "reg_ptr"), arg);
            }
        }
    } else {
        // TODO
    }
    let instruction_count = instructions.get_type().get_element_type().into_array_type().len();
    let opcode_len = match instruction_count {
        0..=0xff => 1,
        0x100..=0xffff => 2,
        _ => 4,
    };
    let opcode_type = context.custom_width_int_type(u8::BITS * opcode_len);
    let code_ptr = builder.build_struct_gep(metadata, 1, "code").unwrap();
    let ip = builder.build_load(code_ptr, "ip").into_pointer_value();
    let ip = builder.build_pointer_cast(ip, opcode_type.ptr_type(inkwell::AddressSpace::Global), "ip");
    let opcode = builder.build_load(ip, "opcode").into_int_value();
    let opcode = builder.build_int_z_extend(opcode, context.custom_width_int_type(usize::BITS), "opcode_z_entend");
    let instruction_ptr = unsafe { builder.build_in_bounds_gep(instructions, &[context.i64_type().const_int(0, true), opcode], "instruction_ptr") };
    let instruction: CallableValue<'ctx> = builder.build_load(instruction_ptr, "instruction").into_pointer_value().try_into().unwrap();
    let call = builder.build_call(instruction, &[regs.into(), ip.into()], "call");
    call.set_call_convention(18); // tailcc
    builder.build_store(return_ptr, call.try_as_basic_value().unwrap_left());
    builder.build_call(module.get_function("llvm.stackrestore").unwrap(), &[stack_state.into()], "restore");

    builder.build_return(None);
    return Ok(function.as_global_value().as_pointer_value());
}
#[derive(Getters, CopyGetters)]
pub struct FunctionBinder {
    #[getset(get = "pub")]
    enter_points: Vec<Callback<FunctionMetadata, i64>>,
    #[getset(get = "pub")]
    enter_points_va_arg: Vec<Callback<FunctionMetadata, i64>>,
    #[getset(get_copy = "pub")]
    enter_point_mul_arg: Callback<FunctionMetadata, i64>,
    #[getset(get_copy = "pub")]
    enter_point_mul_arg_va_arg: Callback<FunctionMetadata, i64>,
}
fn convert_type(vm_type: &vm_core::Type) -> Type {
    match vm_type {
        vm_core::Type::Float(vm_core::FloatKind::F32) => Type::f32(),
        vm_core::Type::Float(vm_core::FloatKind::F64) => Type::f64(),
        vm_core::Type::Int(int_kind) => match int_kind {
            vm_core::IntKind::Bool => Type::c_int(),
            vm_core::IntKind::I8 => Type::i8(),
            vm_core::IntKind::I16 => Type::i16(),
            vm_core::IntKind::I32 => Type::i32(),
            vm_core::IntKind::I64 => Type::i64(),
            vm_core::IntKind::U8 => Type::u8(),
            vm_core::IntKind::U16 => Type::u16(),
            vm_core::IntKind::U32 => Type::u32(),
            vm_core::IntKind::U64 => Type::u64(),
            _ => Type::pointer(),
        },
        _ => Type::pointer(),
    }
}
impl FunctionBinder {
    pub(crate) fn bind<'ctx>(
        &self,
        code: ObjectRef,
        function_type: &FunctionType,
        register_count: u16,
        output: ObjectRef,
        context: Arc<Context>,
    ) -> Fallible<ObjectRef> {
        let mut args_type = Vec::with_capacity(function_type.args.len());
        for arg_type in &function_type.args {
            args_type.push(convert_type(arg_type));
        }
        if let Some(va_arg_type) = function_type.va_arg() {
            args_type.push(convert_type(&vm_core::Type::Tuple(Tuple::Normal(CowArc::Owned(
                vec![vm_core::Type::Pointer(CowArc::new(va_arg_type.clone())), vm_core::Type::Int(IntKind::Usize)].into(),
            )))))
        }
        let ret_type = if let Some(ret) = &function_type.return_type { convert_type(ret) } else { Type::void() };
        let cif = Cif::new(args_type, ret_type);
        let callback = self.enter_points.get(function_type.args().len()).copied().unwrap_or(self.enter_point_mul_arg);
        GhostToken::new(|mut token| {
            let object_builder = ObjectBuilder::default();
            let metadata_memory: &mut MaybeUninit<FunctionMetadata> = object_builder.borrow_mut(&mut token).receive();
            let metadata_ptr_mut = unsafe { metadata_memory.assume_init_mut() as *mut FunctionMetadata };
            let closure = unsafe { Closure::new(cif, callback, metadata_ptr_mut.as_ref().unwrap()) };
            let entry = unsafe { std::mem::transmute(*closure.code_ptr()) };
            metadata_memory.write(FunctionMetadata { register_count, code: null(), args_count: function_type.args.len(), bind: entry, context, closure });
            let offset = unsafe {
                let metadata = metadata_memory.assume_init_ref();
                &metadata.code as *const *const u8 as usize - metadata as *const FunctionMetadata as usize
            };
            let bind_offset = unsafe {
                let metadata = metadata_memory.assume_init_ref();
                &metadata.bind as *const _ as usize - metadata as *const FunctionMetadata as usize
            };
            object_builder.borrow_mut(&mut token).set_pin(true);
            ObjectBuilderInner::set_import(&object_builder, &mut token, offset, ObjectBuilderImport::ObjectRef(code), RelocationKind::UsizePtrAbsolute, 0);
            object_builder.borrow_mut(&mut token).add_symbol(SymbolBuilder::default().offset(bind_offset).symbol_kind(vm_core::SymbolKind::Value).build()?);
            Fallible::Ok(object_builder.take(&mut token).build_into(output)?)
        })
    }

    pub(crate) fn generate<'ctx>(context: &'ctx Context, module: &Module<'ctx>, instructions: PointerValue<'ctx>, arg_count: usize) -> Fallible<()> {
        for i in 0..arg_count {
            get_callback(context, instructions, Some(i), false, module, &format!("ffi_callback_with_arg_count_{}", i))?;
            get_callback(context, instructions, Some(i), true, module, &format!("ffi_callback_with_va_arg_with_arg_count_{}", i))?;
        }
        get_callback(context, instructions, None, false, module, "ffi_callback")?;
        get_callback(context, instructions, None, true, module, "ffi_callback_va_arg")?;
        Ok(())
    }

    pub(crate) fn from_jit(execution_engine: inkwell::execution_engine::ExecutionEngine, arg_count: usize) -> Fallible<Self> {
        let mut enter_points = Vec::with_capacity(arg_count);
        let mut enter_points_va_arg = Vec::with_capacity(arg_count);
        // let e = execution_engine.get_function::<Ca>("ffi_callback");
        let enter_point_mul_arg = unsafe { std::mem::transmute(execution_engine.get_function_address("ffi_callback")?) };
        let enter_point_mul_arg_va_arg = unsafe { std::mem::transmute(execution_engine.get_function_address("ffi_callback_va_arg")?) };
        for i in 0..arg_count {
            unsafe {
                enter_points.push(std::mem::transmute(execution_engine.get_function_address(&format!("ffi_callback_with_arg_count_{}", i))?));
                enter_points_va_arg
                    .push(std::mem::transmute(execution_engine.get_function_address(&format!("ffi_callback_with_va_arg_with_arg_count_{}", i))?));
            }
        }
        Ok(Self { enter_points, enter_points_va_arg, enter_point_mul_arg, enter_point_mul_arg_va_arg })
    }
}
