use std::{convert::TryInto, ops::Index};

use crate::{
    memory::{AssociateStubPoolBuilderTrait, Label},
    template_interpreter::{
        ArchitectureSupportTrait, EnterPointsTrait, Factory, MemoryModelOnArchSupportTrait,
        StackBufferStateTrait, UniversalTemplateTrait,
    },
};
use arch::{
    assembler::{Assembler, Register, *},
    X86_64,
};
use runtime::bytecode::ImmediateKind;
use util::Result;
pub mod deployment;
pub mod functions;
pub mod templates;
pub const STACK_TOP_BUFFER: Register = RAX;
pub const FRAME: Register = RBP;
pub const STACK_TOP: Register = RSP;
pub const IP: Register = RSI;
pub const LOCAL_VARIABLE_PREVIOUS: Register = RDI;
// const FRAME_VARIABLE_OFFSET: i32 = 16;
pub const TEMP_REG_1: Register = RCX;
pub const TEMP_REG_2: Register = RDX;
pub const TEMP_REG_3: Register = RBX;
// const DEFAULT_ARGUMENT0_REG: Register = RDX;
pub const LOCAL_VARIABLE_SIZE: i32 = 8;
pub const TYPE_1_LOCAL_VARIABLE_SIZE: i32 = 8;
pub const TYPE_2_LOCAL_VARIABLE_SIZE: i32 = 8;
pub const TYPE_1_SCALA: Scala = Scala::Scala8;
pub const TYPE_2_SCALA: Scala = Scala::Scala8;
pub const STACK_BUFFER_REG: [Register; 5] = [R15, R14, R13, R12, R10];
/// 加载立即数
pub fn load_immediate(
    asm: &mut Assembler,
    offset: i8,
    dst: Register,
    immediate_kind: ImmediateKind,
) {
    match immediate_kind {
        ImmediateKind::Void => {}
        ImmediateKind::U8 => {
            MOVZX_8.r32_from_m(asm, dst, AddressOperand::Relative(IP, offset as i32))
        }
        ImmediateKind::U16 => {
            MOVZX_16.r32_from_m(asm, dst, AddressOperand::Relative(IP, offset as i32))
        }
        ImmediateKind::U32 => MOV.r32_from_m(asm, dst, AddressOperand::Relative(IP, offset as i32)),
        ImmediateKind::U64 => MOV.r64_from_m(asm, dst, AddressOperand::Relative(IP, offset as i32)),
        ImmediateKind::I8 => {
            MOVSX_8.r32_from_m(asm, dst, AddressOperand::Relative(IP, offset as i32))
        }
        ImmediateKind::I16 => {
            MOVSX_16.r32_from_m(asm, dst, AddressOperand::Relative(IP, offset as i32))
        }
        ImmediateKind::I32 => MOV.r32_from_m(asm, dst, AddressOperand::Relative(IP, offset as i32)),
        ImmediateKind::I64 => MOV.r64_from_m(asm, dst, AddressOperand::Relative(IP, offset as i32)),
    }
}
pub fn cmp_state_top_and_local_variables_previous(asm: &mut Assembler) {
    CMP.r64_from_r(asm, STACK_TOP, LOCAL_VARIABLE_PREVIOUS);
}
/// 栈缓存类型
#[derive(Debug, Clone)]
pub enum StackCellBuffer {
    Register(Register),
    XMMRegister(XMMRegister),
}
impl StackCellBuffer {
    fn as_reg(&self) -> &Register {
        match self {
            StackCellBuffer::Register(r) => r,
            StackCellBuffer::XMMRegister(_) => panic!(),
        }
    }
}
/// 单条普通指令使用的部分栈的缓冲寄存器以及立即数寄存器
#[derive(Default)]
pub struct UsingStackBuffer {
    pop: Vec<StackCellBuffer>,
    push: Vec<StackCellBuffer>,
    immediate: Vec<StackCellBuffer>,
}
/// 栈缓冲寄存器状态
#[derive(Clone, Copy)]
pub struct StackBufferState {
    capacity: u8,
    sync_count: u8,
    buffer_end: u8,
    is_full: bool,
    top_is_float: bool,
    max_immediate_count: u8,
}
impl StackBufferState {
    fn new(capacity: u8, sync_count: u8, max_immediate_count: u8) -> Self {
        Self {
            capacity,
            sync_count,
            max_immediate_count,
            is_full: (capacity == 0) as bool,
            top_is_float: false,
            buffer_end: STACK_BUFFER_REG[0],
        }
    }

    /// 获取指定编号的缓冲寄存器
    fn get_reg(&self, index: i8) -> Register {
        STACK_BUFFER_REG[0..self.capacity][index as usize]
    }

    /// 获取相对栈顶的栈元素对应寄存器
    fn get_stack_reg(&self, index_base_on_top: i8) -> Register {
        self.get_reg(
            ((self.buffer_end as i32 + index_base_on_top as i32) % self.capacity as i32)
                .try_into()
                .unwrap(),
        )
    }

    /// 获取相对栈顶的栈元素对应内存寻址
    fn get_stack_address(&self, index_base_on_top: i8) -> AddressOperand {
        AddressOperand::Relative(STACK_TOP, -8 * index_base_on_top as i32)
    }

    fn load(&self, asm: &mut Assembler, dst: Register, index_base_on_top: i8) {
        if -self.capacity < index_base_on_top {
            if self.top_is_float {
                MOV.r64_from_xmm(asm, dst, XMM0);
            } else {
                let reg = self.get_stack_reg(index_base_on_top);
                if reg != dst {
                    MOV.r64_from_r(asm, dst, reg);
                }
            }
        } else {
            MOV.r64_from_m(asm, dst, self.get_stack_address(index_base_on_top));
        }
    }

    fn store(&self, asm: &mut Assembler, src: Register, index_base_on_top: i8) {
        if -self.capacity < index_base_on_top {
            let reg = self.get_stack_reg(index_base_on_top);
            if reg != dst {
                MOV.r64_from_r(asm, reg, src);
            }
            if index_base_on_top < -self.capacity + self.sync_count {
                MOV.m_from_r64(asm, self.get_stack_address(index_base_on_top), src);
            }
        } else {
            MOV.m_from_r64(asm, self.get_stack_address(index_base_on_top), src);
        }
    }

    fn push(&self, asm: &mut Assembler, reg: Register) {
        PUSH.r64(asm, reg);
    }

    fn push_xmm(&self, asm: &mut Assembler) {
        SUB.r64_from_i8(asm, IP, -LocalVariableSize);
        MOVQ.m64_from_xmm(asm, AddressOperand::Relative(IP, 8), XMM0);
    }

    /// 分配额外寄存器
    fn use_external_register(&self, index: &mut u8) -> Register {
        let reg = self.get_reg(8 + *index);
        *index += 1;
        reg
    }

    /// 加载指定立即数
    fn load_immediate(
        &self,
        asm: &mut Assembler,
        offset: &mut i8,
        index: &mut u8,
        immediate_kind: ImmediateKind,
    ) -> Register {
        let reg = self.use_external_register(index);
        load_immediate(asm, *offset, reg, immediate_kind);
        *offset += immediate_kind.size() as i8;
        reg
    }

    /// 更新状态
    fn transform(self, stack_pop: u8, stack_push: u8, top_is_float: bool) -> Self {
        ADD.r64_from_i32(
            asm,
            STACK_TOP,
            -(stack_push - stack_pop) * LOCAL_VARIABLE_SIZE,
        );
        Self {
            buffer_end: (self.buffer_end + stack_push - stack_pop) % self.capacity,
            is_full: self.is_full || (self.buffer_end + stack_push - stack_pop) >= self.capacity,
            top_is_float,
            ..self
        }
    }

    fn push_dirty_data(&self, asm: &mut Assembler, stack_pop: u8, stack_push: u8) {
        let buffer_usage = i8::max(stack_push, stack_pop);
        if capacity > buffer_usage && stack_push > stack_pop {
            if self.is_full {
                for i in -capacity + self.sync_count
                    ..-capacity + stack_push - stack_pop + self.sync_count
                {
                    self.load(&mut factor.assembler, i, self.get_stack_reg(i));
                }
            } else {
                if self.buffer_end + stack_push - stack_pop > capacity + self.sync_count {
                    for i in -self.buffer_end + self.sync_count
                        ..stack_push - stack_pop - capacity + self.sync_count
                    {
                        self.load(&mut factor.assembler, i, self.get_stack_reg(i));
                    }
                }
            }
        }
    }

    fn filling_buffer(&self, asm: &mut Assembler, stack_pop: u8, stack_push: u8) {
        if self.is_full && stack_push < capacity {
            for i in -capacity..-stack_push {
                new_state.load(&mut factor.assembler, i, self.get_stack_reg(i));
            }
        }
    }

    /// 生成应用模板的机器码
    fn apply_universal_template<M>(
        &self,
        factor: &mut Factory<M, ArchitectureSupport>,
        template: &UniversalTemplate,
    ) where
        M: MemoryModelOnArchSupportTrait<Arch = ArchitectureSupport>,
    {
        let asm = &mut factor.assembler;
        let stack_pop = template.stack_pop() as i8;
        let stack_push = template.stack_push() as i8;
        let capacity = self.capacity as i8;
        let mut next_extern_register_index: u8 = 0;
        let mut pop_list = Vec::with_capacity(stack_pop as usize);
        let buffer_usage = i8::max(stack_push, stack_pop);
        let buffer_regs = Vec::with_capacity(buffer_usage);
        for i in -stack_pop..buffer_usage - stack_pop {
            if i >= -stack_pop + stack_push - capacity && i < stack_push - stack_pop {
                let reg = self.get_stack_reg(i);
                buffer_regs.push(reg);
            } else {
                let reg = self.use_external_register(&mut next_extern_register_index);
                buffer_regs.push(reg);
            }
        }
        // pop
        if stack_pop > stack_push {
            if stack_pop >= 1 {
                if template.input_xmm() {
                    self.load_xmm(&mut factor.assembler, i, XMM0);
                } else {
                    self.load(&mut factor.assembler, i, buffer_regs[i + capacity]);
                }
            }
            for i in (-capacity..-1).iter().rev() {
                self.load(i, buffer_regs[i + capacity]);
            }
        } else {
            for i in -capacity..-1 {
                self.load(i, buffer_regs[i + capacity]);
            }
            if stack_pop >= 1 {
                if template.input_xmm() {
                    self.load_xmm(&mut factor.assembler, i, XMM0);
                } else {
                    self.load(&mut factor.assembler, i, buffer_regs[i + capacity]);
                }
            }
        }
        stack_buffer_state.push_dirty_data(asm, stack_pop, stack_push);
        // load immediate
        let immediate_kind = template.immediate_kind();
        let immediate_count = immediate_kind.len();
        assert!(immediate_count <= self.max_immediate_count as usize);
        let mut immediate_list = Vec::with_capacity(immediate_count);
        let mut immediate_offset = 1;
        for i in immediate_kind {
            immediate_list.push(StackCellBuffer::Register(self.load_immediate(
                asm,
                &mut immediate_offset,
                &mut next_extern_register_index,
                *i,
            )));
        }
        let using_stack_buffer = UsingStackBuffer {
            pop: buffer_regs[0..stack_pop],
            push: buffer_regs[0..stack_push],
            immediate: immediate_list,
        };
        template.generate_core(factor, *self, &using_stack_buffer);

        let new_state = self.transform(
            asm,
            stack_pop as u8,
            stack_push as u8,
            template.output_xmm(),
        );
        // push
        for i in 0..stack_push {
            new_state.store(&mut factor.assembler, buffer_regs[i], i - stack_push)
        }
        new_state.filling_buffer(asm, stack_pop, stack_push);

        // jump
        if !new_state.is_full && new_state.buffer_end == 0 && stack_pop > stack_push {
            let mut simplify_state = new_state;
            simplify_state.is_full = false;
            cmp_state_top_and_local_variables_previous(&mut factor.assembler);
            ArchitectureSupport::generate_table_jump_to_state_alternative(
                factor,
                EQUAL,
                simplify_state,
                new_state,
            );
        } else {
            ArchitectureSupport::generate_table_jump_to_state(factor, new_state);
        }
    }

    pub fn store_all(&self, asm: &mut Assembler) {
        let len = if self.is_full {
            self.buffer_end
        } else {
            self.capacity
        };
        for i in -len..-1 {
            self.push(asm, self.get_stack_reg(i));
        }
        if len > 0 {
            if self.top_is_float {
                self.push_xmm(asm);
            } else {
                self.push(asm, self.get_stack_reg(-1));
            }
        }
    }
}
impl StackBufferStateTrait for StackBufferState {
    type Arch = ArchitectureSupport;
    type UsingStackBuffer = UsingStackBuffer;
}
pub struct EnterPoints {
    full: Vec<Label>,
    partial: Vec<Label>,
}
impl EnterPointsTrait for EnterPoints {
    type Arch = ArchitectureSupport;
    type StackBufferState = StackBufferState;

    fn new_byte_code_deployment_table(_assembler: &mut Assembler) -> Result<Self> {
        todo!();
    }
}
impl Index<StackBufferState> for EnterPoints {
    type Output = Label;

    fn index(&self, _index: StackBufferState) -> &Self::Output {
        todo!()
    }
}

pub struct UniversalTemplate {
    immediate_kind: Vec<ImmediateKind>,
    stack_pop: u16,
    stack_push: u16,
    auto_change_ip: bool,
    input_xmm: bool,
    output_xmm: bool,
    generate: Box<
        dyn for<'a> Fn(&'a mut Assembler, &'a [Register], &'a [Register], &'a [Register]) -> Label
            + Sync,
    >,
}
impl UniversalTemplate {
    fn input_xmm(&self) -> bool {
        self.input_xmm
    }

    fn output_xmm(&self) -> bool {
        self.output_xmm
    }

    fn generate_for_state<M>(
        &self,
        _factor: &mut Factory<M, ArchitectureSupport>,
        _state: StackBufferState,
        _opcode: u8,
    ) where
        M: MemoryModelOnArchSupportTrait<Arch = ArchitectureSupport>,
    {
        todo!();
    }

    fn new<F>(immediate_kind: ImmediateKind, stack_pop: u16, stack_push: u16, generate: F) -> Self
    where
        F: for<'a> Fn(&'a mut Assembler, Register, &'a [Register], &'a [Register]) + Sync + 'static,
    {
        Self {
            immediate_kind: vec![immediate_kind],
            stack_pop,
            stack_push,
            auto_change_ip: true,
            input_xmm: false,
            output_xmm: false,
            generate: Box::new(move |asm, imme, pop, push| {
                let enter_point = asm.relatively_label();
                generate(asm, imme[0], pop, push);
                enter_point
            }),
        }
    }

    fn new_multiply_immediate<F>(
        immediate_kind: Vec<ImmediateKind>,
        stack_pop: u16,
        stack_push: u16,
        generate: F,
    ) -> UniversalTemplate
    where
        F: for<'a> Fn(&'a mut Assembler, &'a [Register], &'a [Register], &'a [Register])
            + Sync
            + 'static,
    {
        Self {
            immediate_kind,
            stack_pop,
            stack_push,
            auto_change_ip: true,
            input_xmm: false,
            output_xmm: false,
            generate: Box::new(move |asm, imme, pop, push| {
                let enter_point = asm.relatively_label();
                generate(asm, imme, pop, push);
                enter_point
            }),
        }
    }

    fn with_enter_point<F>(
        immediate_kind: ImmediateKind,
        stack_pop: u16,
        stack_push: u16,
        generate: F,
    ) -> UniversalTemplate
    where
        F: for<'a> Fn(&'a mut Assembler, Register, &'a [Register], &'a [Register]) -> Label
            + Sync
            + 'static,
    {
        Self {
            immediate_kind: vec![immediate_kind],
            stack_pop,
            stack_push,
            auto_change_ip: true,
            input_xmm: false,
            output_xmm: false,
            generate: Box::new(move |asm, imme, pop, push| generate(asm, imme[0], pop, push)),
        }
    }
}

impl UniversalTemplateTrait for UniversalTemplate {
    type Arch = ArchitectureSupport;
    type Assembler = Assembler;
    type StackBufferState = StackBufferState;
    type UsingStackBuffer = UsingStackBuffer;

    fn auto_change_ip(&self) -> bool {
        self.auto_change_ip
    }

    fn immediate_kind(&self) -> &Vec<ImmediateKind> {
        &self.immediate_kind
    }

    fn stack_pop(&self) -> u16 {
        self.stack_pop
    }

    fn stack_push(&self) -> u16 {
        self.stack_push
    }

    fn generate_core(
        &self,
        _assembler: &mut Self::Assembler,
        _stack_state: Self::StackBufferState,
    ) -> Label {
        todo!()
    }

    fn generate_for_state_and_deploy<M>(
        &self,
        _factor: &mut Factory<M, Self::Arch>,
        _state: Self::StackBufferState,
        _opcode: u8,
    ) where
        M: MemoryModelOnArchSupportTrait<Arch = Self::Arch>,
    {
        todo!()
    }
}
pub struct ArchitectureSupport {}
impl ArchitectureSupportTrait for ArchitectureSupport {
    type Arch = X86_64;
    type Assembler = Assembler;
    type EnterPoints = EnterPoints;
    type Reg = Register;
    type StackBufferState = StackBufferState;
    type UsingStackBuffer = UsingStackBuffer;

    fn generate_into<M: MemoryModelOnArchSupportTrait<Arch = Self>>(
        _factor: &mut Factory<M, Self>,
    ) -> util::Result<()> {
        todo!()
    }

    fn generate_change_ip(&self, assembler: &mut Self::Assembler, offset: i32) {
        ADD.r64_from_i32(assembler, IP, offset);
    }

    fn generate_table_jump(&self, assembler: &mut Self::Assembler, jump_table: Label) {
        lea_64(
            assembler,
            TEMP_REG_1,
            AddressOperand::RipRelative(jump_table),
        );
        MOVZX_8.r64_from_m(assembler, TEMP_REG_2, AddressOperand::Indirect(IP));
        jmp_relative_address(
            assembler,
            AddressOperand::BaseAndIndex {
                base: TEMP_REG_1,
                index: TEMP_REG_2,
                scala: Scala::Scala8,
            },
        );
    }

    fn generate_table_jump_alternative(
        &self,
        asm: &mut Self::Assembler,
        condition: Condition,
        then_jump_table: Label,
        else_jump_table: Label,
    ) {
        XOR.r64_from_r(asm, TEMP_REG_1, TEMP_REG_1);
        MOV.r32_from_i32(
            asm,
            TEMP_REG_3,
            (else_jump_table - then_jump_table).try_into().unwrap(),
        );
        mov_conditional_r64_from_r(asm, condition, TEMP_REG_1, TEMP_REG_3);
        lea_64(
            asm,
            TEMP_REG_3,
            AddressOperand::RipRelative(then_jump_table),
        );
        ADD.r64_from_r(asm, TEMP_REG_3, TEMP_REG_1);
        MOVZX_8.r64_from_m(asm, TEMP_REG_2, AddressOperand::Indirect(IP));
        jmp_relative_address(
            asm,
            AddressOperand::BaseAndIndex {
                base: TEMP_REG_1,
                index: TEMP_REG_2,
                scala: Scala::Scala8,
            },
        );
    }

    fn generate_get_immediate_to_reg(
        &self,
        assembler: &mut Self::Assembler,
        offset: i32,
        immediate_kind: &ImmediateKind,
        dst: &Self::Reg,
    ) {
        match immediate_kind {
            ImmediateKind::Void => panic!(),
            ImmediateKind::I8 => {
                MOVSX_8.r64_from_m(assembler, *dst, AddressOperand::Relative(IP, offset))
            }
            ImmediateKind::U8 => {
                MOVZX_8.r64_from_m(assembler, *dst, AddressOperand::Relative(IP, offset))
            }
            ImmediateKind::I16 => {
                MOVSX_16.r64_from_m(assembler, *dst, AddressOperand::Relative(IP, offset))
            }
            ImmediateKind::U16 => {
                MOVZX_16.r64_from_m(assembler, *dst, AddressOperand::Relative(IP, offset))
            }
            ImmediateKind::I32 => {
                MOV.r32_from_m(assembler, *dst, AddressOperand::Relative(IP, offset))
            }
            ImmediateKind::U32 => {
                MOV.r32_from_m(assembler, *dst, AddressOperand::Relative(IP, offset))
            }
            ImmediateKind::I64 => {
                MOV.r64_from_m(assembler, *dst, AddressOperand::Relative(IP, offset))
            }
            ImmediateKind::U64 => {
                MOV.r64_from_m(assembler, *dst, AddressOperand::Relative(IP, offset))
            }
        }
    }
}
