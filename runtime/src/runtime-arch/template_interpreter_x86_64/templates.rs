use crate::memory::Label;
use arch::assembler::*;

use ImmediateKind::*;

use super::*;
const M1: i32 = -1;
pub fn declare_template() {
    macro_rules! declare_template {
        ($name:tt,$immediate_kind:expr,$stack_pop:expr,$stack_push:expr,$generate:expr) => {
            let _dcmpg: UniversalTemplate =
                UniversalTemplate::new($immediate_kind, $stack_pop, $stack_push, $generate);
        };
    }
    macro_rules! declare_template_with_enter_point {
        ($name:tt,$immediate_kind:expr,$stack_pop:expr,$stack_push:expr,$generate:expr) => {
            let _d2l: UniversalTemplate = UniversalTemplate::with_enter_point(
                $immediate_kind,
                $stack_pop,
                $stack_push,
                $generate,
            );
        };
    }
    macro_rules! declare_template_multiply_immediate {
        ($name:tt,$immediate_kind:expr,$stack_pop:expr,$stack_push:expr,$generate:expr) => {
            let _iinc: UniversalTemplate = UniversalTemplate::new_multiply_immediate(
                $immediate_kind,
                $stack_pop,
                $stack_push,
                $generate,
            );
        };
    }
    declare_template!(NOP, Void, 0, 0, move |_asm, _imme, _pop, _push| {});
    // declare_template!(ACONST_NULL, Void, 0, 1, runtime_memory::aconst_null);
    declare_template!(iconst_m1, Void, 0, 1, move |asm, _imme, _pop, push| {
        iconst::<M1>(asm, push[0]);
    });
    declare_template!(iconst_0, Void, 0, 1, move |asm, _imme, _pop, push| {
        iconst::<0>(asm, push[0]);
    });
    declare_template!(iconst_1, Void, 0, 1, move |asm, _imme, _pop, push| {
        iconst::<1>(asm, push[0]);
    });
    declare_template!(iconst_2, Void, 0, 1, move |asm, _imme, _pop, push| {
        iconst::<2>(asm, push[0]);
    });
    declare_template!(iconst_3, Void, 0, 1, move |asm, _imme, _pop, push| {
        iconst::<3>(asm, push[0]);
    });
    declare_template!(iconst_4, Void, 0, 1, move |asm, _imme, _pop, push| {
        iconst::<4>(asm, push[0]);
    });
    declare_template!(iconst_5, Void, 0, 1, move |asm, _imme, _pop, push| {
        iconst::<5>(asm, push[0]);
    });
    declare_template!(lconst_0, Void, 0, 1, move |asm, _imme, _pop, push| {
        iconst::<0>(asm, push[0]);
    });
    declare_template!(lconst_1, Void, 0, 1, move |asm, _imme, _pop, push| {
        iconst::<1>(asm, push[0]);
    });
    declare_template!(fconst_0, Void, 0, 1, move |asm, _imme, _pop, push| {
        fconst0(asm, push[0]);
    });
    declare_template!(fconst_1, Void, 0, 1, move |asm, _imme, _pop, push| {
        fconst::<1.0>(asm, push[0]);
    });
    declare_template!(fconst_2, Void, 0, 1, move |asm, _imme, _pop, push| {
        fconst::<2.0>(asm, push[0]);
    });
    declare_template!(dconst_0, Void, 0, 1, move |asm, _imme, _pop, push| {
        dconst0(asm, push[0]);
    });
    declare_template!(dconst_1, Void, 0, 1, move |asm, _imme, _pop, push| {
        dconst1(asm, push[0]);
    });
    declare_template!(bipush, I8, 0, 1, move |asm, imme, _, push| {
        bipush(asm, imme, push[0]);
    });
    declare_template!(sipush, I16, 0, 1, move |asm, imme, _, push| {
        sipush(asm, imme, push[0]);
    });

    macro_rules! declare_template_panic {
        ($name:tt,$immediate_kind:expr,$stack_pop:expr,$stack_push:expr) => {
            declare_template!(
                $name,
                $immediate_kind,
                $stack_pop,
                $stack_push,
                |a, _, _, _| { call_panic(a) }
            );
        };
    }
    declare_template_panic!(ldc, I8, 0, 1);
    declare_template_panic!(ldc_w, I16, 0, 1);
    declare_template_panic!(ldc2_w, I16, 0, 1);

    declare_template!(iload, U8, 0, 1, move |asm, imme, _, push| {
        MOV.r32_from_m(asm, push[0], local_variable_address(imme));
    });
    declare_template!(lload, U8, 0, 1, move |asm, imme, _, push| {
        MOV.r64_from_m(asm, push[0], local_variable_address(imme));
    });
    declare_template!(fload, U8, 0, 1, move |asm, imme, _, push| {
        MOV.r32_from_m(asm, push[0], local_variable_address(imme));
    });
    declare_template!(dload, U8, 0, 1, move |asm, imme, _, push| {
        MOV.r64_from_m(asm, push[0], local_variable_address(imme));
    });

    // declare_template!(aload, U8, 0, 1, runtime_memory::aload);

    macro_rules! declare_load_32_const_template {
        ($name:ident,$index:expr) => {
            declare_template!($name, Void, 0, 1, move |asm, _, _, push| {
                MOV.r32_from_m(asm, push[0], local_variable_address_const($index));
            });
        };
    }
    macro_rules! declare_load_64_const_template {
        ($name:ident,$index:expr) => {
            declare_template!($name, Void, 0, 1, move |asm, _, _, push| {
                MOV.r64_from_m(asm, push[0], local_variable_address_const($index));
            });
        };
    }
    declare_load_32_const_template!(iload_0, 1);
    declare_load_32_const_template!(iload_1, 1);
    declare_load_32_const_template!(iload_2, 2);
    declare_load_32_const_template!(iload_3, 3);
    declare_load_64_const_template!(lload_0, 1);
    declare_load_64_const_template!(lload_1, 1);
    declare_load_64_const_template!(lload_2, 2);
    declare_load_64_const_template!(lload_3, 3);
    declare_load_32_const_template!(fload_0, 1);
    declare_load_32_const_template!(fload_1, 1);
    declare_load_32_const_template!(fload_2, 2);
    declare_load_32_const_template!(fload_3, 3);
    declare_load_64_const_template!(dload_0, 1);
    declare_load_64_const_template!(dload_1, 1);
    declare_load_64_const_template!(dload_2, 2);
    declare_load_64_const_template!(dload_3, 3);

    // declare_template!(aload_0, Void, 0, 1, runtime_memory::aload_const::<0>);
    // declare_template!(aload_1, Void, 0, 1, runtime_memory::aload_const::<1>);
    // declare_template!(aload_2, Void, 0, 1, runtime_memory::aload_const::<2>);
    // declare_template!(aload_3, Void, 0, 1, runtime_memory::aload_const::<3>);

    // declare_template!(iaload, Void, 2, 1, runtime_memory::iaload);
    // declare_template!(laload, Void, 2, 1, runtime_memory::laload);
    // declare_template!(faload, Void, 2, 1, runtime_memory::faload);
    // declare_template!(daload, Void, 2, 1, runtime_memory::daload);
    // declare_template!(aaload, Void, 2, 1, runtime_memory::aaload);
    // declare_template!(baload, Void, 2, 1, runtime_memory::baload);
    // declare_template!(caload, Void, 2, 1, runtime_memory::caload);
    // declare_template!(saload, Void, 2, 1, runtime_memory::saload);

    declare_template!(istore, U8, 0, 1, move |asm, imme, _, push| {
        MOV.m_from_r32(asm, local_variable_address(imme), push[0]);
    });
    declare_template!(lstore, U8, 0, 1, move |asm, imme, _, push| {
        MOV.m_from_r64(asm, local_variable_address(imme), push[0]);
    });
    declare_template!(fstore, U8, 0, 1, move |asm, imme, _, push| {
        MOV.m_from_r32(asm, local_variable_address(imme), push[0]);
    });
    declare_template!(dstore, U8, 0, 1, move |asm, imme, _, push| {
        MOV.m_from_r64(asm, local_variable_address(imme), push[0]);
    });

    // declare_template!(astore, U8, 0, 1, runtime_memory::astore);

    macro_rules! declare_store_const_template {
        ($name:ident,$index:expr) => {
            declare_template!($name, Void, 0, 1, move |asm, _, _, push| {
                MOV.r32_from_m(asm, push[0], local_variable_address_const($index));
            });
        };
    }
    declare_store_const_template!(istore_0, 1);
    declare_store_const_template!(istore_1, 1);
    declare_store_const_template!(istore_2, 2);
    declare_store_const_template!(istore_3, 3);
    declare_store_const_template!(lstore_0, 1);
    declare_store_const_template!(lstore_1, 1);
    declare_store_const_template!(lstore_2, 2);
    declare_store_const_template!(lstore_3, 3);
    declare_store_const_template!(fstore_0, 1);
    declare_store_const_template!(fstore_1, 1);
    declare_store_const_template!(fstore_2, 2);
    declare_store_const_template!(fstore_3, 3);
    declare_store_const_template!(dstore_0, 1);
    declare_store_const_template!(dstore_1, 1);
    declare_store_const_template!(dstore_2, 2);
    declare_store_const_template!(dstore_3, 3);

    // declare_template!(astore_0, Void, 0, 1, runtime_memory::astore_const::<0>);
    // declare_template!(astore_1, Void, 0, 1, runtime_memory::astore_const::<1>);
    // declare_template!(astore_2, Void, 0, 1, runtime_memory::astore_const::<2>);
    // declare_template!(astore_3, Void, 0, 1, runtime_memory::astore_const::<3>);

    // declare_template!(iastore, Void, 2, 1, runtime_memory::iastore);
    // declare_template!(lastore, Void, 2, 1, runtime_memory::lastore);
    // declare_template!(fastore, Void, 2, 1, runtime_memory::fastore);
    // declare_template!(dastore, Void, 2, 1, runtime_memory::dastore);
    // declare_template!(aastore, Void, 2, 1, runtime_memory::aastore);
    // declare_template!(bastore, Void, 2, 1, runtime_memory::bastore);
    // declare_template!(castore, Void, 2, 1, runtime_memory::castore);
    // declare_template!(sastore, Void, 2, 1, runtime_memory::sastore);

    declare_template!(pop, Void, 1, 0, move |_asm, _imme, _pop, _push| {});

    // 栈为 ...type1,type1
    //  => pop2 转换为 pop,pop
    // 栈为 ...type2
    //  => pop2 转换为 pop
    declare_template_panic!(pop2, Void, 2, 0);

    declare_template!(dup, Void, 1, 2, dup);
    declare_template!(dup_x1, Void, 2, 3, dup_x1);

    // 栈为 ...type1,type2
    // => dup_x2 转换为 dup_x1
    declare_template!(dup_x2, Void, 3, 4, dup_x2);

    // 栈为 ...type2
    //  => dup 转换为 sup,dup
    declare_template!(dup2, Void, 2, 4, dup2);

    // 栈为 ...type1,type2
    // => dup2_x1 转换为 dup_x1
    declare_template!(dup2_x1, Void, 3, 5, dup2_x1);

    // 栈为 ...type2,type2
    // => dup2_x2 转换为 dup_x1
    // 栈为 ...type2,type1,type1
    // => dup2_x2 转换为 dup2_x1
    // 栈为 ...type2,type1,type2
    // => dup2_x2 转换为 dup_x2
    declare_template!(dup2_x2, Void, 3, 5, dup2_x2);

    declare_template!(swap, Void, 3, 5, swap);

    declare_template!(iadd, Void, 2, 1, iadd);
    declare_template!(ladd, Void, 2, 1, ladd);
    declare_template!(fadd, Void, 2, 1, fadd);
    declare_template!(dadd, Void, 2, 1, dadd);

    declare_template!(isub, Void, 2, 1, isub);
    declare_template!(lsub, Void, 2, 1, lsub);
    declare_template!(fsub, Void, 2, 1, fsub);
    declare_template!(dsub, Void, 2, 1, dsub);

    declare_template!(imul, Void, 2, 1, imul);
    declare_template!(lmul, Void, 2, 1, lmul);
    declare_template!(fmul, Void, 2, 1, fmul);
    declare_template!(dmul, Void, 2, 1, dmul);

    declare_template!(idiv, Void, 2, 1, idiv);
    declare_template!(ldiv, Void, 2, 1, ldiv);
    declare_template!(fdiv, Void, 2, 1, fdiv);
    declare_template!(ddiv, Void, 2, 1, ddiv);

    declare_template!(irem, Void, 2, 1, irem);
    declare_template!(lrem, Void, 2, 1, lrem);
    declare_template!(frem, Void, 2, 1, frem);
    declare_template!(drem, Void, 2, 1, drem);

    declare_template!(ineg, Void, 2, 1, ineg);
    declare_template!(lneg, Void, 2, 1, lneg);
    declare_template!(fneg, Void, 2, 1, fneg);
    declare_template!(dneg, Void, 2, 1, dneg);

    declare_template!(ishl, Void, 2, 1, ishl);
    declare_template!(lshl, Void, 2, 1, lshl);

    declare_template!(ishr, Void, 2, 1, ishr);
    declare_template!(lshr, Void, 2, 1, lshr);

    declare_template!(iushr, Void, 2, 1, iushr);
    declare_template!(lushr, Void, 2, 1, lushr);

    declare_template!(iand, Void, 2, 1, iand);
    declare_template!(land, Void, 2, 1, land);

    declare_template!(ior, Void, 2, 1, ior);
    declare_template!(lor, Void, 2, 1, lor);

    declare_template!(ixor, Void, 2, 1, ixor);
    declare_template!(lxor, Void, 2, 1, lxor);

    declare_template_multiply_immediate!(iinc, vec![U8, I8], 0, 0, iinc);

    declare_template!(i2l, Void, 1, 1, i2l);
    declare_template!(i2f, Void, 1, 1, i2f);
    declare_template!(i2d, Void, 1, 1, i2d);
    declare_template!(l2i, Void, 1, 1, l2i);
    declare_template!(l2f, Void, 1, 1, l2f);
    declare_template!(l2d, Void, 1, 1, l2d);

    declare_template_with_enter_point!(f2i, Void, 1, 1, f2i);
    declare_template_with_enter_point!(f2l, Void, 1, 1, f2l);

    declare_template!(f2d, Void, 1, 1, f2d);

    declare_template_with_enter_point!(d2i, Void, 1, 1, d2i);
    declare_template_with_enter_point!(d2l, Void, 1, 1, d2l);

    declare_template!(d2f, Void, 1, 1, d2f);

    declare_template!(i2b, Void, 1, 1, i2b);
    declare_template!(i2c, Void, 1, 1, i2c);
    declare_template!(i2s, Void, 1, 1, i2s);

    declare_template!(lcmp, Void, 2, 1, lcmp);

    declare_template!(fcmpl, Void, 2, 1, fcmpl);
    declare_template!(fcmpg, Void, 2, 1, fcmpg);
    declare_template!(dcmpl, Void, 2, 1, dcmpl);
    declare_template!(dcmpg, Void, 2, 1, dcmpg);

    todo!(); // TODO
}
pub fn local_variable_address_const(index: u16) -> AddressOperand {
    AddressOperand::Relative(
        LOCAL_VARIABLE_PREVIOUS,
        LOCAL_VARIABLE_SIZE + (index as i32) * TYPE_1_LOCAL_VARIABLE_SIZE,
    )
}
pub fn local_variable_address(index: Register) -> AddressOperand {
    AddressOperand::BaseAndIndexAndOffset {
        base: LOCAL_VARIABLE_PREVIOUS,
        index,
        scala: TYPE_1_SCALA,
        offset: LOCAL_VARIABLE_SIZE,
    }
}
pub fn nop(_asm: &mut Assembler) {}
pub fn iconst<const value: i32>(asm: &mut Assembler, dst: Register) {
    match value {
        0 => {
            XOR.r64_from_r(asm, dst, dst);
        }
        _ => {
            MOV.r32_from_i32(asm, dst, value);
 m_from_i32       }
    }
}
pub fn bipush(asm: &mut Assembler, imme: Register, dst: Register) {
    MOVSX_8.r32_from_r(asm, dst, imme);
}
pub fn sipush(asm: &mut Assembler, imme: Register, dst: Register) {
    MOVSX_16.r32_from_r(asm, dst, imme);
}
pub fn ldc_i32(asm: &mut Assembler, value: i32, dst: Register) {
    MOV.r32_from_i32(asm, dst, value);
}
pub fn ldc_i64(asm: &mut Assembler, value: i64, dst: Register) {
    movabs(asm, dst, value)
}
pub fn fconst0(asm: &mut Assembler, dst: Register) {
    XOR.r64_from_r(asm, dst, dst);
}
pub fn fconst<const VALUE: f32>(asm: &mut Assembler, dst: Register) {
    MOV.r32_from_i32(asm, dst, i32::from_le_bytes(VALUE.to_le_bytes()));
}
pub fn dconst0(asm: &mut Assembler, dst: Register) {
    XOR.r64_from_r(asm, dst, dst);
}
pub fn dconst1(asm: &mut Assembler, dst: Register) {
    movabs(asm, dst, i64::from_le_bytes(1f64.to_le_bytes()));
}

fn dup(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    MOV.r64_from_r(asm, push[1], pop[0]);
}
fn dup_x1(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    MOV.r64_from_r(asm, push[2], pop[1]);
    MOV.r64_from_r(asm, push[1], pop[0]);
    MOV.r64_from_r(asm, push[0], push[2]);
}
fn dup_x2(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    MOV.r64_from_r(asm, push[3], pop[2]);
    MOV.r64_from_r(asm, push[2], pop[1]);
    MOV.r64_from_r(asm, push[1], pop[0]);
    MOV.r64_from_r(asm, pop[0], push[3]);
}
fn dup2(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    MOV.r64_from_r(asm, push[3], pop[1]);
    MOV.r64_from_r(asm, push[2], pop[0]);
}
fn dup2_x1(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    MOV.r64_from_r(asm, push[4], pop[2]);
    MOV.r64_from_r(asm, push[3], pop[1]);
    MOV.r64_from_r(asm, push[2], pop[0]);
    MOV.r64_from_r(asm, push[1], push[4]);
    MOV.r64_from_r(asm, push[0], push[3]);
}
fn dup2_x2(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    MOV.r64_from_r(asm, push[5], pop[3]);
    MOV.r64_from_r(asm, push[4], pop[2]);
    MOV.r64_from_r(asm, push[3], pop[1]);
    MOV.r64_from_r(asm, push[2], pop[0]);
    MOV.r64_from_r(asm, push[1], push[5]);
    MOV.r64_from_r(asm, push[0], push[4]);
}

fn swap(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    MOV.r64_from_r(asm, TEMP_REG_1, pop[0]);
    MOV.r64_from_r(asm, push[0], pop[1]);
    MOV.r64_from_r(asm, push[1], TEMP_REG_1);
}

fn iadd(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    ADD.r32_from_r(asm, push[0], pop[1]);
}
fn ladd(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    ADD.r64_from_r(asm, push[0], pop[1]);
}
fn fadd(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    MOVD.xmm_from_r32(asm, XMM0, pop[0]);
    MOVD.xmm_from_r32(asm, XMM1, pop[1]);
    ADDSS.xmm_from_xmm(asm, XMM0, XMM1);
    MOVD.r32_from_xmm(asm, push[0], XMM0);
}
fn dadd(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    MOVQ.xmm_from_r32(asm, XMM0, pop[0]);
    MOVQ.xmm_from_r32(asm, XMM1, pop[1]);
    ADDSD.xmm_from_xmm(asm, XMM0, XMM1);
    MOVQ.r32_from_xmm(asm, push[0], XMM0);
}
fn isub(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    SUB.r32_from_r(asm, push[0], pop[1]);
}
fn lsub(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    SUB.r64_from_r(asm, push[0], pop[1]);
}
fn fsub(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    MOVD.xmm_from_r32(asm, XMM0, pop[0]);
    MOVD.xmm_from_r32(asm, XMM1, pop[1]);
    SUBSS.xmm_from_xmm(asm, XMM0, XMM1);
    MOVD.r32_from_xmm(asm, push[0], XMM0);
}
fn dsub(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    MOVQ.xmm_from_r32(asm, XMM0, pop[0]);
    MOVQ.xmm_from_r32(asm, XMM1, pop[1]);
    SUBSD.xmm_from_xmm(asm, XMM0, XMM1);
    MOVQ.r32_from_xmm(asm, push[0], XMM0);
}
fn imul(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    IMUL.r32_from_r(asm, push[0], pop[1]);
}
fn lmul(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    IMUL.r64_from_r(asm, push[0], pop[1]);
}
fn fmul(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    MOVD.xmm_from_r32(asm, XMM0, pop[0]);
    MOVD.xmm_from_r32(asm, XMM1, pop[1]);
    MULSS.xmm_from_xmm(asm, XMM0, XMM1);
    MOVD.r32_from_xmm(asm, push[0], XMM0);
}
fn dmul(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    MOVQ.xmm_from_r32(asm, XMM0, pop[0]);
    MOVQ.xmm_from_r32(asm, XMM1, pop[1]);
    MULSD.xmm_from_xmm(asm, XMM0, XMM1);
    MOVQ.r32_from_xmm(asm, push[0], XMM0);
}
fn idiv(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    if pop[0] != RAX {
        MOV.r32_from_r(asm, RAX, pop[0]);
    }
    CDQ.no_operand_32(asm);
    IDIV.r32(asm, pop[1]);
    if push[0] != RAX {
        MOV.r32_from_r(asm, push[1], RAX);
    }
}
fn ldiv(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    if pop[0] != RAX {
        MOV.r64_from_r(asm, RAX, pop[0]);
    }
    CDQ.no_operand_64(asm);
    IDIV.r64(asm, pop[1]);
    if push[0] != RAX {
        MOV.r64_from_r(asm, push[1], RAX);
    }
}
fn fdiv(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    MOVD.xmm_from_r32(asm, XMM0, pop[0]);
    MOVD.xmm_from_r32(asm, XMM1, pop[1]);
    DIVSS.xmm_from_xmm(asm, XMM0, XMM1);
    MOVD.r32_from_xmm(asm, push[0], XMM0);
}
fn ddiv(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    MOVQ.xmm_from_r32(asm, XMM0, pop[0]);
    MOVQ.xmm_from_r32(asm, XMM1, pop[1]);
    DIVSD.xmm_from_xmm(asm, XMM0, XMM1);
    MOVQ.r32_from_xmm(asm, push[0], XMM0);
}
fn irem(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    if pop[0] != RAX {
        MOV.r32_from_r(asm, RAX, pop[0]);
    }
    CDQ.no_operand_32(asm);
    IDIV.r32(asm, pop[1]);
    if push[0] != RDX {
        MOV.r32_from_r(asm, push[1], RDX);
    }
}
fn lrem(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    if pop[0] != RAX {
        MOV.r64_from_r(asm, RAX, pop[0]);
    }
    CDQ.no_operand_64(asm);
    IDIV.r64(asm, pop[1]);
    if push[0] != RDX {
        MOV.r64_from_r(asm, push[1], RDX);
    }
}
fn frem(_asm: &mut Assembler, _imme: Register, _pop: &[Register], _push: &[Register]) {
    todo!() // TODO
}
fn drem(_asm: &mut Assembler, _imme: Register, _pop: &[Register], _push: &[Register]) {
    todo!() // TODO
}
fn ineg(asm: &mut Assembler, _imme: Register, pop: &[Register], _push: &[Register]) {
    NEG.r32(asm, pop[0]);
}
fn lneg(asm: &mut Assembler, _imme: Register, pop: &[Register], _push: &[Register]) {
    NEG.r64(asm, pop[0]);
}
fn fneg(asm: &mut Assembler, _imme: Register, pop: &[Register], _push: &[Register]) {
    XOR.r32_from_i32(asm, pop[0], i32::MIN);
}
fn dneg(asm: &mut Assembler, _imme: Register, pop: &[Register], _push: &[Register]) {
    movabs(asm, TEMP_REG_1, i64::MIN);
    XOR.r64_from_r(asm, pop[0], TEMP_REG_1);
}
fn ishl(asm: &mut Assembler, _imme: Register, pop: &[Register], _push: &[Register]) {
    MOV.r32_from_r(asm, RCX, pop[1]);
    AND.r32_from_i8(asm, RCX, 31);
    SHL.r32(asm, pop[0]);
}
fn lshl(asm: &mut Assembler, _imme: Register, pop: &[Register], _push: &[Register]) {
    MOV.r32_from_r(asm, RCX, pop[1]);
    AND.r32_from_i8(asm, RCX, 31);
    SHL.r64(asm, pop[0]);
}

fn ishr(asm: &mut Assembler, _imme: Register, pop: &[Register], _push: &[Register]) {
    MOV.r32_from_r(asm, RCX, pop[1]);
    AND.r32_from_i8(asm, RCX, 31);
    SAR.r32(asm, pop[0]);
}
fn lshr(asm: &mut Assembler, _imme: Register, pop: &[Register], _push: &[Register]) {
    MOV.r32_from_r(asm, RCX, pop[1]);
    AND.r32_from_i8(asm, RCX, 31);
    SAR.r64(asm, pop[0]);
}

fn iushr(asm: &mut Assembler, _imme: Register, pop: &[Register], _push: &[Register]) {
    MOV.r32_from_r(asm, RCX, pop[1]);
    AND.r32_from_i8(asm, RCX, 31);
    SHR.r32(asm, pop[0]);
}
fn lushr(asm: &mut Assembler, _imme: Register, pop: &[Register], _push: &[Register]) {
    MOV.r32_from_r(asm, RCX, pop[1]);
    AND.r32_from_i8(asm, RCX, 31);
    SHR.r64(asm, pop[0]);
}

fn iand(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    AND.r32_from_r(asm, push[0], pop[1]);
}
fn land(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    AND.r64_from_r(asm, push[0], pop[1]);
}

fn ior(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    OR.r32_from_r(asm, push[0], pop[1]);
}
fn lor(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    OR.r64_from_r(asm, push[0], pop[1]);
}

fn ixor(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    XOR.r32_from_r(asm, push[0], pop[1]);
}
fn lxor(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    XOR.r64_from_r(asm, push[0], pop[1]);
}
fn iinc(asm: &mut Assembler, immediates: &[Register], _pop: &[Register], _push: &[Register]) {
    ADD.m_from_r32(asm, local_variable_address(immediates[0]), immediates[1]);
}
fn i2l(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    movsxd_r64_from_r32(asm, push[0], pop[0]);
}
fn i2f(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    CVTSI2SS.xmm_from_r32(asm, XMM0, pop[0]);
    MOVD.r32_from_xmm(asm, push[0], XMM0);
}
fn i2d(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    CVTSI2SD.xmm_from_r32(asm, XMM0, pop[0]);
    MOVQ.r64_from_xmm(asm, push[0], XMM0);
}
fn l2i(_asm: &mut Assembler, _imme: Register, _pop: &[Register], _push: &[Register]) {
    // nothing to do
}
fn l2f(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    CVTSI2SS.xmm_from_r32(asm, XMM0, pop[0]);
    MOVQ.r64_from_xmm(asm, push[0], XMM0);
}
fn l2d(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    CVTSI2SD.xmm_from_r64(asm, XMM0, pop[0]);
    MOVQ.r64_from_xmm(asm, push[0], XMM0);
}
fn f2i(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) -> Label {
    let input = pop[0];
    let output = push[0];
    MOVD.xmm_from_r32(asm, XMM0, input);
    let max = asm.relatively_label();
    asm.push_u32(0x4effffff);
    let min = asm.relatively_label();
    asm.push_u32(0xcf000000);
    let enter_point = asm.relatively_label();
    UCOMISS.xmm_from_m32(asm, XMM0, RipRelative(max));
    movaps_xmm_from_xmm(asm, XMM1, XMM0);
    MAXSS.xmm_from_m32(asm, XMM1, RipRelative(min));
    CVTTSS2SI.r32_from_xmm(asm, TEMP_REG_1, XMM0);
    CMOVBE.r32_from_r(asm, TEMP_REG_2, TEMP_REG_1);
    XOR.r32_from_r(asm, output, output);
    UCOMISS.xmm_from_xmm(asm, XMM0, XMM0);
    CMOVNP.r32_from_r(asm, output, TEMP_REG_2);
    enter_point
}
fn f2l(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) -> Label {
    let input = pop[0];
    let output = push[0];
    MOVD.xmm_from_r32(asm, XMM0, input);
    let max = asm.relatively_label();
    asm.push_u64(0x5effffff);
    let min = asm.relatively_label();
    asm.push_u64(0xdf000000);
    let enter_point = asm.relatively_label();
    UCOMISS.xmm_from_m64(asm, XMM0, RipRelative(max));
    movaps_xmm_from_xmm(asm, XMM1, XMM0);
    MAXSS.xmm_from_m64(asm, XMM1, RipRelative(min));
    CVTTSS2SI.r64_from_xmm(asm, TEMP_REG_1, XMM0);
    movabs(asm, TEMP_REG_2, 0x7fff_ffff_ffff_ffff);
    CMOVBE.r64_from_r(asm, TEMP_REG_2, TEMP_REG_1);
    XOR.r64_from_r(asm, output, output);
    UCOMISS.xmm_from_xmm(asm, XMM0, XMM0);
    CMOVNP.r64_from_r(asm, output, TEMP_REG_2);
    enter_point
}
fn f2d(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    MOVD.xmm_from_r32(asm, XMM0, pop[0]);
    CVTSS2SD.xmm_from_xmm(asm, XMM0, XMM0);
    MOVQ.r64_from_xmm(asm, push[0], XMM0);
}
fn d2i(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) -> Label {
    let input = pop[0];
    let output = push[0];
    MOVQ.xmm_from_r64(asm, XMM0, input);
    let max = asm.relatively_label();
    asm.push_u64(0x41dfffffffc00000);
    let min = asm.relatively_label();
    asm.push_u64(0xc1e0000000000000);
    let enter_point = asm.relatively_label();
    UCOMISD.xmm_from_m32(asm, XMM0, RipRelative(max));
    movaps_xmm_from_xmm(asm, XMM1, XMM0);
    MAXSD.xmm_from_m32(asm, XMM1, RipRelative(min));
    CVTTSD2SI.r32_from_xmm(asm, TEMP_REG_1, XMM0);
    MOV.r32_from_i32(asm, TEMP_REG_2, 0x7fffffff);
    CMOVBE.r32_from_r(asm, TEMP_REG_2, TEMP_REG_1);
    XOR.r32_from_r(asm, output, output);
    UCOMISD.xmm_from_xmm(asm, XMM0, XMM0);
    CMOVNP.r32_from_r(asm, output, TEMP_REG_2);
    enter_point
}
fn d2l(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) -> Label {
    let input = pop[0];
    let output = push[0];
    MOVQ.xmm_from_r64(asm, XMM0, input);
    let max = asm.relatively_label();
    asm.push_u64(0x43dfffffffffffff);
    let min = asm.relatively_label();
    asm.push_u64(0xc3e0000000000000);
    let enter_point = asm.relatively_label();
    UCOMISD.xmm_from_m64(asm, XMM0, RipRelative(max));
    movaps_xmm_from_xmm(asm, XMM1, XMM0);
    MAXSD.xmm_from_m64(asm, XMM1, RipRelative(min));
    CVTTSD2SI.r64_from_xmm(asm, TEMP_REG_1, XMM0);
    movabs(asm, TEMP_REG_2, 0x7fff_ffff_ffff_ffff);
    CMOVBE.r64_from_r(asm, TEMP_REG_2, TEMP_REG_1);
    XOR.r64_from_r(asm, output, output);
    UCOMISD.xmm_from_xmm(asm, XMM0, XMM0);
    CMOVNP.r64_from_r(asm, output, TEMP_REG_2);
    enter_point
}
fn d2f(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    MOVQ.xmm_from_r64(asm, XMM0, pop[0]);
    CVTSD2SS.xmm_from_xmm(asm, XMM0, XMM0);
    MOVD.r32_from_xmm(asm, push[0], XMM0);
}
fn i2b(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    MOVSX_8.r32_from_r(asm, push[0], pop[0]);
}
fn i2c(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    MOVZX_16.r32_from_r(asm, push[0], pop[0]);
}
fn i2s(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    MOVSX_16.r32_from_r(asm, push[0], pop[0]);
}
fn lcmp(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    let output = push[0];
    XOR.r32_from_r(asm, RCX, RCX);
    CMP.r64_from_r(asm, pop[0], pop[1]);
    set_condition(asm, GREATER, RCX);
    MOV.r32_from_i8(asm, output, -1);
    mov_conditional_r64_from_r(asm, GREATEREQUAL, output, RCX);
}
fn fcmpl(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    let output = push[0];
    MOV.xmm_from_r32(asm, XMM0, pop[0]);
    MOV.xmm_from_r32(asm, XMM1, pop[1]);
    UCOMISS.xmm_from_xmm(asm, XMM1, XMM0);
    MOV.r32_from_i32(asm, output, -1);
    let parity = jump_conditional_short_from(asm, PARITY);
    let below = jump_conditional_short_from(asm, BELOW);
    set_condition(asm, NOTEQUAL, RCX);
    MOVZX_8.r32_from_r(asm, output, RCX);
    parity.bind(asm);
    below.bind(asm);
}
fn fcmpg(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    let output = push[0];
    MOV.xmm_from_r32(asm, XMM0, pop[0]);
    MOV.xmm_from_r32(asm, XMM1, pop[1]);
    UCOMISS.xmm_from_xmm(asm, XMM1, XMM0);
    MOV.r32_from_i32(asm, output, 1);
    let parity = jump_conditional_short_from(asm, PARITY);
    let above = jump_conditional_short_from(asm, ABOVE);
    set_condition(asm, NOTEQUAL, RCX);
    MOVZX_8.r32_from_r(asm, output, RCX);
    NEG.r32(asm, output);
    parity.bind(asm);
    above.bind(asm);
}

fn dcmpl(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    let output = push[0];
    MOV.xmm_from_r64(asm, XMM0, pop[0]);
    MOV.xmm_from_r64(asm, XMM1, pop[1]);
    UCOMISD.xmm_from_xmm(asm, XMM1, XMM0);
    MOV.r32_from_i32(asm, output, -1);
    let parity = jump_conditional_short_from(asm, PARITY);
    let below = jump_conditional_short_from(asm, BELOW);
    set_condition(asm, NOTEQUAL, RCX);
    MOVZX_8.r32_from_r(asm, output, RCX);
    parity.bind(asm);
    below.bind(asm);
}
fn dcmpg(asm: &mut Assembler, _imme: Register, pop: &[Register], push: &[Register]) {
    let output = push[0];
    MOV.xmm_from_r64(asm, XMM0, pop[0]);
    MOV.xmm_from_r64(asm, XMM1, pop[1]);
    UCOMISD.xmm_from_xmm(asm, XMM1, XMM0);
    MOV.r32_from_i32(asm, output, 1);
    let parity = jump_conditional_short_from(asm, PARITY);
    let above = jump_conditional_short_from(asm, ABOVE);
    set_condition(asm, NOTEQUAL, RCX);
    MOVZX_8.r32_from_r(asm, output, RCX);
    NEG.r32(asm, output);
    parity.bind(asm);
    above.bind(asm);
}
fn jump_const(asm: &mut Assembler, offset_i32: Register) {
    SUB.r32_from_r(asm, IP, offset_i32);
}
const IF_CONDITION_SIZE: i8 = 3;
fn jump_condition(asm: &mut Assembler, condition: Condition, offset_i32: Register) {
    MOV.r64_from_r(asm, TEMP_REG_1, IP);
    MOV.r64_from_i8(asm, IP, IF_CONDITION_SIZE);
    mov_conditional_r64_from_r(asm, condition, IP, offset_i32);
    ADD.r64_from_r(asm, IP, TEMP_REG_1);
}
fn ifeq(asm: &mut Assembler, imme: Register, pop: &[Register], _push: &[Register]) {
    TEST.r32_from_r(asm, pop[0], pop[0]);
    jump_condition(asm, EQUAL, imme);
}
fn ifne(asm: &mut Assembler, imme: Register, pop: &[Register], _push: &[Register]) {
    TEST.r32_from_r(asm, pop[0], pop[0]);
    jump_condition(asm, NOTEQUAL, imme);
}
fn iflt(asm: &mut Assembler, imme: Register, pop: &[Register], _push: &[Register]) {
    TEST.r32_from_r(asm, pop[0], pop[0]);
    jump_condition(asm, LESS, imme);
}
fn ifge(asm: &mut Assembler, imme: Register, pop: &[Register], _push: &[Register]) {
    TEST.r32_from_r(asm, pop[0], pop[0]);
    jump_condition(asm, GREATEREQUAL, imme);
}
fn ifgt(asm: &mut Assembler, imme: Register, pop: &[Register], _push: &[Register]) {
    TEST.r32_from_r(asm, pop[0], pop[0]);
    jump_condition(asm, GREATER, imme);
}
fn ifle(asm: &mut Assembler, imme: Register, pop: &[Register], _push: &[Register]) {
    TEST.r32_from_r(asm, pop[0], pop[0]);
    jump_condition(asm, LESSEQUAL, imme);
}
fn if_icmpeq(asm: &mut Assembler, imme: Register, pop: &[Register], _push: &[Register]) {
    CMP.r32_from_r(asm, pop[0], pop[1]);
    jump_condition(asm, EQUAL, imme);
}
fn if_icmpne(asm: &mut Assembler, imme: Register, pop: &[Register], _push: &[Register]) {
    CMP.r32_from_r(asm, pop[0], pop[1]);
    jump_condition(asm, NOTEQUAL, imme);
}
fn if_icmplt(asm: &mut Assembler, imme: Register, pop: &[Register], _push: &[Register]) {
    CMP.r32_from_r(asm, pop[0], pop[1]);
    jump_condition(asm, LESS, imme);
}
fn if_icmpge(asm: &mut Assembler, imme: Register, pop: &[Register], _push: &[Register]) {
    CMP.r32_from_r(asm, pop[0], pop[1]);
    jump_condition(asm, GREATEREQUAL, imme);
}
fn if_icmpgt(asm: &mut Assembler, imme: Register, pop: &[Register], _push: &[Register]) {
    CMP.r32_from_r(asm, pop[0], pop[1]);
    jump_condition(asm, GREATER, imme);
}
fn if_icmple(asm: &mut Assembler, imme: Register, pop: &[Register], _push: &[Register]) {
    CMP.r32_from_r(asm, pop[0], pop[1]);
    jump_condition(asm, LESSEQUAL, imme);
}
fn goto(asm: &mut Assembler, imme: Register, _pop: &[Register], _push: &[Register]) {
    ADD.r64_from_r(asm, IP, imme);
}

// RAX:stack[-1]
// RCX
// RBX:
// RDX: mul,div
// RSP:&stack.top
// RBP:&frame
// RSI:IP:&ByteCode
// RDI:stack_buffer_size:i64
pub fn call_panic(asm: &mut Assembler) {
    call_c(asm, panic as *const fn() as *const u8);
}
pub fn call_c(_asm: &mut Assembler, _function: *const u8) {
    todo!(); // TODO
}
// pub fn call_Rust(asm: &mut Assembler,function: *const u8) {
//     todo!(); //TODO
// }
extern "C" fn panic() {
    panic!();
}
