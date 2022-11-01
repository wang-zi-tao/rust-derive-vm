use failure::{format_err, Fallible};
use inkwell::{context::Context as LLVMContext, execution_engine::ExecutionEngine, module::Module};
use std::{ops::Deref, ptr::NonNull};

pub(crate) struct RuntimeContext {
    context: NonNull<LLVMContext>,
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
        unsafe { &self.context.as_ref() }
    }
}
impl RuntimeContext {
    pub fn new() -> Self {
        let context: &'static _ = Box::leak(Box::new(LLVMContext::create()));
        Self { context: NonNull::from(context), execution_engine: None }
    }

    pub fn execution_engine(&self) -> Option<&ExecutionEngine<'static>> {
        self.execution_engine.as_ref()
    }

    pub fn set_execution_engine(&mut self, execution_engine: Option<ExecutionEngine<'static>>) {
        self.execution_engine = execution_engine;
    }

    pub fn create_execution_engine(&mut self, module: Module<'static>) -> Fallible<()> {
        if let Some(execution_engine) = &mut self.execution_engine {
            execution_engine.add_module(&module).map_err(|_| format_err!("failed to add module"))?;
        } else {
            self.execution_engine = Some(module.create_execution_engine().map_err(|e| format_err!("llvm error: {:?}", e))?);
        }
        Ok(())
    }

    pub(crate) unsafe fn context(&self) -> &'static LLVMContext {
        self.context.as_ref()
    }
}

impl Drop for RuntimeContext {
    fn drop(&mut self) {
        self.execution_engine = None;
        let _ = unsafe { Box::from_raw(self.context.as_ptr()) };
    }
}
