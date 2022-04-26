use std::sync::Arc;

use failure::Fallible;
pub trait Code {}
pub trait MachineCode: Code {}
pub trait Deployer<E: Code> {
    fn deploy(&self, executable: E) -> Fallible<()>;
}
pub trait Compiler<S: Code, T: Code> {
    type Report;
    fn compiler(&self, source: S) -> Fallible<(T, Self::Report)>;
}
pub trait Stub {
    fn get_code<C: Code>(&self) -> &C;
    fn add_code<C: Code>(&self, c: Arc<C>) -> Fallible<bool>;
}
pub trait CompilerComposer {}
