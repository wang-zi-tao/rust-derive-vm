use self::OpCode::*;
pub use super::{registers::*, xmm_registers::*};
pub use condition::*;
use jvm_core::ObjectBuilderInner;
use std::convert::TryFrom;
pub use AddressOperand::*;
pub use InstructionSet::*;
pub use Scala::Scala1;
type Label = usize;
pub fn label_offset(target: Label, current: Label) -> i64 {
    i64::try_from(target)
        .unwrap()
        .checked_sub(i64::try_from(current).unwrap())
        .unwrap()
}
pub struct JumpShortLabel(Label);
impl JumpShortLabel {
    pub fn bind(self, asm: &mut Assembler) {
        let current = asm.relatively_label();
        let offset = i8::try_from(label_offset(current, self.0) - 1).unwrap();
        asm.set_i8(self.0, offset);
    }
}
pub struct JumpNearLabel(Label);
impl JumpNearLabel {
    pub fn bind(self, asm: &mut Assembler) {
        let current = asm.relatively_label();
        let offset = i32::try_from(label_offset(current, self.0) - 4).unwrap();
        asm.set_i32(self.0, offset);
    }
}
pub mod condition {
    pub struct Condition(u8);
    impl Condition {
        pub fn encode(&self) -> u8 {
            self.0
        }
    }
    pub const ZERO: Condition = Condition(0x4);
    pub const NOTZERO: Condition = Condition(0x5);
    pub const EQUAL: Condition = Condition(0x4);
    pub const NOTEQUAL: Condition = Condition(0x5);
    pub const LESS: Condition = Condition(0xc);
    pub const LESSEQUAL: Condition = Condition(0xe);
    pub const GREATER: Condition = Condition(0xf);
    pub const GREATEREQUAL: Condition = Condition(0xd);
    pub const BELOW: Condition = Condition(0x2);
    pub const BELOWEQUAL: Condition = Condition(0x6);
    pub const ABOVE: Condition = Condition(0x7);
    pub const ABOVEEQUAL: Condition = Condition(0x3);
    pub const OVERFLOW: Condition = Condition(0x0);
    pub const NOOVERFLOW: Condition = Condition(0x1);
    pub const CARRYSET: Condition = Condition(0x2);
    pub const CARRYCLEAR: Condition = Condition(0x3);
    pub const NEGATIVE: Condition = Condition(0x8);
    pub const POSITIVE: Condition = Condition(0x9);
    pub const PARITY: Condition = Condition(0xa);
    pub const NOPARITY: Condition = Condition(0xb);
}
pub struct Assembler<'l, 'a: 'l> {
    builder: &'l mut ObjectBuilderInner<'a>,
}
impl<'l, 'a: 'l> Assembler<'l, 'a> {
    pub fn set_i8(&mut self, offset: usize, value: i8) {
        self.builder.set(offset, value)
    }

    pub fn set_i32(&mut self, offset: usize, value: i32) {
        self.builder.set(offset, value)
    }

    pub fn push_u8(&mut self, value: u8) {
        self.builder.push(value);
    }

    pub fn push_u32(&mut self, value: u32) {
        self.builder.push(value);
    }

    pub fn push_u64(&mut self, value: u64) {
        self.builder.push(value);
    }

    pub fn push_i8(&mut self, value: i8) {
        self.builder.push(value);
    }

    pub fn push_i32(&mut self, value: i32) {
        self.builder.push(value);
    }

    pub fn push_i64(&mut self, value: i64) {
        self.builder.push(value);
    }

    pub fn relatively_label(&self) -> Label {
        self.builder.len()
    }

    fn encode_sib(&mut self, scala: Scala, index: Register, base: Register) {
        let offset = self.builder.len() as u32;
        self.push_u8(
            scala.encode() << 6 | index.encode_shift(offset, 3) | base.encode_shift(offset, 0),
        );
    }

    fn encode_modrm_code(&mut self, mode: u8, reg: u8, rm: u8) {
        self.push_u8(mode << 6 | reg << 6 | rm);
    }

    fn encode_modrm(&mut self, mode: u8, reg: Register, rm: Register) {
        self.encode_modrm_code(mode, reg.encode(), rm.encode());
    }

    fn encode_modrm_xmm(&mut self, mode: u8, reg: XMMRegister, rm: XMMRegister) {
        self.encode_modrm_code(mode, reg.encode(), rm.encode());
    }

    pub fn rex_prefix_code(
        &mut self,
        byte: bool,
        reg_code_large_than_3: bool,
        w: bool,
        r: bool,
        x: bool,
        b: bool,
    ) {
        let mut prefix = if w { prefix::REX_W } else { prefix::REX };
        if r {
            prefix |= prefix::REX_R;
        }
        if x {
            prefix |= prefix::REX_X;
        }
        if b {
            prefix |= prefix::REX_B;
        }
        if prefix != prefix::REX || (byte && reg_code_large_than_3) {
            self.push_u8(prefix);
        }
    }

    pub fn rex_64_prefix(&mut self, rm: &Register, reg: &Register) {
        self.rex_prefix_code(
            false,
            false,
            true,
            reg.code_between_8_and_15(),
            false,
            rm.code_between_8_and_15(),
        );
    }

    pub fn rex_32_prefix(&mut self, rm: &Register, reg: &Register) {
        self.rex_prefix_code(
            false,
            false,
            false,
            reg.code_between_8_and_15(),
            false,
            rm.code_between_8_and_15(),
        );
    }

    pub fn rex_64_prefix_address(&mut self, reg: &Register, address: &AddressOperand) {
        address.rex_prefix(self, reg, true, false);
    }

    pub fn rex_64_prefix_reg(&mut self, reg: Register) {
        self.rex_prefix_code(
            false,
            false,
            true,
            false,
            false,
            reg.code_between_8_and_15(),
        );
    }

    pub fn rex_32_prefix_reg(&mut self, reg: Register) {
        self.rex_prefix_code(
            false,
            false,
            false,
            false,
            false,
            reg.code_between_8_and_15(),
        );
    }

    pub fn rex_32_prefix_address(&mut self, reg: &Register, address: &AddressOperand) {
        address.rex_prefix(self, reg, false, false);
    }

    pub fn rex_8_prefix_address(&mut self, reg: &Register, address: &AddressOperand) {
        address.rex_prefix(self, reg, false, true);
    }

    fn rex_8_prefix_reg(&mut self, reg: &Register) {
        self.rex_prefix_code(
            true,
            reg.code_between_4_and_15(),
            false,
            false,
            false,
            reg.code_between_8_and_15(),
        );
    }

    pub fn rex_32_prefix_xmm(&mut self, rm: &XMMRegister, reg: &XMMRegister) {
        self.rex_prefix_code(
            false,
            false,
            false,
            reg.code_between_8_and_15(),
            false,
            rm.code_between_8_and_15(),
        );
    }

    pub fn rex_64_prefix_xmm(&mut self, rm: &XMMRegister, reg: &XMMRegister) {
        self.rex_prefix_code(
            false,
            false,
            false,
            reg.code_between_8_and_15(),
            false,
            rm.code_between_8_and_15(),
        );
    }

    pub fn rex_64_prefix_xmm_address(&mut self, reg: &XMMRegister, address: &AddressOperand) {
        address.prefix_rex_xmm(self, reg, true, false);
    }

    pub fn rex_32_prefix_xmm_address(&mut self, reg: &XMMRegister, address: &AddressOperand) {
        address.prefix_rex_xmm(self, reg, false, false);
    }
}

pub enum AddressOperand {
    /// [bass]
    Indirect(Register),
    /// [base+offset]
    Relative(Register, i32),
    /// [scala*index]
    Index {
        index: Register,
        scala: Scala,
    },
    /// [scala*index+offset]
    IndexAndOffset {
        index: Register,
        scala: Scala,
        offset: i32,
    },
    /// [scala*index]
    BaseAndIndex {
        base: Register,
        index: Register,
        scala: Scala,
    },
    /// [scala*index+offset]
    BaseAndIndexAndOffset {
        base: Register,
        index: Register,
        scala: Scala,
        offset: i32,
    },
    /// [immediate]
    Direct(i32),
    RipRelative(Label),
}
impl AddressOperand {
    fn prefix_rex_code(
        &self,
        asm: &mut Assembler,
        reg_extend: bool,
        reg_code_large_than_3: bool,
        wide: bool,
        byte: bool,
    ) {
        let (mut b, mut x) = (false, false);
        match self {
            AddressOperand::Indirect(base) | AddressOperand::Relative(base, _) => {
                b = base.code_between_8_and_15()
            }
            AddressOperand::Index { index, scala: _ }
            | AddressOperand::IndexAndOffset {
                index,
                scala: _,
                offset: _,
            } => x = index.code_between_8_and_15(),
            AddressOperand::BaseAndIndex {
                base,
                index,
                scala: _,
            }
            | AddressOperand::BaseAndIndexAndOffset {
                base,
                index,
                scala: _,
                offset: _,
            } => {
                b = base.code_between_8_and_15();
                x = index.code_between_8_and_15();
            }
            AddressOperand::Direct(_) => {}
            AddressOperand::RipRelative { .. } => {}
        }
        asm.rex_prefix_code(byte, reg_code_large_than_3, wide, reg_extend, x, b);
    }

    fn rex_prefix(&self, asm: &mut Assembler, reg: &Register, wide: bool, byte: bool) {
        self.prefix_rex_code(
            asm,
            reg.code_between_8_and_15(),
            reg.code_between_4_and_15(),
            wide,
            byte,
        );
    }

    fn prefix_rex_xmm(&self, asm: &mut Assembler, reg: &XMMRegister, wide: bool, byte: bool) {
        self.prefix_rex_code(
            asm,
            reg.code_between_8_and_15(),
            reg.code_between_4_and_15(),
            wide,
            byte,
        );
    }

    fn encode_xmm(&self, reg: XMMRegister, asm: &mut Assembler) {
        self.encode(Register::new(reg.encode()), asm);
    }

    fn encode(&self, reg: Register, asm: &mut Assembler) {
        self.encode_code(reg.encode(), asm);
    }

    pub fn encode_code(&self, reg: u8, asm: &mut Assembler) {
        match self {
            AddressOperand::Indirect(base) => match base.encode() {
                0b100 => {
                    // [rsp] | [r12]
                    // [00 reg 100][00 100 100]
                    asm.encode_modrm_code(0, reg, 4);
                    asm.encode_sib(Scala1, RSP, RSP);
                }
                0b101 => {
                    // [RBP] | [R13]
                    // [01 rsg 101] 0i8
                    asm.encode_modrm_code(1, reg, 4);
                    asm.push_i8(0);
                }
                _ => {
                    // [base]
                    // [00 reg base]
                    asm.encode_modrm_code(0, reg, base.encode());
                }
            },
            AddressOperand::Relative(base, offset) => {
                match base.encode() {
                    0b100 => {
                        match (*offset) >> 7 {
                            0 | -1 => {
                                // [rsp|r12 + offset]
                                // [01 reg 100][00 100 100] offset_i8
                                asm.encode_modrm_code(1, reg, 4);
                                asm.encode_sib(Scala1, RSP, RSP);
                                asm.push_i8(*offset as i8);
                            }
                            _ => {
                                // [rsp|r12 + offset]
                                // [10 reg 100][00 100 100] offset_i32
                                asm.encode_modrm_code(2, reg, 4);
                                asm.encode_sib(Scala1, RSP, RSP);
                                asm.push_i32(*offset);
                            }
                        }
                    }
                    _ => {
                        match (*offset) >> 7 {
                            0 | -1 => {
                                // [base + offset]
                                // [01 reg base] offset_i8
                                asm.encode_modrm_code(1, reg, base.encode());
                                asm.push_i8(*offset as i8);
                            }
                            _ => {
                                // [base + offset]
                                // [10 reg base] offset_i32
                                asm.encode_modrm_code(10, reg, base.encode());
                                asm.push_i32(*offset);
                            }
                        }
                    }
                };
            }
            AddressOperand::Index { index, scala } => {
                // [index*scale + offset]
                // [00 reg 100][scala index 101] 0i32
                assert!(index != &RSP, "illegal addressing mode");
                asm.encode_modrm_code(0, reg, 4);
                asm.encode_sib(*scala, *index, RBP);
                asm.push_u32(0);
            }
            AddressOperand::IndexAndOffset {
                index,
                scala,
                offset,
            } => {
                assert!(index != &RSP, "illegal addressing mode");
                if *offset == 0 {
                    // [index*scale + offset]
                    // [00 reg 100][scala index 101] 0i32
                    asm.encode_modrm_code(0, reg, 4);
                    asm.encode_sib(*scala, *index, RBP);
                    asm.push_u32(0);
                } else {
                    // [index*scale + offset]
                    // [00 reg 100][scala index 101] offset_i32
                    asm.encode_modrm_code(0, reg, 4);
                    asm.encode_sib(*scala, *index, RBP);
                    asm.push_i32(*offset);
                }
            }
            AddressOperand::BaseAndIndex { base, index, scala } => {
                assert!(index != &RSP, "illegal addressing mode");
                match *base {
                    RBP | R13 => {
                        // [rbp|r13 + index*scale]
                        // [01 reg 100][scala index 101] 0i8
                        asm.encode_modrm_code(1, reg, 4);
                        asm.encode_sib(*scala, *index, RBP);
                        asm.push_i8(0);
                    }
                    _ => {
                        // [base + index*scale]
                        // [00 reg 100][scala index base]
                        asm.encode_modrm_code(0, reg, 4);
                        asm.encode_sib(*scala, *index, *base);
                    }
                }
            }
            AddressOperand::BaseAndIndexAndOffset {
                base,
                index,
                scala,
                offset,
            } => {
                assert!(index != &RSP, "illegal addressing mode");
                if *offset == 0 && base != &RBP && base != &R13 {
                    // [base + index*scale]
                    // [00 reg 100][scala index base]
                    asm.encode_modrm_code(0, reg, 0b100);
                    asm.encode_sib(*scala, *index, *base);
                } else {
                    match (*offset) >> 7 {
                        0 | -1 => {
                            // [base + index*scale + offset]
                            // [01 reg 100][scala index base] offset_i8
                            asm.encode_modrm_code(1, reg, 0b100);
                            asm.encode_sib(*scala, *index, *base);
                            asm.push_i8(*offset as i8);
                        }
                        _ => {
                            // [base + index*scale + offset]
                            // [10 reg 100][scala index base] offset_i32
                            asm.encode_modrm_code(2, reg, 4);
                            asm.encode_sib(*scala, *index, *base);
                            asm.push_i32(*offset);
                        }
                    }
                }
            }
            AddressOperand::Direct(addr) => {
                // [offset] ABSOLUTE
                // [00 reg 100][00 100 101] offset_i32
                asm.encode_modrm_code(0, reg, 4);
                asm.encode_sib(Scala1, RSP, RBP);
                asm.push_i32(*addr);
            }
            AddressOperand::RipRelative(target) => {
                asm.encode_modrm_code(0, reg, 5);
                let next_instruction = asm.relatively_label() + 4;
                let offset = label_offset(*target, next_instruction);
                asm.push_i32(i32::try_from(offset).unwrap());
            }
        }
    }
}
pub mod prefix {
    pub const REX: u8 = 0b0100_0000;
    pub const REX_W: u8 = 0b0100_1000;
    pub const REX_R: u8 = 0b0100_0100; // Extension of modRT/M reg field
    pub const REX_X: u8 = 0b0100_0010; // Extension of SIB index field
    pub const REX_B: u8 = 0b0100_0001; // EXtension of the ModR/M r/m field, SIB base field,or OPcode reg field
    pub const SCALAR_F32: u8 = 0xf3;
    pub const SCALAR_F64: u8 = 0xf2;
}
#[derive(Clone, Copy)]
pub enum Scala {
    Scala1,
    Scala2,
    Scala4,
    Scala8,
}
impl Scala {
    pub fn encode(&self) -> u8 {
        let c = match self {
            Scala::Scala1 => 0,
            Scala::Scala2 => 1,
            Scala::Scala4 => 2,
            Scala::Scala8 => 3,
        };
        c << 6
    }

    pub fn new(times: u8) -> Scala {
        match times {
            1 => Scala::Scala1,
            2 => Scala::Scala2,
            4 => Scala::Scala4,
            8 => Scala::Scala8,
            _ => panic!(),
        }
    }
}
#[derive(Clone, Copy)]
pub enum OpCode {
    U8Narrow(InstructionSet, u8),
    U8(InstructionSet, u8),
    U8Extend(InstructionSet, u8, u8),
    U16(InstructionSet, u8),
    U16Prefix(InstructionSet, u8, u8),
    U16PrefixREXW(InstructionSet, u8, u8),
}
pub mod opcode_extension {
    pub const ADD: u8 = 0 << 3;
    pub const OR: u8 = 1 << 3;
    pub const ADC: u8 = 2 << 3;
    pub const SBB: u8 = 3 << 3;
    pub const AND: u8 = 4 << 3;
    pub const SUB: u8 = 5 << 3;
    pub const XOR: u8 = 6 << 3;
    pub const CMP: u8 = 7 << 3;
}
impl OpCode {
    pub const fn set_d(&self) -> OpCode {
        match self {
            OpCode::U8(i, c) => OpCode::U8(*i, *c | 0x02),
            OpCode::U8Extend(i, c0, c1) => OpCode::U8Extend(*i, *c0 | 0x02, *c1),
            o => *o,
        }
    }

    pub const fn set_one_byte(&self) -> OpCode {
        self.set_d()
    }

    pub const fn set_b(&self) -> OpCode {
        match self {
            OpCode::U8(i, c) => OpCode::U8(*i, *c | 0x01),
            OpCode::U8Extend(i, c0, c1) => OpCode::U8Extend(*i, *c0 | 0x01, *c1),
            o => *o,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum InstructionSet {
    X86_64,
    SSE,
    SSE2,
    AVX,
}
pub struct Mnemonic {
    pub m_from_r8: Option<OpCode>,
    pub r8_from_m: Option<OpCode>,

    pub r_from_r_or_m: Option<OpCode>,
    pub r_or_m_from_r: Option<OpCode>,

    pub r_from_m: Option<OpCode>,
    pub m_from_r: Option<OpCode>,

    pub r_from_i32: Option<OpCode>,
    pub r_from_i8: Option<OpCode>,

    pub m_from_i8: Option<OpCode>,
    pub m_from_i16: Option<OpCode>,
    pub m_from_i32: Option<OpCode>,

    pub r_from_r_and_i32: Option<OpCode>,
    pub r_from_r_and_i8: Option<OpCode>,

    pub r: Option<OpCode>,
    pub dst_only: Option<OpCode>,
    pub no_operand: Option<OpCode>,

    pub xmm_from_xmm: Option<OpCode>,

    pub xmm_from_r32: Option<OpCode>,
    pub r32_from_xmm: Option<OpCode>,
    pub xmm_from_m32: Option<OpCode>,
    pub m32_from_xmm: Option<OpCode>,

    pub xmm_from_r64: Option<OpCode>,
    pub r64_from_xmm: Option<OpCode>,
    pub xmm_from_m64: Option<OpCode>,
    pub m64_from_xmm: Option<OpCode>,

    pub no_operand_32: Option<OpCode>,
}
const MNEMONIC_NONE: Mnemonic = Mnemonic {
    r_from_i32: None,
    r_from_i8: None,
    m_from_i8: None,
    m_from_i16: None,
    m_from_i32: None,
    m_from_r8: None,
    r8_from_m: None,
    r_from_r_or_m: None,
    r_or_m_from_r: None,
    r_from_m: None,
    m_from_r: None,
    r_from_r_and_i32: None,
    r_from_r_and_i8: None,
    r: None,
    dst_only: None,
    no_operand: None,
    xmm_from_xmm: None,
    xmm_from_m32: None,
    m32_from_xmm: None,
    r32_from_xmm: None,
    xmm_from_r32: None,
    xmm_from_m64: None,
    m64_from_xmm: None,
    r64_from_xmm: None,
    xmm_from_r64: None,
    no_operand_32: None,
};
impl Mnemonic {
    fn encode_opcode_u8_or_u16(asm: &mut Assembler, opcode: &OpCode) {
        match opcode {
            OpCode::U8(_, c) => {
                asm.push_u8(*c);
            }
            OpCode::U16(_, c) => {
                asm.push_u8(0x0f);
                asm.push_u8(*c);
            }
            OpCode::U16Prefix(_, p, c) => {
                asm.push_u8(*p);
                asm.push_u8(0x0f);
                asm.push_u8(*c);
            }
            _ => panic!(),
        }
    }

    fn encode_opcode_u8_narrow_or_extend(asm: &mut Assembler, opcode: &OpCode, dst: Register) {
        match opcode {
            OpCode::U8Narrow(_, c) => {
                asm.rex_32_prefix_reg(dst);
                asm.push_u8(*c | dst.encode());
            }
            U8Extend(_, c1, c2) => {
                asm.rex_32_prefix_reg(dst);
                asm.push_u8(*c1);
                asm.push_u8(*c2 | dst.encode());
            }
            _ => panic!(),
        }
    }

    pub fn r32_from_i32(&self, asm: &mut Assembler, dst: Register, value: i32) {
        asm.rex_32_prefix_reg(dst);
        let opcode = &self.r_from_i32.unwrap();
        Mnemonic::encode_opcode_u8_narrow_or_extend(asm, opcode, dst);
        asm.push_i32(value);
    }

    pub fn r64_from_i32(&self, asm: &mut Assembler, dst: Register, value: i32) {
        asm.rex_64_prefix_reg(dst);
        let opcode = &self.r_from_i32.unwrap();
        Mnemonic::encode_opcode_u8_narrow_or_extend(asm, opcode, dst);
        asm.push_i32(value);
    }

    pub fn r32_from_i8(&self, asm: &mut Assembler, dst: Register, value: i8) {
        asm.rex_32_prefix_reg(dst);
        let opcode = &self.r_from_i8.unwrap();
        Mnemonic::encode_opcode_u8_narrow_or_extend(asm, opcode, dst);
        asm.push_i8(value);
    }

    pub fn r64_from_i8(&self, asm: &mut Assembler, dst: Register, value: i8) {
        let opcode = &self.r_from_i8.unwrap();
        asm.rex_64_prefix_reg(dst);
        Mnemonic::encode_opcode_u8_narrow_or_extend(asm, opcode, dst);
        asm.push_i8(value);
    }

    pub fn r32(&self, asm: &mut Assembler, reg: Register) {
        asm.rex_32_prefix_reg(reg);
        let opcode = &self.r.unwrap();
        Mnemonic::encode_opcode_u8_narrow_or_extend(asm, opcode, reg);
    }

    pub fn r64(&self, asm: &mut Assembler, reg: Register) {
        asm.rex_64_prefix_reg(reg);
        let opcode = &self.r.unwrap();
        Mnemonic::encode_opcode_u8_narrow_or_extend(asm, opcode, reg);
    }

    pub fn r64_from_r(&self, asm: &mut Assembler, dst: Register, src: Register) {
        if let Some(opcode) = &self.r_or_m_from_r {
            asm.rex_64_prefix(&dst, &src);
            Mnemonic::encode_opcode_u8_or_u16(asm, opcode);
            asm.encode_modrm(3, src, dst);
        } else {
            let opcode = &self.r_from_r_or_m.unwrap();
            asm.rex_64_prefix(&src, &dst);
            Mnemonic::encode_opcode_u8_or_u16(asm, opcode);
            asm.encode_modrm(3, dst, src);
        }
    }

    pub fn r32_from_r(&self, asm: &mut Assembler, dst: Register, src: Register) {
        if let Some(opcode) = &self.r_or_m_from_r {
            asm.rex_32_prefix(&dst, &src);
            Mnemonic::encode_opcode_u8_or_u16(asm, opcode);
            asm.encode_modrm(3, src, dst);
        } else {
            let opcode = &self.r_from_r_or_m.unwrap();
            asm.rex_32_prefix(&src, &dst);
            Mnemonic::encode_opcode_u8_or_u16(asm, opcode);
            asm.encode_modrm(3, dst, src);
        }
    }

    pub fn r32_from_m(&self, asm: &mut Assembler, dst: Register, src: AddressOperand) {
        asm.rex_32_prefix_address(&dst, &src);
        let opcode = &self.r_from_r_or_m.or(self.r_from_m).unwrap();
        Mnemonic::encode_opcode_u8_or_u16(asm, opcode);
        src.encode(dst, asm);
    }

    pub fn r64_from_m(&self, asm: &mut Assembler, dst: Register, src: AddressOperand) {
        asm.rex_64_prefix_address(&dst, &src);
        let opcode = &self.r_from_r_or_m.or(self.r_from_m).unwrap();
        Mnemonic::encode_opcode_u8_or_u16(asm, opcode);
        src.encode(dst, asm);
    }

    pub fn m_from_r32(&self, asm: &mut Assembler, dst: AddressOperand, src: Register) {
        asm.rex_32_prefix_address(&src, &dst);
        let opcode = &self.r_or_m_from_r.or(self.m_from_r).unwrap();
        Mnemonic::encode_opcode_u8_or_u16(asm, opcode);
        dst.encode(src, asm);
    }

    pub fn m_from_r64(&self, asm: &mut Assembler, dst: AddressOperand, src: Register) {
        asm.rex_64_prefix_address(&src, &dst);
        let opcode = &self.r_or_m_from_r.or(self.m_from_r).unwrap();
        Mnemonic::encode_opcode_u8_or_u16(asm, opcode);
        dst.encode(src, asm);
    }

    pub fn r8_from_m(&self, asm: &mut Assembler, dst: Register, src: AddressOperand) {
        asm.rex_8_prefix_address(&dst, &src);
        let opcode = &self.r8_from_m.unwrap();
        Mnemonic::encode_opcode_u8_or_u16(asm, opcode);
        src.encode(dst, asm);
    }

    pub fn xmm_from_xmm(&self, asm: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
        let opcode = &self.xmm_from_xmm.unwrap();
        if let U16Prefix(_, prefix, _) = opcode {
            asm.push_u8(*prefix);
        };
        asm.rex_32_prefix_xmm(&dst, &src);
        Mnemonic::encode_opcode_u8_or_u16(asm, opcode);
        asm.encode_modrm_xmm(0x11, src, dst);
    }

    pub fn xmm_from_m32(&self, asm: &mut Assembler, dst: XMMRegister, src: AddressOperand) {
        let opcode = &self.xmm_from_m32.unwrap();
        if let U16Prefix(_, prefix, _) = opcode {
            asm.push_u8(*prefix);
        };
        Mnemonic::encode_opcode_u8_or_u16(asm, opcode);
        src.encode_xmm(dst, asm);
    }

    pub fn xmm_from_m64(&self, asm: &mut Assembler, dst: XMMRegister, src: AddressOperand) {
        let opcode = &self.xmm_from_m64.unwrap();
        if let U16Prefix(_, prefix, _) = opcode {
            asm.push_u8(*prefix);
        };
        Mnemonic::encode_opcode_u8_or_u16(asm, opcode);
        src.encode_xmm(dst, asm);
    }

    pub fn xmm_from_r32(&self, asm: &mut Assembler, dst: XMMRegister, src: Register) {
        let opcode = &self.xmm_from_r32.unwrap();
        if let U16Prefix(_, prefix, _) = opcode {
            asm.push_u8(*prefix);
        };
        Mnemonic::encode_opcode_u8_or_u16(asm, opcode);
        asm.encode_modrm_code(3, src.encode(), dst.encode());
    }

    pub fn xmm_from_r64(&self, asm: &mut Assembler, dst: XMMRegister, src: Register) {
        let opcode = &self.xmm_from_r32.unwrap();
        match opcode {
            U16Prefix(_, prefix, _) => {
                asm.push_u8(*prefix);
            }
            U16PrefixREXW(_, prefix, _) => {
                asm.push_u8(*prefix);
                asm.rex_prefix_code(
                    false,
                    false,
                    true,
                    src.code_between_8_and_15(),
                    false,
                    dst.code_between_8_and_15(),
                );
            }
            _ => {}
        }
        if let U16Prefix(_, prefix, _) = opcode {
            asm.push_u8(*prefix);
        };
        Mnemonic::encode_opcode_u8_or_u16(asm, opcode);
        asm.encode_modrm_code(3, src.encode(), dst.encode());
    }

    pub fn r32_from_xmm(&self, asm: &mut Assembler, dst: Register, src: XMMRegister) {
        let opcode = &self.r32_from_xmm.unwrap();
        if let U16Prefix(_, prefix, _) = opcode {
            asm.push_u8(*prefix);
        };
        Mnemonic::encode_opcode_u8_or_u16(asm, opcode);
        asm.encode_modrm_code(3, src.encode(), dst.encode());
    }

    pub fn r64_from_xmm(&self, asm: &mut Assembler, dst: Register, src: XMMRegister) {
        let opcode = &self.r64_from_xmm.unwrap();
        match opcode {
            U16Prefix(_, prefix, _) => {
                asm.push_u8(*prefix);
            }
            U16PrefixREXW(_, prefix, _) => {
                asm.push_u8(*prefix);
                asm.rex_prefix_code(
                    false,
                    false,
                    true,
                    src.code_between_8_and_15(),
                    false,
                    dst.code_between_8_and_15(),
                );
            }
            _ => {}
        }
        if let U16Prefix(_, prefix, _) = opcode {
            asm.push_u8(*prefix);
        };
        Mnemonic::encode_opcode_u8_or_u16(asm, opcode);
        asm.encode_modrm_code(3, src.encode(), dst.encode());
    }

    pub fn no_operand_32(&self, asm: &mut Assembler) {
        Mnemonic::encode_opcode_u8_or_u16(asm, &self.no_operand.unwrap());
    }

    pub fn no_operand_64(&self, asm: &mut Assembler) {
        asm.rex_prefix_code(false, false, true, false, false, false);
        Mnemonic::encode_opcode_u8_or_u16(asm, &self.no_operand.unwrap());
    }

    pub fn m32_from_xmm(&self, asm: &mut Assembler, dst: AddressOperand, src: XMMRegister) {
        asm.rex_32_prefix_xmm_address(&src, &dst);
        let opcode = &self.m32_from_xmm.or(self.m_from_r).unwrap();
        Mnemonic::encode_opcode_u8_or_u16(asm, opcode);
        dst.encode_xmm(src, asm);
    }

    pub fn m64_from_xmm(&self, asm: &mut Assembler, dst: AddressOperand, src: XMMRegister) {
        asm.rex_64_prefix_xmm_address(&src, &dst);
        let opcode = &self.m64_from_xmm.or(self.m_from_r).unwrap();
        Mnemonic::encode_opcode_u8_or_u16(asm, opcode);
        dst.encode_xmm(src, asm);
    }
}
macro_rules! define_mnemonic {
    ($name:tt,$m_from_r:expr,$r_from_i32:expr) => {
        pub const $name: Mnemonic = Mnemonic {
            r_or_m_from_r: Some($m_from_r),
            r_from_r_or_m: Some($m_from_r.set_d()),
            m_from_r8: Some($m_from_r.set_b()),
            r8_from_m: Some($m_from_r.set_d().set_b()),
            r_from_i32: Some($r_from_i32),
            r_from_i8: Some($r_from_i32.set_one_byte()),
            ..MNEMONIC_NONE
        };
    };
}
macro_rules! define_mnemonic_sse_f32 {
    ($name:tt,$opcode:expr) => {
        pub const $name: Mnemonic = Mnemonic {
            xmm_from_xmm: Some($opcode),
            xmm_from_m32: Some($opcode),
            xmm_from_r32: Some($opcode),
            ..MNEMONIC_NONE
        };
    };
}
macro_rules! define_mnemonic_sse_f64 {
    ($name:tt,$opcode:expr) => {
        pub const $name: Mnemonic = Mnemonic {
            xmm_from_xmm: Some($opcode),
            xmm_from_m64: Some($opcode),
            xmm_from_r64: Some($opcode),
            ..MNEMONIC_NONE
        };
    };
}
macro_rules! define_mnemonic_dst_only {
    ($name:tt,$opcode:expr) => {
        pub const $name: Mnemonic = Mnemonic {
            dst_only: Some($opcode),
            ..MNEMONIC_NONE
        };
    };
}
define_mnemonic!(
    ADD,
    U8(X86_64, 0x01),
    U8Extend(X86_64, 0x81, opcode_extension::ADD << 3)
);
define_mnemonic!(
    OR,
    U8(X86_64, 0x0b),
    U8Extend(X86_64, 0x81, opcode_extension::OR << 3)
);
define_mnemonic!(
    ADC,
    U8(X86_64, 0x11),
    U8Extend(X86_64, 0x81, opcode_extension::ADC << 3)
);
define_mnemonic!(
    AND,
    U8(X86_64, 0x23),
    U8Extend(X86_64, 0x81, opcode_extension::AND << 3)
);
define_mnemonic!(
    SUB,
    U8(X86_64, 0x29),
    U8Extend(X86_64, 0x81, opcode_extension::SUB << 3)
);
define_mnemonic!(
    XOR,
    U8(X86_64, 0x33),
    U8Extend(X86_64, 0x81, opcode_extension::XOR << 3)
);
define_mnemonic!(
    CMP,
    U8(X86_64, 0x39),
    U8Extend(X86_64, 0x81, opcode_extension::CMP << 3)
);

define_mnemonic_sse_f64!(MULSD, U16Prefix(SSE2, 0xf2, 0x59));
define_mnemonic_sse_f32!(MULSS, U16Prefix(SSE, 0xf3, 0x59));
define_mnemonic_sse_f64!(SUBSD, U16Prefix(SSE2, 0xf2, 0x5c));
define_mnemonic_sse_f32!(SUBSS, U16Prefix(SSE, 0xf3, 0x5c));
define_mnemonic_sse_f64!(ADDSD, U16Prefix(SSE2, 0xf2, 0x58));
define_mnemonic_sse_f32!(ADDSS, U16Prefix(SSE, 0xf3, 0x58));
define_mnemonic_sse_f64!(DIVSD, U16Prefix(SSE2, 0xf2, 0x5e));
define_mnemonic_sse_f32!(DIVSS, U16Prefix(SSE, 0xf3, 0x5e));
define_mnemonic_sse_f32!(CVTSI2SS, U16Prefix(SSE, 0xf3, 0x2a));
define_mnemonic_sse_f64!(CVTSI2SD, U16Prefix(SSE2, 0xf2, 0x2a));
define_mnemonic_sse_f32!(MAXSS, U16Prefix(SSE, 0xf3, 0x5f));
define_mnemonic_sse_f64!(MAXSD, U16Prefix(SSE, 0xf2, 0x5f));
define_mnemonic_sse_f32!(UCOMISS, U16(SSE, 0x2e));
define_mnemonic_sse_f64!(UCOMISD, U16Prefix(SSE, 0x66, 0x2e));
pub const CVTTSS2SI: Mnemonic = Mnemonic {
    r32_from_xmm: Some(U16Prefix(SSE, 0xf3, 0x2c)),
    r64_from_xmm: Some(U16Prefix(SSE, 0xf3, 0x2c)),
    r_from_m: Some(U16Prefix(SSE, 0xf3, 0x2c)),
    ..MNEMONIC_NONE
};

pub const CVTTSD2SI: Mnemonic = Mnemonic {
    r32_from_xmm: Some(U16Prefix(SSE2, 0xf2, 0x2c)),
    r64_from_xmm: Some(U16Prefix(SSE2, 0xf2, 0x2c)),
    r_from_m: Some(U16Prefix(SSE2, 0xf2, 0x2c)),
    ..MNEMONIC_NONE
};
pub const CVTSS2SD: Mnemonic = Mnemonic {
    xmm_from_xmm: Some(U16Prefix(SSE2, 0xf3, 0x5a)),
    xmm_from_m32: Some(U16Prefix(SSE2, 0xf3, 0x5a)),
    ..MNEMONIC_NONE
};
pub const CVTSD2SS: Mnemonic = Mnemonic {
    xmm_from_xmm: Some(U16Prefix(SSE2, 0xf2, 0x5a)),
    xmm_from_m64: Some(U16Prefix(SSE2, 0xf2, 0x5a)),
    ..MNEMONIC_NONE
};
// define_mnemonic_sse_f32!(CVTTSS2SI,U16Prefix(SSE,0xf3,0x2c));
// define_mnemonic_sse_f64!(CVTTSD2SI,U16Prefix(SSE,0xf2,0x2c));
pub const IMUL: Mnemonic = Mnemonic {
    r_from_r_and_i32: Some(U8(X86_64, 0x69)),
    r_from_r_and_i8: Some(U8(X86_64, 0x6b)),
    r_from_r_or_m: Some(U16(X86_64, 0xaf)),
    ..MNEMONIC_NONE
};
pub const IDIV: Mnemonic = Mnemonic {
    r: Some(U8Extend(X86_64, 0xf7, 7 << 3)),
    ..MNEMONIC_NONE
};
pub const CDQ: Mnemonic = Mnemonic {
    no_operand_32: Some(U8(X86_64, 0x99)),
    ..MNEMONIC_NONE
};

pub const TEST: Mnemonic = Mnemonic {
    r_or_m_from_r: Some(U8(X86_64, 0x85)),
    r_from_i32: Some(U8Extend(X86_64, 0xf7, 0 << 3)),
    ..MNEMONIC_NONE
};
define_mnemonic_dst_only!(NEG, U8Extend(X86_64, 0xf7, 3 << 3));
define_mnemonic_dst_only!(SAR, U8Extend(X86_64, 0xd3, 7 << 3));
define_mnemonic_dst_only!(SHL, U8Extend(X86_64, 0xd3, 6 << 3));
define_mnemonic_dst_only!(SHR, U8Extend(X86_64, 0xd3, 5 << 3));
define_mnemonic_dst_only!(INC, U8Extend(X86_64, 0xff, 0 << 3));
pub const MOV: Mnemonic = Mnemonic {
    r_or_m_from_r: Some(U8(X86_64, 0x89)),
    r_from_r_or_m: Some(U8(X86_64, 0x8b)),
    m_from_r8: Some(U8(X86_64, 0x88)),
    r8_from_m: Some(U8(X86_64, 0x9a)),
    m_from_i8: Some(U8(X86_64, 0xc6)),
    m_from_i32: Some(U8(X86_64, 0xc7)),
    r_from_i32: Some(U8Narrow(X86_64, 0xb8)),
    m_from_i16: Some(U8(X86_64, 0xc7)),
    ..MNEMONIC_NONE
};
pub const MOVQ: Mnemonic = Mnemonic {
    xmm_from_m64: Some(U16Prefix(SSE2, 0xf3, 0x7e)),
    m64_from_xmm: Some(U16Prefix(SSE2, 0x66, 0xd6)),
    r64_from_xmm: Some(U16PrefixREXW(SSE2, 0x66, 0x7e)),
    xmm_from_r64: Some(U16PrefixREXW(SSE2, 0x66, 0x6e)),
    ..MNEMONIC_NONE
};
pub const MOVD: Mnemonic = Mnemonic {
    xmm_from_m32: Some(U16Prefix(SSE2, 0x66, 0x6e)),
    m32_from_xmm: Some(U16Prefix(SSE2, 0x66, 0x7e)),
    r32_from_xmm: Some(U16Prefix(SSE2, 0x66, 0x7e)),
    xmm_from_r32: Some(U16Prefix(SSE2, 0x66, 0x6e)),
    ..MNEMONIC_NONE
};
pub const MOVSX_16: Mnemonic = Mnemonic {
    r_from_r_or_m: Some(U16(X86_64, 0xB7)),
    ..MNEMONIC_NONE
};
pub const MOVSX_8: Mnemonic = Mnemonic {
    r_from_r_or_m: Some(U16(X86_64, 0xB6)),
    ..MNEMONIC_NONE
};
pub const MOVZX_16: Mnemonic = Mnemonic {
    r_from_r_or_m: Some(U16(X86_64, 0xBf)),
    ..MNEMONIC_NONE
};
pub const MOVZX_8: Mnemonic = Mnemonic {
    r_from_r_or_m: Some(U16(X86_64, 0xBe)),
    ..MNEMONIC_NONE
};
pub const PUSH: Mnemonic = Mnemonic {
    r: Some(U8Narrow(X86_64, 0x50)),
    ..MNEMONIC_NONE
};
pub const POP: Mnemonic = Mnemonic {
    dst_only: Some(U8Narrow(X86_64, 0x58)),
    ..MNEMONIC_NONE
};
pub fn mov_m_from_r16(asm: &mut Assembler, dst: AddressOperand, src: Register) {
    asm.push_u8(0x66);
    dst.rex_prefix(asm, &src, false, false);
    asm.push_u8(0x89);
    dst.encode(src, asm);
}
pub fn movabs(asm: &mut Assembler, dst: Register, value: i64) {
    if dst.code_between_8_and_15() {
        asm.push_u8(0x49);
    }
    asm.push_u8(0xb8);
    asm.push_i64(value);
}
pub fn movsxd_r64_from_r32(asm: &mut Assembler, dst: Register, src: Register) {
    asm.rex_64_prefix(&dst, &src);
    asm.push_u8(0x63);
    asm.encode_modrm(3, dst, src);
}
pub fn movsxd_r64_from_m32(asm: &mut Assembler, dst: Register, src: AddressOperand) {
    src.rex_prefix(asm, &dst, true, false);
    asm.push_u8(0x63);
    src.encode(dst, asm);
}
pub fn movaps_xmm_from_xmm(asm: &mut Assembler, dst: XMMRegister, src: XMMRegister) {
    asm.rex_32_prefix_xmm(&src, &dst);
    asm.push_u8(0x0f);
    asm.push_u8(0x28);
    asm.encode_modrm_xmm(3, dst, src);
}
pub const CMOVNP: Mnemonic = Mnemonic {
    r_from_r_or_m: Some(U16(X86_64, 0x4B)),
    ..MNEMONIC_NONE
};
pub const CMOVBE: Mnemonic = Mnemonic {
    r_from_r_or_m: Some(U16(X86_64, 0x46)),
    ..MNEMONIC_NONE
};
pub fn jump_conditional_to(asm: &mut Assembler, condition: Condition, target: &Label) {
    let instruction_start = asm.relatively_label();
    let offset = label_offset(*target, instruction_start);
    match offset - 2 {
        -128..128 => {
            asm.push_u8(0x70 | condition.encode());
            asm.push_i8((offset - 2) as i8);
        }
        _ => {
            asm.push_u8(0x0f);
            asm.push_u8(0x80 | condition.encode());
            asm.push_i32(i32::try_from(offset - 6).unwrap());
        }
    }
}
pub fn jump_conditional_short_from(asm: &mut Assembler, condition: Condition) -> JumpShortLabel {
    asm.push_u8(0x70 | condition.encode());
    let label = asm.relatively_label();
    asm.push_i8(0);
    JumpShortLabel(label)
}

pub fn jump_conditional_near_from(asm: &mut Assembler, condition: Condition) -> JumpNearLabel {
    asm.push_u8(0x0f);
    asm.push_u8(0x80 | condition.encode());
    let label = asm.relatively_label();
    asm.push_i32(0);
    JumpNearLabel(label)
}
pub fn set_condition(asm: &mut Assembler, condition: Condition, dst: Register) {
    asm.rex_8_prefix_reg(&dst);
    asm.push_u8(0x0f);
    asm.push_u8(0x90 | condition.encode());
    asm.push_u8(0xc0 | dst.encode());
}
pub fn mov_conditional_r32_from_r(
    asm: &mut Assembler,
    condition: Condition,
    dst: Register,
    src: Register,
) {
    asm.rex_32_prefix(&src, &dst);
    asm.push_u8(0x0f);
    asm.push_u8(0x40 | condition.encode());
    asm.encode_modrm(3, dst, src);
}
pub fn mov_conditional_r64_from_r(
    asm: &mut Assembler,
    condition: Condition,
    dst: Register,
    src: Register,
) {
    asm.rex_64_prefix(&src, &dst);
    asm.push_u8(0x0f);
    asm.push_u8(0x40 | condition.encode());
    asm.encode_modrm(3, dst, src);
}
pub fn mov_conditional_r64_from_m(
    asm: &mut Assembler,
    condition: Condition,
    dst: Register,
    src: AddressOperand,
) {
    src.rex_prefix(asm, &dst, true, false);
    asm.push_u8(0x0f);
    asm.push_u8(0x40 | condition.encode());
    src.encode(dst, asm);
}
pub fn cmpss_condition(
    asm: &mut Assembler,
    condition: Condition,
    op1: XMMRegister,
    op2: XMMRegister,
) {
    asm.push_u8(0xf3);
    asm.push_u8(0x0f);
    asm.push_u8(0xc2);
    asm.encode_modrm_xmm(3, op1, op2);
    asm.push_u8(condition.encode());
}
pub fn lea_64(asm: &mut Assembler, dst: Register, address: AddressOperand) {
    address.rex_prefix(asm, &dst, true, false);
    asm.push_u8(0x8d);
    address.encode(dst, asm);
}
pub fn jmp_relative_address(asm: &mut Assembler, address: AddressOperand) {
    address.prefix_rex_code(asm, false, false, false, false);
    asm.push_u8(0xff);
    address.encode_code(4, asm);
}

pub fn shl_r64_from_i8(asm: &mut Assembler, dst: Register, imme: i8) {
    asm.rex_64_prefix_reg(dst);
    asm.push_u8(0xc0);
    asm.encode_modrm_code(3, 4, dst.encode());
    asm.push_i8(imme);
}
pub const CALL: Mnemonic = Mnemonic {
    dst_only: Some(U8Extend(X86_64, 0xff, 2)),
    ..MNEMONIC_NONE
};
pub fn ret(asm: &mut Assembler) {
    asm.push_u8(0xc3);
}
