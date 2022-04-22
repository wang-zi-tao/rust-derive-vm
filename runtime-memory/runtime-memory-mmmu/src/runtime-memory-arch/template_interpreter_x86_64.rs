use arch::assembler::*;

use runtime::FrameDataType;
use runtime_arch::{
    template_interpreter::*,
    template_interpreter_arch::{templates::*, ArchitectureSupport, StackBufferState},
};
use util::Result;
pub struct MemoryModelOnArchSupport {}
impl Default for MemoryModelOnArchSupport {
    fn default() -> Self {
        todo!()
    }
}
impl MemoryModelOnArchSupportTrait for MemoryModelOnArchSupport {
    type Arch = ArchitectureSupport;
    type StackBufferState = StackBufferState;

    fn generate_into(_factor: &mut Factory<Self, Self::Arch>) -> Result<()> {
        todo!()
    }

    fn generate_null_exception_entry(_factor: &mut Factory<Self, Self::Arch>) -> Result<()> {
        todo!()
    }

    fn generate_array_index_out_of_bounds_exception_exception_entry(
        _factor: &mut Factory<Self, Self::Arch>,
    ) -> Result<()> {
        todo!()
    }

    fn generate_array_store_exception_exception_entry(
        _factor: &mut Factory<Self, Self::Arch>,
    ) -> Result<()> {
        todo!()
    }

    fn generate_negative_array_size_exception_entry(
        _factor: &mut Factory<Self, Self::Arch>,
    ) -> Result<()> {
        todo!()
    }

    fn generate_illegal_monitor_state_exception_entry(
        _factor: &mut Factory<Self, Self::Arch>,
    ) -> Result<()> {
        todo!()
    }
}
pub fn aconst_null(asm: &mut Assembler, _imme: Register, _pop: &[Register], push: &[Register]) {
    XOR.r64_from_r(asm, push[0], push[0]);
}
pub fn aload(asm: &mut Assembler, imme: Register, _pop: &[Register], push: &[Register]) {
    MOV.r64_from_m(asm, push[0], local_variable_address(imme));
}
pub fn aload_const<const INDEX: u16>(
    asm: &mut Assembler,
    _imme: Register,
    _pop: &[Register],
    push: &[Register],
) {
    MOV.r64_from_m(asm, push[0], local_variable_address_const(INDEX));
}
fn verify_non_null(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    oop: Register,
) {
    let asm = &mut factor.assembler;
    TEST.r64_from_r(asm, oop, oop);
    jump_conditional_to(
        asm,
        ZERO,
        &factor.null_exception_entry.as_ref().unwrap()[stack_state],
    );
}
fn verify_index(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    array: Register,
    index: Register,
) {
    let asm = &mut factor.assembler;
    CMP.m_from_r32(asm, AddressOperand::Indirect(array), index);
    jump_conditional_to(
        asm,
        BELOWEQUAL,
        &factor
            .array_index_out_of_bounds_exception_exception_entry
            .as_ref()
            .unwrap()[stack_state],
    );
}
fn init_class(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    class_info_offset: Register,
    offset_output: Register,
) {
    todo!(); // TODO
}
fn load(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    _stack_state: StackBufferState,
    dst: Register,
    src: AddressOperand,
    data_type: FrameDataType,
) {
    let asm = &mut factor.assembler;
    match data_type {
        FrameDataType::Byte => MOVSX_8.r32_from_m(asm, dst, src),
        FrameDataType::Char => MOVZX_16.r32_from_m(asm, dst, src),
        FrameDataType::Short => MOVSX_16.r32_from_m(asm, dst, src),
        FrameDataType::Int => MOV.r32_from_m(asm, dst, src),
        FrameDataType::Long => MOV.r64_from_m(asm, dst, src),
        FrameDataType::Reference => {
            todo!();
        }
        _ => panic!(),
    }
}
fn load_xmm(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    _stack_state: StackBufferState,
    dst: XMMRegister,
    src: AddressOperand,
    data_type: FrameDataType,
) {
    let asm = &mut factor.assembler;
    match data_type {
        FrameDataType::Float => MOVD.xmm_from_m32(asm, dst, src),
        FrameDataType::Double => MOVQ.xmm_from_m64(asm, dst, src),
        _ => panic!(),
    }
}
fn store(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    _stack_state: StackBufferState,
    dst: AddressOperand,
    src: Register,
    data_type: FrameDataType,
) {
    let asm = &mut factor.assembler;
    match data_type {
        FrameDataType::Byte => {
            MOV.m_from_r8(asm, dst, src);
        }
        FrameDataType::Char | FrameDataType::Short => {
            mov_m_from_r16(asm, dst, src);
        }
        FrameDataType::Int => {
            MOV.m32_from_r(asm, dst, src);
        }
        FrameDataType::Long => {
            MOV.m64_from_r(asm, dst, src);
        }
        FrameDataType::Reference => todo!(),
        _ => panic!(),
    }
}
fn store_xmm(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    _stack_state: StackBufferState,
    dst: AddressOperand,
    src: XMMRegister,
    data_type: FrameDataType,
) {
    let asm = &mut factor.assembler;
    match data_type {
        FrameDataType::Float => MOVD.m32_from_xmm(asm, dst, src),
        FrameDataType::Double => MOVQ.m64_from_xmm(asm, dst, src),
        _ => panic!(),
    }
}
pub fn iaload(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    _imme: Register,
    pop: &[Register],
    push: &[Register],
) {
    verify_index(factor, stack_state, pop[0], pop[1]);
    load(
        factor,
        stack_state,
        push[0],
        AddressOperand::BaseAndIndexAndOffset {
            base: pop[0],
            index: pop[1],
            scala: Scala::Scala8,
            offset: 8,
        },
        FrameDataType::Reference,
    );
}
pub fn laload(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    _imme: Register,
    pop: &[Register],
    push: &[Register],
) {
    verify_index(factor, stack_state, pop[0], pop[1]);
    load(
        factor,
        stack_state,
        push[0],
        AddressOperand::BaseAndIndexAndOffset {
            base: pop[0],
            index: pop[1],
            scala: Scala::Scala8,
            offset: 8,
        },
        FrameDataType::Long,
    );
}
pub fn faload(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    _imme: Register,
    pop: &[Register],
    _push: &[Register],
) {
    verify_index(factor, stack_state, pop[0], pop[1]);
    load_xmm(
        factor,
        stack_state,
        XMM0,
        AddressOperand::BaseAndIndexAndOffset {
            base: pop[0],
            index: pop[1],
            scala: Scala::Scala8,
            offset: 8,
        },
        FrameDataType::Float,
    );
}
pub fn daload(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    _imme: Register,
    pop: &[Register],
    _push: &[Register],
) {
    verify_index(factor, stack_state, pop[0], pop[1]);
    load_xmm(
        factor,
        stack_state,
        XMM0,
        AddressOperand::BaseAndIndexAndOffset {
            base: pop[0],
            index: pop[1],
            scala: Scala::Scala8,
            offset: 8,
        },
        FrameDataType::Double,
    );
}
pub fn aaload(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    _imme: Register,
    pop: &[Register],
    push: &[Register],
) {
    verify_index(factor, stack_state, pop[0], pop[1]);
    load(
        factor,
        stack_state,
        push[0],
        AddressOperand::BaseAndIndexAndOffset {
            base: pop[0],
            index: pop[1],
            scala: Scala::Scala8,
            offset: 8,
        },
        FrameDataType::Reference,
    );
}
pub fn baload(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    _imme: Register,
    pop: &[Register],
    push: &[Register],
) {
    verify_index(factor, stack_state, pop[0], pop[1]);
    load(
        factor,
        stack_state,
        push[0],
        AddressOperand::BaseAndIndexAndOffset {
            base: pop[0],
            index: pop[1],
            scala: Scala::Scala8,
            offset: 8,
        },
        FrameDataType::Byte,
    );
}
pub fn caload(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    _imme: Register,
    pop: &[Register],
    push: &[Register],
) {
    verify_index(factor, stack_state, pop[0], pop[1]);
    load(
        factor,
        stack_state,
        push[0],
        AddressOperand::BaseAndIndexAndOffset {
            base: pop[0],
            index: pop[1],
            scala: Scala::Scala8,
            offset: 8,
        },
        FrameDataType::Char,
    );
}
pub fn saload(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    _imme: Register,
    pop: &[Register],
    push: &[Register],
) {
    verify_index(factor, stack_state, pop[0], pop[1]);
    load(
        factor,
        stack_state,
        push[0],
        AddressOperand::BaseAndIndexAndOffset {
            base: pop[0],
            index: pop[1],
            scala: Scala::Scala8,
            offset: 8,
        },
        FrameDataType::Short,
    );
}

pub fn astore(
    _factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    _stack_state: StackBufferState,
    _imme: Register,
    _pop: &[Register],
    _push: &[Register],
) {
    todo!(); // TODO
}
pub fn astore_const<const INDEX: i32>(
    _asm: &mut Assembler,
    _imme: Register,
    _pop: &[Register],
    _push: &[Register],
) {
    todo!(); // TODO
}

pub fn iastore(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    _imme: Register,
    pop: &[Register],
    push: &[Register],
) {
    verify_index(factor, stack_state, pop[0], pop[1]);
    store(
        factor,
        stack_state,
        AddressOperand::BaseAndIndexAndOffset {
            base: pop[0],
            index: pop[1],
            scala: Scala::Scala8,
            offset: 8,
        },
        push[0],
        FrameDataType::Reference,
    );
}
pub fn lastore(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    _imme: Register,
    pop: &[Register],
    push: &[Register],
) {
    verify_index(factor, stack_state, pop[0], pop[1]);
    store(
        factor,
        stack_state,
        AddressOperand::BaseAndIndexAndOffset {
            base: pop[0],
            index: pop[1],
            scala: Scala::Scala8,
            offset: 8,
        },
        push[0],
        FrameDataType::Long,
    );
}
pub fn fastore(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    _imme: Register,
    pop: &[Register],
    _push: &[Register],
) {
    verify_index(factor, stack_state, pop[0], pop[1]);
    store_xmm(
        factor,
        stack_state,
        AddressOperand::BaseAndIndexAndOffset {
            base: pop[0],
            index: pop[1],
            scala: Scala::Scala8,
            offset: 8,
        },
        XMM0,
        FrameDataType::Float,
    );
}
pub fn dastore(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    _imme: Register,
    pop: &[Register],
    _push: &[Register],
) {
    verify_index(factor, stack_state, pop[0], pop[1]);
    store_xmm(
        factor,
        stack_state,
        AddressOperand::BaseAndIndexAndOffset {
            base: pop[0],
            index: pop[1],
            scala: Scala::Scala8,
            offset: 8,
        },
        XMM0,
        FrameDataType::Double,
    );
}
pub fn aastore(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    _imme: Register,
    pop: &[Register],
    push: &[Register],
) {
    verify_index(factor, stack_state, pop[0], pop[1]);
    store(
        factor,
        stack_state,
        AddressOperand::BaseAndIndexAndOffset {
            base: pop[0],
            index: pop[1],
            scala: Scala::Scala8,
            offset: 8,
        },
        push[0],
        FrameDataType::Reference,
    );
}
pub fn bastore(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    _imme: Register,
    pop: &[Register],
    push: &[Register],
) {
    verify_index(factor, stack_state, pop[0], pop[1]);
    store(
        factor,
        stack_state,
        AddressOperand::BaseAndIndexAndOffset {
            base: pop[0],
            index: pop[1],
            scala: Scala::Scala8,
            offset: 8,
        },
        push[0],
        FrameDataType::Byte,
    );
}
pub fn castore(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    _imme: Register,
    pop: &[Register],
    push: &[Register],
) {
    verify_index(factor, stack_state, pop[0], pop[1]);
    store(
        factor,
        stack_state,
        AddressOperand::BaseAndIndexAndOffset {
            base: pop[0],
            index: pop[1],
            scala: Scala::Scala8,
            offset: 8,
        },
        push[0],
        FrameDataType::Char,
    );
}
pub fn sastore(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    _imme: Register,
    pop: &[Register],
    push: &[Register],
) {
    verify_index(factor, stack_state, pop[0], pop[1]);
    todo!(); // TODO
    store(
        factor,
        stack_state,
        AddressOperand::BaseAndIndexAndOffset {
            base: pop[0],
            index: pop[1],
            scala: Scala::Scala8,
            offset: 8,
        },
        push[0],
        FrameDataType::Short,
    );
}
pub fn push_32(asm: &mut Assembler, imme: Register, _pop: &[Register], push: &[Register]) {
    MOV.r32_from_r(asm, push[0], imme);
}
pub fn push_64(asm: &mut Assembler, imme: Register, _pop: &[Register], push: &[Register]) {
    MOV.r64_from_r(asm, push[0], imme);
}
pub fn apply_invoke_adapter(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    adapter: Register,
    enter_point: Register,
) {
    if enter_point != R15 {
        MOV.r64_from_r(&mut factor.assembler, R15, enter_point);
    }
    CALL.r64(&mut factor.assembler, R15);
}
pub fn invoke_special_direct(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    _imme: &[Register],
    _pop: &[Register],
    _push: &[Register],
) {
    apply_invoke_adapter(factor, stack_state, pop[0], pop[1]);
}
pub fn metadata_get_address(
    asm: &mut Assembler,
    oop_mut: Register,
    offset: Register,
    output: Register,
) {
    shl_r64_from_i8(asm, oop_mut, HEAP_PAGE_SIZE.trailing_zeros() - 3);
    MOV.r64_from_address(asm, oop_mut, AddressOperand::Direct(oop_mut));
    MOV.r64_from_address(
        asm,
        oop_mut,
        AddressOperand::BaseAndIndex {
            base: oop_mut,
            index: offset,
            scala: Scala1,
        },
    );
}
pub fn invoke_from_virtual_table(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    _imme: Register,
    pop: &[Register],
    _push: &[Register],
) {
    stack_state.load(&mut factor.assembler, pop[2], R15);
    metadata_get_address(factor, R15, pop[1], R15);
    apply_invoke_adapter(factor, stack_state, pop[0], R15);
}
pub fn jvm_call(_asm: &mut Assembler, _imme: Register, _pop: &[Register], _push: &[Register]) {
    todo!(); // TODO
}
pub fn invoke_static_direct(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    _imme: Register,
    _pop: &[Register],
    _push: &[Register],
) {
    apply_invoke_adapter(factor, stack_state, pop[0], pop[1]);
}
pub fn invoke_c_direct(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    _imme: Register,
    _pop: &[Register],
    _push: &[Register],
) {
    todo!(); // TODO
}
pub fn new(_asm: &mut Assembler, _imme: Register, _pop: &[Register], _push: &[Register]) {
    todo!(); // TODO
}
pub fn new_array(_asm: &mut Assembler, _imme: Register, _pop: &[Register], _push: &[Register]) {
    todo!(); // TODO
}
pub fn add_const_32(asm: &mut Assembler, imme: Register, pop: &[Register], _push: &[Register]) {
    ADD.r64_from_r(asm, pop[0], imme);
}
pub fn add_const_64(asm: &mut Assembler, _imme: Register, _pop: &[Register], _push: &[Register]) {
    ADD.r64_from_r(asm, pop[0], imme);
}
pub fn get_field_8(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    imme: Register,
    pop: &[Register],
    push: &[Register],
) {
    verify_non_null(factor, stack_state, pop[0]);
    get_field_unchecked_8(&mut factor.assembler, imme, pop, push)
}
pub fn get_field_16(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    imme: Register,
    pop: &[Register],
    push: &[Register],
) {
    verify_non_null(factor, stack_state, pop[0]);
    get_field_unchecked_16(&mut factor.assembler, imme, pop, push)
}
pub fn get_field_32(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    imme: Register,
    pop: &[Register],
    push: &[Register],
) {
    verify_non_null(factor, stack_state, pop[0]);
    get_field_unchecked_32(&mut factor.assembler, imme, pop, push)
}
pub fn get_field_64(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    imme: Register,
    pop: &[Register],
    push: &[Register],
) {
    verify_non_null(factor, stack_state, pop[0]);
    get_field_unchecked_64(&mut factor.assembler, imme, pop, push)
}
pub fn get_field_unchecked_8(
    asm: &mut Assembler,
    imme: Register,
    pop: &[Register],
    push: &[Register],
) {
    MOVSX_8.r32_from_m(
        asm,
        push[0],
        AddressOperand::BaseAndIndex {
            base: pop[0],
            index: imme,
            scala: Scala1,
        },
    );
}
pub fn get_field_unchecked_16(
    asm: &mut Assembler,
    imme: Register,
    pop: &[Register],
    push: &[Register],
) {
    MOVSX_16.r32_from_m(
        asm,
        push[0],
        AddressOperand::BaseAndIndex {
            base: pop[0],
            index: imme,
            scala: Scala1,
        },
    );
}
pub fn get_field_unchecked_32(
    asm: &mut Assembler,
    imme: Register,
    pop: &[Register],
    push: &[Register],
) {
    MOV.r32_from_m(
        asm,
        push[0],
        AddressOperand::BaseAndIndex {
            base: pop[0],
            index: imme,
            scala: Scala1,
        },
    );
}
pub fn get_field_unchecked_64(
    asm: &mut Assembler,
    imme: Register,
    pop: &[Register],
    push: &[Register],
) {
    MOV.r64_from_m(
        asm,
        push[0],
        AddressOperand::BaseAndIndex {
            base: pop[0],
            index: imme,
            scala: Scala1,
        },
    );
}

pub fn get_static_8(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    imme: Register,
    pop: &[Register],
    push: &[Register],
) {
    init_class(factor, stack_state, imme, TEMP_REG_1);
    get_static_unchecked_8(factor, TEMP_REG_1, pop, push);
}
pub fn get_static_16(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    _imme: Register,
    _pop: &[Register],
    _push: &[Register],
) {
    init_class(factor, stack_state, imme, TEMP_REG_1);
    get_static_unchecked_16(factor, TEMP_REG_1, pop, push);
}
pub fn get_static_32(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    _imme: Register,
    _pop: &[Register],
    _push: &[Register],
) {
    init_class(factor, stack_state, imme, TEMP_REG_1);
    get_static_unchecked_32(factor, TEMP_REG_1, pop, push);
}
pub fn get_static_64(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    _imme: Register,
    _pop: &[Register],
    _push: &[Register],
) {
    init_class(factor, stack_state, imme, TEMP_REG_1);
    get_static_unchecked_64(factor, TEMP_REG_1, pop, push);
}
pub fn get_static_unchecked_8(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    imme: Register,
    _pop: &[Register],
    push: &[Register],
) {
    MOVSX_8.r32_from_m(
        &mut factor.assembler,
        push[0],
        AddressOperand::Indirect(imme),
    );
}
pub fn get_static_unchecked_16(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    imme: Register,
    _pop: &[Register],
    push: &[Register],
) {
    MOVSX_16.r32_from_m(
        &mut factor.assembler,
        push[0],
        AddressOperand::Indirect(imme),
    );
}
pub fn get_static_unchecked_32(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    imme: Register,
    _pop: &[Register],
    push: &[Register],
) {
    MOV.r32_from_m(
        &mut factor.assembler,
        push[0],
        AddressOperand::Indirect(imme),
    );
}
pub fn get_static_unchecked_64(
    factor: &mut Factory<MemoryModelOnArchSupport, ArchitectureSupport>,
    stack_state: StackBufferState,
    imme: Register,
    _pop: &[Register],
    push: &[Register],
) {
    MOV.r64_from_m(
        &mut factor.assembler,
        push[0],
        AddressOperand::Indirect(imme),
    );
}

pub fn call_java(asm: &mut Assembler, enter_point: Register, adapter: Register) {
    todo!() // TODO
}
