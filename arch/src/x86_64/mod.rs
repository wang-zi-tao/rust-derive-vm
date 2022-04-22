use crate::{Architecture, RegisterBaseArch};

use self::registers::Register;

#[macro_use]
pub mod assembler;
pub const MINIMUMALLOCATION_UNIIT_OF_VM: usize = 1 << 12;
pub const TOTAL_VM: usize = 1 << 47;

pub struct X86_64 {}
impl Architecture for X86_64 {}
impl RegisterBaseArch for X86_64 {
    type GenericRegister = Register;
}
mod registers {
    use crate::GenericRegister;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Register {
        id: u8,
    }
    impl Register {
        pub fn encode(&self) -> u8 {
            self.id & 7
        }

        pub fn encode_shift(&self, _offset: u32, shift: u8) -> u8 {
            self.encode() << shift
        }

        pub fn code_between_8_and_15(&self) -> bool {
            self.id > 7
        }

        pub fn code_between_4_and_15(&self) -> bool {
            self.id > 3
        }

        pub const fn new(id: u8) -> Self {
            Self { id }
        }
    }
    impl GenericRegister for Register {}
    pub const RAX: Register = Register::new(0);
    pub const RCX: Register = Register::new(1);
    pub const RDX: Register = Register::new(2);
    pub const RBX: Register = Register::new(3);
    pub const RSP: Register = Register::new(4);
    pub const RBP: Register = Register::new(5);
    pub const RSI: Register = Register::new(6);
    pub const RDI: Register = Register::new(7);
    pub const R8: Register = Register::new(8);
    pub const R9: Register = Register::new(9);
    pub const R10: Register = Register::new(10);
    pub const R11: Register = Register::new(11);
    pub const R12: Register = Register::new(12);
    pub const R13: Register = Register::new(13);
    pub const R14: Register = Register::new(14);
    pub const R15: Register = Register::new(15);
}

pub mod xmm_registers {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct XMMRegister {
        id: u8,
    }
    impl XMMRegister {
        pub const fn new(id: u8) -> XMMRegister {
            Self { id }
        }

        pub fn encode(&self) -> u8 {
            self.id & 7
        }

        pub fn encode_shift(&self, _offset: u32, shift: u8) -> u8 {
            self.encode() << shift
        }

        pub fn code_between_8_and_15(&self) -> bool {
            self.id > 7
        }

        pub fn code_between_4_and_15(&self) -> bool {
            self.id > 3
        }
    }
    pub const XMM0: XMMRegister = XMMRegister::new(0);
    pub const XMM1: XMMRegister = XMMRegister::new(1);
    pub const XMM2: XMMRegister = XMMRegister::new(2);
    pub const XMM3: XMMRegister = XMMRegister::new(3);
    pub const XMM4: XMMRegister = XMMRegister::new(4);
    pub const XMM5: XMMRegister = XMMRegister::new(5);
    pub const XMM6: XMMRegister = XMMRegister::new(6);
    pub const XMM7: XMMRegister = XMMRegister::new(7);
    pub const XMM8: XMMRegister = XMMRegister::new(8);
    pub const XMM9: XMMRegister = XMMRegister::new(9);
    pub const XMM10: XMMRegister = XMMRegister::new(10);
    pub const XMM11: XMMRegister = XMMRegister::new(11);
    pub const XMM12: XMMRegister = XMMRegister::new(12);
    pub const XMM13: XMMRegister = XMMRegister::new(13);
    pub const XMM14: XMMRegister = XMMRegister::new(14);
    pub const XMM15: XMMRegister = XMMRegister::new(15);
}
