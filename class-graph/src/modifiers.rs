use std::collections::HashMap;

use classfile::attributes::Attribute;
use jvm_core::Component;
use modifier::*;
use util::PooledStr;
pub trait Modifier {
    fn is_abstract(&self) -> bool;
    fn is_public(&self) -> bool;
    fn is_private(&self) -> bool;
    fn is_protected(&self) -> bool;
    fn is_static(&self) -> bool;
    fn is_final(&self) -> bool;
    fn is_synchronized(&self) -> bool;
    fn is_volatile(&self) -> bool;
    fn is_transient(&self) -> bool;
    fn is_native(&self) -> bool;
    fn is_interface(&self) -> bool;
    fn is_strict(&self) -> bool;
    fn is_implicit(&self) -> bool;
    fn is_synthetic(&self) -> bool;
    fn is_annotation(&self) -> bool;
    fn is_enum(&self) -> bool;
    fn is_primitive(&self) -> bool;
}
impl Modifier for i32 {
    fn is_abstract(&self) -> bool {
        0 != *self & modifier::ABSTRACT
    }

    fn is_public(&self) -> bool {
        0 != *self & modifier::PUBLIC
    }

    fn is_private(&self) -> bool {
        0 != *self & modifier::PRIVATE
    }

    fn is_protected(&self) -> bool {
        0 != *self & modifier::PROTECTED
    }

    fn is_static(&self) -> bool {
        0 != *self & modifier::STATIC
    }

    fn is_final(&self) -> bool {
        0 != *self & modifier::FINAL
    }

    fn is_synchronized(&self) -> bool {
        0 != *self & modifier::SYNCHRONIZED
    }

    fn is_volatile(&self) -> bool {
        0 != *self & modifier::VOLATILE
    }

    fn is_transient(&self) -> bool {
        0 != *self & modifier::TRANSIENT
    }

    fn is_native(&self) -> bool {
        0 != *self & modifier::NATIVE
    }

    fn is_interface(&self) -> bool {
        0 != *self & modifier::INTERFACE
    }

    fn is_strict(&self) -> bool {
        0 != *self & modifier::STRICT
    }

    fn is_implicit(&self) -> bool {
        0 != *self & modifier::IMPLICIT
    }

    fn is_synthetic(&self) -> bool {
        0 != *self & modifier::SYNTHETIC
    }

    fn is_annotation(&self) -> bool {
        0 != *self & modifier::ANNOTATION
    }

    fn is_enum(&self) -> bool {
        0 != *self & modifier::ENUM
    }

    fn is_primitive(&self) -> bool {
        0 != *self & modifier::PRIMITIVE
    }
}
impl Modifier for i16 {
    fn is_abstract(&self) -> bool {
        0 != (*self as i32) & modifier::ABSTRACT
    }

    fn is_public(&self) -> bool {
        0 != (*self as i32) & modifier::PUBLIC
    }

    fn is_private(&self) -> bool {
        0 != (*self as i32) & modifier::PRIVATE
    }

    fn is_protected(&self) -> bool {
        0 != (*self as i32) & modifier::PROTECTED
    }

    fn is_static(&self) -> bool {
        0 != (*self as i32) & modifier::STATIC
    }

    fn is_final(&self) -> bool {
        0 != (*self as i32) & modifier::FINAL
    }

    fn is_synchronized(&self) -> bool {
        0 != (*self as i32) & modifier::SYNCHRONIZED
    }

    fn is_volatile(&self) -> bool {
        0 != (*self as i32) & modifier::VOLATILE
    }

    fn is_transient(&self) -> bool {
        0 != (*self as i32) & modifier::TRANSIENT
    }

    fn is_native(&self) -> bool {
        0 != (*self as i32) & modifier::NATIVE
    }

    fn is_interface(&self) -> bool {
        0 != (*self as i32) & modifier::INTERFACE
    }

    fn is_strict(&self) -> bool {
        0 != (*self as i32) & modifier::STRICT
    }

    fn is_implicit(&self) -> bool {
        false
    }

    fn is_synthetic(&self) -> bool {
        0 != (*self as i32) & modifier::SYNTHETIC
    }

    fn is_annotation(&self) -> bool {
        0 != (*self as i32) & modifier::ANNOTATION
    }

    fn is_enum(&self) -> bool {
        0 != (*self as i32) & modifier::ENUM
    }

    fn is_primitive(&self) -> bool {
        false
    }
}
pub trait HasModifier {
    fn modifiers(&self) -> i32;
}
impl<T: HasModifier> Modifier for T {
    fn is_abstract(&self) -> bool {
        self.modifiers().is_abstract()
    }

    fn is_public(&self) -> bool {
        self.modifiers().is_public()
    }

    fn is_private(&self) -> bool {
        self.modifiers().is_private()
    }

    fn is_protected(&self) -> bool {
        self.modifiers().is_protected()
    }

    fn is_static(&self) -> bool {
        self.modifiers().is_static()
    }

    fn is_final(&self) -> bool {
        self.modifiers().is_final()
    }

    fn is_synchronized(&self) -> bool {
        self.modifiers().is_synchronized()
    }

    fn is_volatile(&self) -> bool {
        self.modifiers().is_volatile()
    }

    fn is_transient(&self) -> bool {
        self.modifiers().is_transient()
    }

    fn is_native(&self) -> bool {
        self.modifiers().is_native()
    }

    fn is_interface(&self) -> bool {
        self.modifiers().is_interface()
    }

    fn is_strict(&self) -> bool {
        self.modifiers().is_strict()
    }

    fn is_implicit(&self) -> bool {
        self.modifiers().is_implicit()
    }

    fn is_synthetic(&self) -> bool {
        self.modifiers().is_synthetic()
    }

    fn is_annotation(&self) -> bool {
        self.modifiers().is_annotation()
    }

    fn is_enum(&self) -> bool {
        self.modifiers().is_enum()
    }

    fn is_primitive(&self) -> bool {
        self.modifiers().is_primitive()
    }
}
pub mod modifier {
    pub const PUBLIC: i32 = 0x1;
    pub const PRIVATE: i32 = 0x2;
    pub const PROTECTED: i32 = 0x4;
    pub const STATIC: i32 = 0x8;
    pub const FINAL: i32 = 0x10;
    pub const SYNCHRONIZED: i32 = 0x20;
    pub const VOLATILE: i32 = 0x40;
    pub const TRANSIENT: i32 = 0x80;
    pub const NATIVE: i32 = 0x100;
    pub const INTERFACE: i32 = 0x200;
    pub const ABSTRACT: i32 = 0x400;
    pub const STRICT: i32 = 0x800;
    pub const SYNTHETIC: i32 = 0x1000;
    pub const ANNOTATION: i32 = 0x2000;
    pub const ENUM: i32 = 0x4000;
    pub const IMPLICIT: i32 = 0x4000_0000;
    pub const PRIMITIVE: i32 = 0x8000_0000u32 as i32;
}
pub fn parse_modifiers(access_flags: u16, attributes: &HashMap<PooledStr, Attribute>) -> i32 {
    let mut modifiers = access_flags as i32;
    if let Some(_a) = attributes.get("Synthetic") {
        modifiers |= SYNTHETIC;
    }
    modifiers
}
