#![feature(const_generics)]
#![feature(associated_type_defaults)]
#![feature(const_fn)]

pub(crate) mod template_interpreter_x86_64;
#[cfg(target_arch = "x86_64")]
pub mod template_interpreter_arch {

    pub use super::template_interpreter_x86_64::*;
}
pub mod interpreter {}
pub mod template_interpreter {

    use crate::memory::{AssociateStubPoolBuilderTrait, Label};
    use arch::{assembler::Condition, GenericRegister, RegisterBaseArch};
    use runtime::bytecode::ImmediateKind;
    use std::ops::Index;
    use util::Result;

    pub trait StackBufferStateTrait: Sized + Clone {
        type Arch: ArchitectureSupportTrait<UsingStackBuffer = Self::UsingStackBuffer>;
        type UsingStackBuffer;
    }
    pub trait EnterPointsTrait: Sized {
        type StackBufferState: StackBufferStateTrait<Arch = Self::Arch>;
        type Arch: ArchitectureSupportTrait;
        fn new_byte_code_deployment_table(
            assembler: &mut <Self::Arch as ArchitectureSupportTrait>::Assembler,
        ) -> Result<Self>;
    }
    pub trait TemplateTrait {
        type Reg: GenericRegister;
        type Arch: ArchitectureSupportTrait<Reg = Self::Reg>;
        fn generate<M>(&self, factor: &mut Factory<M, Self::Arch>)
        where
            M: MemoryModelOnArchSupportTrait<Arch = Self::Arch>;
    }
    pub trait UniversalTemplateTrait {
        type Assembler: AssociateStubPoolBuilderTrait;
        type StackBufferState: StackBufferStateTrait<
            Arch = Self::Arch,
            UsingStackBuffer = Self::UsingStackBuffer,
        >;
        type UsingStackBuffer;
        type Arch: ArchitectureSupportTrait<
            Assembler = Self::Assembler,
            StackBufferState = Self::StackBufferState,
            UsingStackBuffer = Self::UsingStackBuffer,
        >;

        fn generate_core(
            &self,
            assembler: &mut Self::Assembler,
            stack_state: Self::StackBufferState,
        ) -> Label;
        fn generate_core_and_change_ip<M>(
            &self,
            factor: &mut Factory<M, Self::Arch>,
            stack_state: Self::StackBufferState,
            _buffer: &Self::UsingStackBuffer,
        ) -> Label
        where
            M: MemoryModelOnArchSupportTrait<Arch = Self::Arch>,
        {
            if self.auto_change_ip() {
                let immediate_size: u16 = self.immediate_kind().iter().map(|i| i.size()).sum();
                let instruction_size: u16 = 1u16 + immediate_size;
                factor
                    .arch_model
                    .generate_change_ip(&mut factor.assembler, instruction_size as i32)
            }
            self.generate_core(&mut factor.assembler, stack_state)
        }
        fn auto_change_ip(&self) -> bool;
        fn immediate_kind(&self) -> &Vec<ImmediateKind>;
        fn stack_pop(&self) -> u16;
        fn stack_push(&self) -> u16;

        fn generate_for_state_and_deploy<M>(
            &self,
            factor: &mut Factory<M, Self::Arch>,
            state: Self::StackBufferState,
            opcode: u8,
        ) where
            M: MemoryModelOnArchSupportTrait<Arch = Self::Arch>;
    }

    pub trait MemoryModelOnArchSupportTrait: Sized + Default {
        type StackBufferState: StackBufferStateTrait =
            <Self::Arch as ArchitectureSupportTrait>::StackBufferState;
        // type Template: UniversalTemplateTrait = <Self::Arch as ArchitectureSupportTrait>::Template;
        type Arch: ArchitectureSupportTrait;
        fn generate_into(factor: &mut Factory<Self, Self::Arch>) -> Result<()>;
        fn generate_null_exception_entry(factor: &mut Factory<Self, Self::Arch>) -> Result<()>;
        fn generate_array_index_out_of_bounds_exception_exception_entry(
            factor: &mut Factory<Self, Self::Arch>,
        ) -> Result<()>;
        fn generate_array_store_exception_exception_entry(
            factor: &mut Factory<Self, Self::Arch>,
        ) -> Result<()>;
        fn generate_negative_array_size_exception_entry(
            factor: &mut Factory<Self, Self::Arch>,
        ) -> Result<()>;
        fn generate_illegal_monitor_state_exception_entry(
            factor: &mut Factory<Self, Self::Arch>,
        ) -> Result<()>;
    }
    pub trait ArchitectureSupportTrait: Sized {
        type StackBufferState: StackBufferStateTrait<
            Arch = Self,
            UsingStackBuffer = Self::UsingStackBuffer,
        >;
        type EnterPoints: EnterPointsTrait<Arch = Self, StackBufferState = Self::StackBufferState>
            + Index<Self::StackBufferState, Output = Label>;
        type Assembler: AssociateStubPoolBuilderTrait;
        type Reg: GenericRegister;
        type Arch: RegisterBaseArch<GenericRegister = Self::Reg>;
        type UsingStackBuffer;

        fn generate_into<M>(factor: &mut Factory<M, Self>) -> Result<()>
        where
            M: MemoryModelOnArchSupportTrait<Arch = Self>;

        fn generate_change_ip(&self, assembler: &mut Self::Assembler, change: i32);
        fn generate_get_immediate_to_reg(
            &self,
            assembler: &mut Self::Assembler,
            offset: i32,
            immediate_kind: &ImmediateKind,
            dst: &Self::Reg,
        );
        fn generate_table_jump(&self, assembler: &mut Self::Assembler, jump_table: Label);
        fn generate_table_jump_alternative(
            &self,
            assembler: &mut Self::Assembler,
            condition: Condition,
            then_jump_table: Label,
            else_jump_table: Label,
        );
        fn generate_table_jump_to_state<M>(
            factor: &mut Factory<M, Self>,
            state: Self::StackBufferState,
        ) where
            M: MemoryModelOnArchSupportTrait<Arch = Self>,
        {
            let table = factor.enter_points[state];
            factor
                .arch_model
                .generate_table_jump(&mut factor.assembler, table);
        }
        fn generate_table_jump_to_state_alternative<M>(
            factor: &mut Factory<M, Self>,
            condition: Condition,
            then_state: Self::StackBufferState,
            else_state: Self::StackBufferState,
        ) where
            M: MemoryModelOnArchSupportTrait<Arch = Self>,
        {
            let ten_table = factor.enter_points[then_state];
            let else_table = factor.enter_points[else_state];
            factor.arch_model.generate_table_jump_alternative(
                &mut factor.assembler,
                condition,
                ten_table,
                else_table,
            );
        }
    }

    pub struct Factory<M, A>
    where
        M: MemoryModelOnArchSupportTrait<Arch = A>,
        A: ArchitectureSupportTrait,
    {
        pub assembler: <A as ArchitectureSupportTrait>::Assembler,
        pub enter_points: <A as ArchitectureSupportTrait>::EnterPoints,
        pub memory_model: M,
        pub arch_model: A,
        pub null_exception_entry: Option<<A as ArchitectureSupportTrait>::EnterPoints>,
        pub array_index_out_of_bounds_exception_exception_entry:
            Option<<A as ArchitectureSupportTrait>::EnterPoints>,
        pub array_store_exception_exception_entry:
            Option<<A as ArchitectureSupportTrait>::EnterPoints>,
        pub negative_array_size_exception_entry:
            Option<<A as ArchitectureSupportTrait>::EnterPoints>,
        pub illegal_monitor_state_exception_entry:
            Option<<A as ArchitectureSupportTrait>::EnterPoints>,
    }
    impl<M, A> Factory<M, A>
    where
        M: MemoryModelOnArchSupportTrait<Arch = A>,
        A: ArchitectureSupportTrait,
    {
        pub fn generate_into(&mut self) -> Result<<A as ArchitectureSupportTrait>::Assembler> {
            M::generate_null_exception_entry(self)?;
            M::generate_array_index_out_of_bounds_exception_exception_entry(self)?;
            M::generate_array_store_exception_exception_entry(self)?;
            M::generate_negative_array_size_exception_entry(self)?;
            M::generate_illegal_monitor_state_exception_entry(self)?;
            M::generate_into(self)?;
            A::generate_into(self)?;
            todo!()
        }

        fn new(
            mut assembler: <A as ArchitectureSupportTrait>::Assembler,
            arch_model: A,
            memory_model: M,
        ) -> Result<Self> {
            Ok(Self {
                memory_model,
                arch_model,
                enter_points:
                    <A as ArchitectureSupportTrait>::EnterPoints::new_byte_code_deployment_table(
                        &mut assembler,
                    )?,

                null_exception_entry: None,
                array_index_out_of_bounds_exception_exception_entry: None,
                array_store_exception_exception_entry: None,
                negative_array_size_exception_entry: None,
                illegal_monitor_state_exception_entry: None,
                assembler,
            })
        }
    }
}
