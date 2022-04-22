extern crate jvm_core as vm_core;
use std::{
    any::TypeId,
    fmt::Debug,
    sync::{Arc, Mutex},
};

use failure::{format_err, Fallible};
use inkwell::{
    context::Context as LLVMContext, execution_engine::ExecutionEngine as LLVMExecutionEngine,
    module::Module as LLVMModule,
};
use runtime::RuntimeFilter;
use smallvec::SmallVec;
use vm_core::Module;
pub struct LLVMRuntime<'ctx> {
    context: &'ctx LLVMContext,
    _root_module: LLVMModule<'ctx>,
    execute_engine: LLVMExecutionEngine<'ctx>,
}
pub struct LLVMRuntimeWraped<'ctx>(Mutex<LLVMRuntime<'ctx>>);
impl<'ctx> Debug for LLVMRuntimeWraped<'ctx> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LLVMRuntimeWraped").finish()
    }
}
impl<'ctx> Module for LLVMRuntimeWraped<'ctx> {}
impl<'ctx> RuntimeFilter for LLVMRuntimeWraped<'ctx> {
    fn get_input_type_id(&self) -> std::any::TypeId {
        TypeId::of::<LLVMMoudleBuilder>()
    }

    fn consume<'l>(
        &'l self,
        source_input: Arc<dyn std::any::Any + Send + Sync>,
    ) -> Fallible<SmallVec<[(Arc<dyn std::any::Any + Send + Sync>, &'l dyn RuntimeFilter); 1]>>
    {
        let input: Arc<LLVMMoudleBuilder> = source_input
            .downcast()
            .map_err(|_| format_err!("downcast failed"))?;
        let mut guard = self
            .0
            .lock()
            .map_err(|e| format_err!("lock error,message:\n{:#?}", e))?;
        guard.build_module(&*input)?;
        Ok(SmallVec::new())
    }
}
pub type LLVMMoudleBuilder =
    Box<dyn Send + Sync + for<'ctx> Fn(&'ctx LLVMContext) -> Fallible<LLVMModule>>;
unsafe impl<'ctx> Send for LLVMRuntime<'ctx> {}
impl<'ctx> LLVMRuntime<'ctx> {
    pub fn new(context: &'ctx LLVMContext) -> Fallible<Self> {
        let root_module = context.create_module("/");
        let execute_engine = root_module
            .create_jit_execution_engine(inkwell::OptimizationLevel::Default)
            .map_err(|m| format_err!("error when creating jit execution engine,message:{}", m))?;
        Ok(Self {
            context,
            _root_module: root_module,
            execute_engine,
        })
    }

    pub fn build_module(
        &mut self,
        builder: impl FnOnce(&'ctx LLVMContext) -> Fallible<LLVMModule>,
    ) -> Fallible<()> {
        let module = builder(self.context)?;
        self.execute_engine
            .add_module(&module)
            .map_err(|_| format_err!("error when add module into the execute_engine"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn llvm_runtime() {
        let context = inkwell::context::Context::create();
        let mut llvm_runtime = LLVMRuntime::new(&context).unwrap();
        llvm_runtime
            .build_module(|context| {
                let builder = context.create_builder();
                let module = context.create_module("test");
                let function =
                    module.add_function("f", context.void_type().fn_type(&[], false), None);
                let basic_block = context.append_basic_block(function, "entry");
                builder.position_at_end(basic_block);
                builder.build_return(None);
                Ok(module)
            })
            .unwrap();
        assert_ne!(
            0,
            llvm_runtime
                .execute_engine
                .get_function_address("f")
                .unwrap()
        );
    }
}
