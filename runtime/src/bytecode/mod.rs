use classfile::{attributes::Code, constants::Constant, Method};
use code::byte_code::OpCode;
use frame::FrameState;
use memory::associate::{AssociateStubPoolBuilder, AssociateStubPoolBuilderTrait, ExtendAssociateStubPoolBuilder};
use vm_core::{ClassLoaderRef, ExecutableRef, JavaClassRef};

pub mod code;
pub mod constant_pool;
pub mod frame;
pub mod immediate;
pub mod verify;

pub struct Environment<'l> {
    pub offset: &'l mut u32,
    pub byte_code: &'l ByteCode,
    pub code: &'l Code,
    pub method: &'l Method,
    pub method_ref: &'l ExecutableRef,
    pub constants: &'l Vec<Constant>,
    pub class: &'l JavaClassRef,
    pub class_loader: &'l ClassLoaderRef,
    pub frame: &'l mut FrameState,
}
pub struct ByteCode {
    code: AssociateStubPoolBuilder,
}
impl ExtendAssociateStubPoolBuilder for ByteCode {
    type Super = AssociateStubPoolBuilder;

    fn get_super(&self) -> &Self::Super {
        &self.code
    }

    fn get_super_mut(&mut self) -> &mut Self::Super {
        &mut self.code
    }

    fn take_super(self) -> Self::Super {
        self.code
    }
}
