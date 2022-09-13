use getset::{CopyGetters, Getters};
use inkwell::execution_engine::ExecutionEngine;

#[derive(Getters, CopyGetters)]
struct Context {
    #[getset(get = "pub")]
    context: &'static Context,
    execute_engine: ExecutionEngine<'static>,
}
impl Context {
    fn new(context: &'static Context, execute_engine: ExecutionEngine<'static>) -> Self {
        Self { context, execute_engine }
    }
}
