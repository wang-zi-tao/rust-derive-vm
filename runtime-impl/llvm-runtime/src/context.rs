use failure::{format_err, Fallible};
use inkwell::{context::Context as LLVMContext, execution_engine::ExecutionEngine, module::Module};
use std::{ops::Deref, ptr::NonNull};

pub(crate) struct RuntimeContext {
    context: NonNull<LLVMContext>,
    module: Option<Module<'static>>,
    execution_engine: Option<ExecutionEngine<'static>>,
}
unsafe impl Send for RuntimeContext {}

impl Default for RuntimeContext {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for RuntimeContext {
    type Target = LLVMContext;

    fn deref(&self) -> &Self::Target {
        unsafe { self.context.as_ref() }
    }
}
impl RuntimeContext {
    pub fn new() -> Self {
        let context: &'static _ = Box::leak(Box::new(LLVMContext::create()));
        Self { context: NonNull::from(context), module: None, execution_engine: None }
    }

    pub fn execution_engine(&self) -> Option<&ExecutionEngine<'static>> {
        self.execution_engine.as_ref()
    }

    pub fn create_root_module<'l>(&'l mut self, name: &str) -> &'l Module<'static> {
        let context = unsafe { self.context() };
        self.module.get_or_insert_with(|| context.create_module(name))
    }

    pub fn create_execution_engine(&mut self, module: &Module<'static>) -> Fallible<&ExecutionEngine<'static>> {
        if let Some(execution_engine) = &mut self.execution_engine {
            execution_engine.add_module(&module).map_err(|_| format_err!("failed to add module"))?;
        } else {
            self.execution_engine =
                Some(module.create_jit_execution_engine(inkwell::OptimizationLevel::Aggressive).map_err(|e| format_err!("llvm error: {:?}", e))?);
        }
        Ok(self.execution_engine.as_ref().unwrap())
    }

    pub unsafe fn context(&self) -> &'static LLVMContext {
        self.context.as_ref()
    }

    pub fn set_module(&mut self, module: Option<Module<'static>>) {
        self.module = module;
    }

    pub fn module(&self) -> Option<&Module<'static>> {
        self.module.as_ref()
    }

    pub fn set_execution_engine(&mut self, execution_engine: Option<ExecutionEngine<'static>>) {
        self.execution_engine = execution_engine;
    }
}

impl Drop for RuntimeContext {
    fn drop(&mut self) {
        self.execution_engine = None;
        self.module = None;
        let _ = unsafe { Box::from_raw(self.context.as_ptr()) };
    }
}
