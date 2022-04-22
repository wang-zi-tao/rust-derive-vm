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
const ARG0: Register = RDI;
const ARG1: Register = RSI;
const ARG2: Register = RDX;
const ARG3: Register = RCX;
const ARG4: Register = R8;
const ARG5: Register = R9;
const ARGS: [Register; 6] = [ARG0, ARG1, ARG2, ARG3, ARG4, ARG5];
const FARG0: XMMRegister = XMM0;
const FARG1: XMMRegister = XMM1;
const FARG2: XMMRegister = XMM2;
const FARG3: XMMRegister = XMM3;
const FARGS: [XMMRegister; 4] = [FARG0, FARG1, FARG2, FARG3];
const RETURN: Register = RAX;
const FUNCTION: Register = R15;
fn adapter(
    asm: &mut Assembler,
    stack_buffer_state: StackBufferState,
    params_is_float: &[bool],
    has_return: bool,
) -> Result<StackBufferState> {
    let parameter_count = params_is_float.len() as isize;
    let capacity = stack_buffer_state.capacity as isize;
    // -capacity .. isize::max(-6,-parameter_count)
    // ensure stored
    // for i in -capacity..isize::max(-6, -parameter_count) {
    // stack_buffer_state.ensure_empty(i);
    // }
    // pop
    for i in isize::max(-6, -parameter_count)..0 {
        if i >= -4 && params_is_float[(-1 - i) as usize] {
            stack_buffer_state.load_xmm(asm, i, FARGS[(-1 - i) as usize]);
        } else {
            stack_buffer_state.load(asm, i, ARGS[(-1 - i) as usize]);
        }
    }
    stack_buffer_state.push_dirty_data(asm,stack_pop,stack_push);
    CALL.r64(asm, FUNCTION);
    // push
    if has_return {
        MOV.r64_from_r(asm, stack_buffer_state.ensure_empty(0));
        stack_buffer_state.store(asm, RETURN, stack_push - stack_pop);
    }
    let new_state = self.transparent(asm, parameter_count, if has_return { 1 } else { 0 }, false);

    // load buffer
    stack_buffer_state.filling_buffer(asm,stack_pop,stack_push);
    ret(asm);
    Ok(new_state)
}
fn call_stub(
    factor: &mut Factory<M, ArchitectureSupport>,
    local_count: u16,
    byte_code_enter_point: *const (),
    params_is_float: &[bool],
    frame_buffer_capacity: u8,
    max_immediate_count: u8,
) -> Result<()> {
    let asm = &mut factor.assembler;
    MOV.r64_from_r(asm, LOCAL_VARIABLE_PREVIOUS, FRAME);
    ADD.r64_from_i32(
        asm,
        LOCAL_VARIABLE_PREVIOUS,
        -LOCAL_VARIABLE_SIZE - (local_count as i32),
    );
    MOV.r64_from_r(asm, STACK_TOP, LOCAL_VARIABLE_PREVIOUS);
    movabs(asm, IP, byte_code_enter_point as usize);
    for i in 0..6 {
        if i >= 4 && params_is_float[i] {
            MOV.m64_from_xmm(asm, local_variable_address(i), FARGS[i]);
        } else {
            MOV.m_from_r64(asm, local_variable_address(i), ARGS[i]);
        }
    }
    let frame_buffer_state = FrameBufferState::new(capacity, max_immediate_count);
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
