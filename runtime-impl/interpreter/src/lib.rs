#![feature(ptr_metadata)]
#![feature(iterator_try_collect)]
use std::{
    any::Any,
    cell::RefCell,
    fmt::Debug,
    marker::PhantomData,
    rc::Rc,
    sync::{Arc, Mutex, RwLock, Weak},
};

use arc_swap::{access::Access, ArcSwapAny};
use either::Either;
use enter_point::FunctionBinder;
use failure::{format_err, Fallible};
use genarator::GlobalBuilder;
use getset::Getters;
use inkwell::context::Context;
use runtime::{
    code::FunctionPack,
    instructions::{InstructionSet, InstructionType, MemoryInstructionSet},
    mem::MemoryInstructionSetProvider,
};
use util::AsAny;
use vm_core::{
    AtomicObjectRef, AtomicObjectWeekRef, Component, ExecutableResourceTrait, FunctionType, LockedObjectInner, ObjectRef, ObjectWeekRef, Resource,
    ResourceError, ResourceFactory, RuntimeTrait,
};

mod enter_point;
mod genarator;
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
            let global_builder = Rc::new(RefCell::new(GlobalBuilder { symbol_maps: Default::default(), module, context: context_ref }));
            let instruction_functions = genarator::LLVMFunctionBuilder::generate_instruction_set(
                instructions,
                instruction_count,
                context_ref,
                global_builder.clone(),
                memory_instruction_set,
                name,
            )?;
            let GlobalBuilder { symbol_maps, module, .. } = Rc::try_unwrap(global_builder).unwrap().into_inner();
            // if let Some(i) = module.get_function("instruction_MakeSlice") {
            //     i.print_to_stderr()
            // }
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
    type ResourceImpl = FunctionResource;

    fn define(&self) -> Fallible<Arc<Self::ResourceImpl>> {
        Ok(Arc::new(Default::default()))
    }

    fn upload(&self, resource: &Self::ResourceImpl, input: FunctionPack<S>) -> Fallible<()> {
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
        *this.inner.write().unwrap() = Some(FunctionResourceInner { ir, bind });
        Ok(this)
    }
}
impl<S: InstructionSet, M: MemoryInstructionSetProvider> RuntimeTrait<FunctionPack<S>> for Interpreter<S, M> {}
#[derive(Getters, Default)]
#[getset(get = "pub")]
pub struct FunctionResourceInner {
    ir: ObjectRef,
    bind: ObjectRef,
}
#[derive(Getters, Default)]
#[getset(get = "pub")]
pub struct FunctionResource {
    inner: RwLock<Option<FunctionResourceInner>>,
}
impl<S> ExecutableResourceTrait<FunctionPack<S>> for FunctionResource {
    fn get_object(&self) -> Fallible<ObjectRef> {
        let inner = self.inner.read().unwrap();
        Ok(inner.as_ref().ok_or_else(|| ResourceError::NotLoaded)?.bind.clone())
    }
}
unsafe impl Send for FunctionResource {}
unsafe impl Sync for FunctionResource {}
impl AsAny for FunctionResource {
    fn as_any(&self) -> &(dyn Any) {
        self
    }

    fn as_any_arc(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }
}
impl Debug for FunctionResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.read().unwrap();
        f.debug_struct("FunctionBind").field("ir", &inner.as_ref().map(|inner| inner.bind.clone())).finish()
    }
}

impl Component for FunctionResource {}
impl<M> Resource<FunctionPack<M>> for FunctionResource {
    fn get_state(&self) -> vm_core::ResourceState {
        todo!()
    }
}
