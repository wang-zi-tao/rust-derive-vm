use std::{
    alloc::{Layout, LayoutError},
    cell::RefCell,
    collections::{HashMap, HashSet},
    convert::TryInto,
    fmt::Debug,
    marker::PhantomData,
    mem::{align_of, size_of},
    num::TryFromIntError,
    ptr::NonNull,
    rc::Rc,
    sync::Arc,
    usize,
};

use failure::{format_err, Error, Fail, Fallible};
use getset::Getters;
use inkwell::{
    basic_block::BasicBlock,
    context::Context,
    execution_engine::{ExecutionEngine},
    module::Module,
    support::LLVMString,
    values::{FunctionValue, PointerValue},
    AddressSpace,
};
use runtime::{
    code::FunctionPack,
    instructions::{InstructionSet, InstructionType, MemoryInstructionSet},
    mem::MemoryInstructionSetProvider,
};

use util::{FromNode};
use util_derive::AsAny;
use vm_core::{
    Component, ExecutableResourceTrait, FunctionType, ObjectBuilder, ObjectRef, Resource, ResourceError, ResourceFactory, RuntimeTrait, SymbolBuilder, Type, _ghost_cell::GhostToken,
};

#[derive(Debug, Fail)]
pub enum JITCompileError {
    #[fail(display = "Opcode was not found in index {}", _0)]
    OpcodeOutOfBound(usize),
    #[fail(display = "Parameter was not found in index {}", _0)]
    ParamIndexOutOfBound(usize),
    #[fail(display = "Offset index out of bounds {}", _0)]
    OffsetOutOfBound(usize),
    #[fail(display = "{}", _0)]
    InstructionError(#[cause] InstructionError),
    #[fail(display = "llvm error: {}", _0)]
    LLVMError(String),
    #[fail(display = "{}", _0)]
    OtherError(#[cause] Error),
}
use JITCompileError::*;
impl From<LayoutError> for JITCompileError {
    fn from(error: LayoutError) -> Self {
        OtherError(error.into())
    }
}
impl From<TryFromIntError> for JITCompileError {
    fn from(error: TryFromIntError) -> Self {
        OtherError(error.into())
    }
}
impl From<Error> for JITCompileError {
    fn from(error: Error) -> Self {
        OtherError(error)
    }
}
impl From<InstructionError> for JITCompileError {
    fn from(error: InstructionError) -> Self {
        InstructionError(error)
    }
}
impl From<LLVMString> for JITCompileError {
    fn from(error: LLVMString) -> Self {
        LLVMError(error.to_string())
    }
}
type Result<T> = std::result::Result<T, JITCompileError>;

use crate::genarator::{function_type_to_llvm_type, vm_type_to_llvm_type, GlobalBuilder, InstructionError, LLVMFunctionBuilder};
pub enum JITConstantKind {
    Const(Type, usize),
    Mut(Type, usize),
    BasicBlock(usize),
    State,
}
pub struct JITInstruction {
    pub(crate) function: FunctionValue<'static>,
    pub(crate) align: usize,
    pub(crate) size: usize,
    pub(crate) is_returned: bool,
    pub(crate) operand_types: Vec<Type>,
    pub(crate) constant_size: usize,
    pub(crate) constants: Vec<JITConstantKind>,
}
#[derive(Getters)]
#[getset(get = "pub")]
pub struct RawJITCompiler {
    instructions: Vec<JITInstruction>,
    context: NonNull<Context>,
    module: Arc<Module<'static>>,
    execution_engine: ExecutionEngine<'static>,
}
unsafe impl Sync for RawJITCompiler {}
unsafe impl Send for RawJITCompiler {}

impl Drop for RawJITCompiler {
    fn drop(&mut self) {
        self.instructions.clear();
        let _ = unsafe { Box::from_raw(self.context.as_ptr()) };
    }
}
impl RawJITCompiler {
    pub fn new((instructions, instruction_count): (&[(usize, InstructionType)], usize), memory_instruction_set: &MemoryInstructionSet) -> Result<Self> {
        let context = Box::leak(Box::new(Context::create()));
        let mut context_non_null = NonNull::from(context);
        let context: &'static mut Context = unsafe { context_non_null.as_mut() };
        let module = Arc::new(context.create_module("jit"));
        let global_builder = Rc::new(RefCell::new(GlobalBuilder {
            symbol_maps: Default::default(),
            module: module.clone(),
            context,
            memory_instruction_set: memory_instruction_set.clone(),
        }));
        let jit_instructions = LLVMFunctionBuilder::generate_instruction_set_jit(instructions, instruction_count, context, global_builder.clone())?;
        let GlobalBuilder { symbol_maps, module, .. } = Rc::try_unwrap(global_builder).unwrap().into_inner();
        let execution_engine = module.create_jit_execution_engine(inkwell::OptimizationLevel::Aggressive).map_err(|e| format_err!("llvm error: {}", e))?;
        for (symbol, ptr) in symbol_maps {
            execution_engine.add_global_mapping(&symbol, ptr as usize);
        }
        let this = Self { instructions: jit_instructions, context: context_non_null, module, execution_engine };
        Ok(this)
    }

    pub fn generate_function<'ctx>(&self, ir: &ObjectRef, function_type: &FunctionType) -> Result<FunctionValue<'ctx>> {
        let context: &'static mut Context = unsafe { self.context.clone().as_mut() };
        let module = &self.module;
        let usize_type = context.custom_width_int_type(usize::BITS);
        let mut regs = HashMap::<(u16, Type), PointerValue<'ctx>>::new();
        let function_llvm_type = function_type_to_llvm_type(function_type, context)?;
        let function = module.add_function("jited_ir_", function_llvm_type, None);
        let mut blocks = HashMap::<usize, JITBasicBlock<'ctx>>::new();
        let entry_block = context.append_basic_block(function, "entry");
        let entry_builder = context.create_builder();
        entry_builder.position_at_end(entry_block);
        let mut params_layout = Layout::new::<()>();
        for (param_index, param_type) in function_type.args.iter().enumerate() {
            let llvm_type = vm_type_to_llvm_type(param_type, context)?;
            let param_layout: Layout = param_type.get_layout()?.into();
            let param_layout = Layout::from_size_align(param_layout.size().max(size_of::<usize>()), param_layout.align().max(align_of::<usize>()))?;
            let (new_layout, offset) = params_layout.extend(param_layout)?;
            params_layout = new_layout;
            let reg = offset / size_of::<usize>();
            let reg_pointer = entry_builder.build_alloca(llvm_type, &format!("reg_param_{}", param_index));
            let param = function.get_nth_param(param_index.try_into()?).ok_or(ParamIndexOutOfBound(param_index))?;
            entry_builder.build_store(reg_pointer, param);
            regs.insert((reg.try_into()?, param_type.clone()), reg_pointer);
        }
        let jump_to = entry_builder.build_alloca(usize_type, "jump_to");
        let block = context.append_basic_block(function, "entry");
        let builder = context.create_builder();
        builder.position_at_end(block);
        let mut error_block = None;
        let locked_ir = ir.lock().unwrap();
        let mut ir_buffer = locked_ir.get_buffer().clone();
        let mut tasks = vec![0usize];
        let mut finished_task = HashSet::new();
        let opcode_size = match self.instructions.len() {
            0..=0xff => 1,
            0x100..=0xffff => 2,
            0x10000..=0xffffffff => 4,
            _ => 8,
        };
        let mut ip = 0;
        blocks.insert(ip, JITBasicBlock { offset: ip, llvm_block: block });
        let first_block = block;
        while let Some(block_start) = tasks.pop() {
            if finished_task.contains(&block_start) {
                continue;
            }
            finished_task.insert(block_start);
            let block = blocks.get(&block_start).unwrap();
            builder.position_at_end(block.llvm_block);
            loop {
                let opcode = match opcode_size {
                    1 => unsafe { ir_buffer.get::<u8>(ip) as usize },
                    2 => unsafe { ir_buffer.get::<u16>(ip) as usize },
                    4 => unsafe { ir_buffer.get::<u32>(ip).try_into()? },
                    8 => unsafe { ir_buffer.get::<u64>(ip).try_into()? },
                    _ => unreachable!(),
                };
                let jit_instruction = self.instructions.get(opcode).ok_or(OpcodeOutOfBound(opcode))?;
                let constant_start = (ip + opcode_size + (jit_instruction.align - 1)) & !(jit_instruction.align - 1);
                let instruction_function = jit_instruction.function;
                let params = instruction_function.get_type().get_param_types();
                let mut args = Vec::with_capacity(params.len());
                args.push(jump_to.into());
                let mut goto_list = Vec::new();
                for (index, (&llvm_type, constants)) in
                    params.get(0..jit_instruction.constants.len()).unwrap().iter().zip(jit_instruction.constants.iter()).enumerate()
                {
                    match constants {
                        JITConstantKind::Const(_, constant_offset) | JITConstantKind::Mut(_, constant_offset) => {
                            let ptr: NonNull<u8> = ir_buffer.get_ptr(constant_start + constant_offset);
                            let pointer_value = usize_type.const_int((ptr.as_ptr() as usize).try_into()?, false);
                            let pointer_value = builder.build_int_to_ptr(pointer_value, llvm_type.into_pointer_type(), &format!("constnat_{}", index));
                            args.push(pointer_value.into());
                        }
                        JITConstantKind::BasicBlock(constant_offset) => {
                            let offset: i32 = unsafe {
                                ir_buffer.try_get(constant_start + constant_offset).ok_or_else(|| OffsetOutOfBound(constant_start + constant_offset))?
                            };
                            let target = constant_start + constant_offset + offset as usize;
                            goto_list.push(target);
                            args.push(usize_type.const_int(target.try_into()?, false).into());
                        }
                        JITConstantKind::State => {
                            let global =
                                module.add_global(llvm_type.into_pointer_type().get_element_type().into_int_type(), Some(AddressSpace::Global), "state");
                            args.push(global.as_pointer_value().into());
                        }
                    }
                }
                for (index, operand_type) in jit_instruction.operand_types.iter().enumerate() {
                    let reg = unsafe {
                        ir_buffer
                            .try_get(constant_start + jit_instruction.constant_size + 2 * index)
                            .ok_or(OffsetOutOfBound(constant_start + jit_instruction.constant_size + 2 * index))?
                    };
                    let reg_pointer = match regs.entry((reg, operand_type.clone())) {
                        std::collections::hash_map::Entry::Occupied(o) => *o.get(),
                        std::collections::hash_map::Entry::Vacant(v) => {
                            *v.insert(entry_builder.build_alloca(vm_type_to_llvm_type(operand_type, context)?, &format!("reg_{}_", reg)))
                        }
                    };
                    args.push(reg_pointer.into());
                }
                let ret = builder.build_call(instruction_function, &*args, &format!("call_{}", ip));
                if jit_instruction.is_returned {
                    if let Some(ret) = ret.try_as_basic_value().left() {
                        builder.build_return(Some(&ret));
                    } else {
                        builder.build_return(None);
                    }
                } else if !goto_list.is_empty() {
                    let jump_to_value = builder.build_load(jump_to, "jump_to_value").into_int_value();
                    let mut switch_cases = Vec::with_capacity(goto_list.len());
                    for goto in goto_list {
                        let goto_block = blocks
                            .entry(goto)
                            .or_insert_with(|| JITBasicBlock { offset: goto, llvm_block: context.append_basic_block(function, &format!("block_{}", goto)) });
                        if !finished_task.contains(&goto) {
                            tasks.push(goto);
                        }
                        switch_cases.push((usize_type.const_int(goto.try_into()?, false), goto_block.llvm_block));
                    }
                    let else_block = error_block.get_or_insert_with(|| {
                        let error_block = context.append_basic_block(function, "error_block");
                        let error_builder = context.create_builder();
                        error_builder.position_at_end(error_block);
                        error_builder.build_unreachable();
                        error_block
                    });
                    builder.build_switch(jump_to_value, *else_block, &*switch_cases);
                    break;
                }
                ip = constant_start + constant_start + jit_instruction.constant_size + 2 * jit_instruction.operand_types.len();
            }
        }
        entry_builder.build_unconditional_branch(first_block);
        Ok(function)
    }

    pub fn wrap_function(&self, function: FunctionValue<'static>, output: ObjectRef) -> Fallible<ObjectRef> {
        let address = self.execution_engine.get_function_address(&function.get_name().to_string_lossy())?;
        GhostToken::new(|mut token| {
            let builder = ObjectBuilder::default();
            builder.borrow_mut(&mut token).push(address);
            builder.borrow_mut(&mut token).add_symbol(SymbolBuilder::default().offset(0).symbol_kind(vm_core::SymbolKind::Value).build().unwrap());
            builder.take(&mut token).build_into(output)
        })
    }
}
struct JITBasicBlock<'ctx> {
    offset: usize,
    llvm_block: BasicBlock<'ctx>,
}
#[derive(Getters)]
#[getset(get = "pub")]
pub struct JITCompiler<S: InstructionSet, M: MemoryInstructionSetProvider> {
    raw: RawJITCompiler,
    _ph: PhantomData<(S, M)>,
}

unsafe impl<S: InstructionSet, M: MemoryInstructionSetProvider> Send for JITCompiler<S, M> {}
unsafe impl<S: InstructionSet, M: MemoryInstructionSetProvider> Sync for JITCompiler<S, M> {}

impl<S: InstructionSet, M: MemoryInstructionSetProvider> JITCompiler<S, M> {
    pub fn new() -> Fallible<Self> {
        let raw = RawJITCompiler::new((&S::INSTRUCTIONS, S::INSTRUCTION_COUNT), &*M::get_memory_instruction_set()?)?;
        Ok(Self { raw, _ph: PhantomData })
    }

    pub fn compile(&self, pack: FunctionPack<S>) -> Fallible<ObjectRef> {
        let function_value = self.raw.generate_function(pack.byte_code(), pack.function_type())?;
        let function = self.raw.wrap_function(function_value, pack.output.unwrap_or_default())?;
        Ok(function)
    }
}
#[derive(Getters, Default, Debug, AsAny)]
#[getset(get = "pub")]
pub struct JITFunction {
    ir: ObjectRef,
    function: ObjectRef,
}
unsafe impl Send for JITFunction {}
unsafe impl Sync for JITFunction {}

impl Component for JITFunction {}
impl<M> Resource<FunctionPack<M>> for JITFunction {
    fn get_state(&self) -> vm_core::ResourceState {
        vm_core::ResourceState::Ready
    }
}
impl<S: InstructionSet, M: MemoryInstructionSetProvider> Debug for JITCompiler<S, M> {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
impl<S: InstructionSet, M: MemoryInstructionSetProvider> vm_core::Module for JITCompiler<S, M> {}
impl<S: InstructionSet, M: MemoryInstructionSetProvider> ResourceFactory<FunctionPack<S>> for JITCompiler<S, M> {
    type ResourceImpl = JITFunction;

    fn define(&self) -> Fallible<Arc<Self::ResourceImpl>> {
        Ok(Arc::new(Default::default()))
    }

    fn upload(&self, _resource: &Self::ResourceImpl, _input: FunctionPack<S>) -> Fallible<()> {
        Err(ResourceError::Unsupported.into())
    }

    fn create(&self, input: FunctionPack<S>) -> Fallible<Arc<Self::ResourceImpl>> {
        let ir = input.byte_code().clone();
        let function = self.compile(input)?;
        Ok(Arc::new(JITFunction { ir, function }))
    }
}
impl<S> ExecutableResourceTrait<FunctionPack<S>> for JITFunction {
    fn get_object(&self) -> Fallible<ObjectRef> {
        Ok(self.function.clone())
    }
}
impl<S: InstructionSet, M: MemoryInstructionSetProvider> RuntimeTrait<FunctionPack<S>> for JITCompiler<S, M> {}
