use super::{
    symbol::{check_is_unqualified_name, FieldSymbol, MethodSymbol, TypeSymbol},
    Version,
};
use crate::{
    constants::{tag::*, Constant::*, ConstantMethodHandleImpl::*, RawConstant::*},
    parser::Parser,
    symbol::{
        check_is_class_name, check_is_field_descriptor, check_is_field_name,
        check_is_initialization_method_descriptor, check_is_method_descriptor,
        check_is_method_name,
    },
    util::PooledStr,
};
use failure::{Fallible};
use std::{option::Option, rc::Rc};
use ConstantFormatError::*;
pub fn parse_constant_pool(version: Version, parser: &mut Parser<'_>) -> Fallible<Vec<Constant>> {
    let raw_constant_pool = RawConstant::parse_vec(parser)?;
    let mut constant_pool_option: Vec<Option<Constant>> =
        Vec::with_capacity(raw_constant_pool.len());
    for _i in 0..raw_constant_pool.len() {
        constant_pool_option.push(None);
    }
    let _i = 0;
    for i in 0..raw_constant_pool.len() {
        let _c = get_optional_constant(
            version,
            i as u16,
            &raw_constant_pool,
            &mut constant_pool_option,
        )?;
    }
    let mut constant_pool = Vec::with_capacity(constant_pool_option.len());
    constant_pool_option
        .into_iter()
        .enumerate()
        .try_for_each(|(i, constant_option)| {
            if let Some(constant) = constant_option {
                constant_pool.push(constant);
                Ok(())
            } else {
                Err(NotFoundError(i as u16))
            }
        })?;
    Ok(constant_pool)
}
#[derive(Clone, Debug)]
enum RawConstant {
    RawConstantNone,
    RawConstantUtf8(PooledStr),
    RawConstantInteger(i32),
    RawConstantFloat(f32),
    RawConstantLong(i64),
    RawConstantDouble(f64),
    RawConstantClass(u16),
    RawConstantNameAndType(u16, u16),
    RawConstantString(u16),
    RawConstantFieldRef(u16, u16),
    RawConstantMethodRef(u16, u16),
    RawConstantInterfaceMethodRef(u16, u16),
    RawConstantInvokeDynamic(u16, u16),
    RawConstantMethodType(u16),
    RawConstantMethodHandle(u8, u16),
}
fn get_optional_constant(
    version: Version,
    index: u16,
    raw_constant_pool: &Vec<RawConstant>,
    constant_pool_option: &mut Vec<Option<Constant>>,
) -> Fallible<Constant> {
    match constant_pool_option.get(index as usize) {
        None => Err(NotFoundError(index))?,
        Some(c_option) => {
            if let Some(c) = c_option.as_ref() {
                return Ok(c.clone());
            }
        }
    }
    let raw_constant = raw_constant_pool
        .get(index as usize)
        .ok_or(NotFoundError(index))?;
    let constant = match raw_constant {
        RawConstantNone => ConstantNone,
        RawConstantUtf8(s) => ConstantUtf8(s.clone()),
        RawConstantInteger(v) => ConstantInteger(*v),
        RawConstantFloat(v) => ConstantFloat(*v),
        RawConstantLong(v) => ConstantLong(*v),
        RawConstantDouble(v) => ConstantDouble(*v),
        RawConstantClass(name_index) => {
            let name = get_optional_constant(
                version,
                *name_index,
                raw_constant_pool,
                constant_pool_option,
            )?
            .try_as_class_name_in_utf8()?;
            ConstantClass(Rc::new(ConstantClassInfoImpl {
                symbol: TypeSymbol::from_class_name(name.clone())
                    .ok_or(IllegalClassName(name))?,
            }))
        }
        RawConstantNameAndType(name_index, descriptor_index) => {
            let name = get_optional_constant(
                version,
                *name_index,
                raw_constant_pool,
                constant_pool_option,
            )?
            .try_as_utf8()?;
            let descriptor = get_optional_constant(
                version,
                *descriptor_index,
                raw_constant_pool,
                constant_pool_option,
            )?
            .try_as_utf8()?;
            ConstantNameAndType(Rc::new(ConstantNameAndTypeImpl { name, descriptor }))
        }
        RawConstantString(utf8_index) => {
            let value = get_optional_constant(
                version,
                *utf8_index,
                raw_constant_pool,
                constant_pool_option,
            )?
            .try_as_utf8()?;
            ConstantString(value)
        }
        RawConstantFieldRef(class_index, name_and_type_index) => {
            let class = get_optional_constant(
                version,
                *class_index,
                raw_constant_pool,
                constant_pool_option,
            )?
            .try_as_class()?;
            let name_and_type = get_optional_constant(
                version,
                *name_and_type_index,
                raw_constant_pool,
                constant_pool_option,
            )?
            .try_as_name_and_type_of_field()?;
            ConstantFieldRef(Rc::new(ConstantFieldRefImpl {
                class,
                symbol: FieldSymbol::new(
                    name_and_type.name.clone(),
                    name_and_type.descriptor.clone(),
                )
                .ok_or_else(|| {
                    IllegalFieldNameAndType(
                        name_and_type.name.clone(),
                        name_and_type.descriptor.clone(),
                    )
                })?,
            }))
        }
        RawConstantMethodRef(class_index, name_and_type_index) => {
            let class = get_optional_constant(
                version,
                *class_index,
                raw_constant_pool,
                constant_pool_option,
            )?
            .try_as_class()?;
            let name_and_type = get_optional_constant(
                version,
                *name_and_type_index,
                raw_constant_pool,
                constant_pool_option,
            )?
            .try_as_name_and_type_of_method()?;
            ConstantMethodRef(Rc::new(ConstantMethodRefImpl {
                class,
                symbol: MethodSymbol::new(name_and_type.name.clone(), &name_and_type.descriptor)
                    .ok_or_else(|| {
                        IllegalMethodNameAndType(
                            name_and_type.name.clone(),
                            name_and_type.descriptor.clone(),
                        )
                    })?,
            }))
        }
        RawConstantInterfaceMethodRef(class_index, name_and_type_index) => {
            let class = get_optional_constant(
                version,
                *class_index,
                raw_constant_pool,
                constant_pool_option,
            )?
            .try_as_class()?;
            let name_and_type = get_optional_constant(
                version,
                *name_and_type_index,
                raw_constant_pool,
                constant_pool_option,
            )?
            .try_as_name_and_type_of_interface_method()?;
            ConstantInterfaceMethodRef(Rc::new(ConstantInterfaceMethodRefImpl {
                class,
                symbol: MethodSymbol::new_interface(
                    name_and_type.name.clone(),
                    &name_and_type.descriptor,
                )
                .ok_or_else(|| {
                    IllegalInterfaceMethodNameAndType(
                        name_and_type.name.clone(),
                        name_and_type.descriptor.clone(),
                    )
                })?,
            }))
        }
        RawConstantInvokeDynamic(bootstrap_method_attr_index, name_and_type_index) => {
            let name_and_type = get_optional_constant(
                version,
                *name_and_type_index,
                raw_constant_pool,
                constant_pool_option,
            )?
            .try_as_name_and_type_of_method()?;
            ConstantInvokeDynamic(Rc::new(ConstantInvokeDynamicImpl {
                bootstrap_method_attr_index: *bootstrap_method_attr_index,
                name_and_type,
            }))
        }
        RawConstantMethodType(descriptor_index) => {
            let descriptor = get_optional_constant(
                version,
                *descriptor_index,
                raw_constant_pool,
                constant_pool_option,
            )?
            .try_as_method_descriptor()?;
            ConstantMethodType(Rc::new(ConstantMethodTypeImpl { descriptor }))
        }
        RawConstantMethodHandle(tag_ref, descriptor_index) => {
            let tag = *tag_ref;
            let constant = get_optional_constant(
                version,
                *descriptor_index,
                raw_constant_pool,
                constant_pool_option,
            )?;
            let method_impl = match tag {
                REF_GET_FIELD => GetField(constant.try_as_field_ref()?),
                REF_GET_STATIC => GetStatic(constant.try_as_field_ref()?),
                REF_PUT_FIELD => PutField(constant.try_as_field_ref()?),
                REF_PUT_STATIC => PutStatic(constant.try_as_field_ref()?),
                REF_INVOKE_VIRTUAL => {
                    constant.check_not_constructor()?;
                    InvokeVirtual(constant.try_as_method_ref()?)
                }
                REF_NEW_INVOKE_SPECIAL => {
                    constant.check_is_constructor()?;
                    if version.major() >= 52 {
                        constant
                            .try_as_method_ref()
                            .map(InvokeSpecialMethodRef)
                            .or_else(|_| {
                                constant
                                    .try_as_interface_method_ref()
                                    .map(InvokeSpecialInterfaceMethodRef)
                            })?
                    } else {
                        NewInvokeSpecial(constant.try_as_method_ref()?)
                    }
                }
                REF_INVOKE_INTERFACE => {
                    constant.check_not_constructor()?;
                    InvokeInterface(constant.try_as_interface_method_ref()?)
                }
                REF_INVOKE_STATIC | REF_INVOKE_SPECIAL => {
                    constant.check_not_constructor()?;
                    if version.major() >= 52 {
                        constant
                            .try_as_method_ref()
                            .map(InvokeStaticMethodRef)
                            .or_else(|_| {
                                constant
                                    .try_as_interface_method_ref()
                                    .map(InvokeStaticInterfaceMethodRef)
                            })?
                    } else {
                        NewInvokeSpecial(constant.try_as_method_ref()?)
                    }
                }
                _ => Err(IllegalTag(tag))?,
            };
            ConstantMethodHandle(Rc::new(method_impl))
        }
    };
    constant_pool_option.push(Some(constant.clone()));
    constant_pool_option.swap_remove(index as usize);
    Ok(constant)
}
impl RawConstant {
    pub fn parse(parser: &mut Parser<'_>) -> Fallible<RawConstant> {
        let tag = parser.next_u8()?;
        match tag {
            CONSTANT_CLASS => Ok(RawConstantClass(parser.next_u16()?)),
            CONSTANT_FIELD_REF => Ok(RawConstantFieldRef(parser.next_u16()?, parser.next_u16()?)),
            CONSTANT_METHOD_REF => Ok(RawConstantMethodRef(parser.next_u16()?, parser.next_u16()?)),
            CONSTANT_INTERFACE_METHOD_REF => Ok(RawConstantInterfaceMethodRef(
                parser.next_u16()?,
                parser.next_u16()?,
            )),
            CONSTANT_STRING => Ok(RawConstantString(parser.next_u16()?)),
            CONSTANT_INTEGER => Ok(RawConstantInteger(parser.next_i32()?)),
            CONSTANT_FLOAT => Ok(RawConstantFloat(parser.next_f32()?)),
            CONSTANT_LONG => Ok(RawConstantLong(parser.next_i64()?)),
            CONSTANT_DOUBLE => Ok(RawConstantDouble(parser.next_f64()?)),
            CONSTANT_NAME_AND_TYPE => Ok(RawConstantNameAndType(
                parser.next_u16()?,
                parser.next_u16()?,
            )),
            CONSTANT_UTF8 => {
                let length = parser.next_u16()? as usize;
                let vec = parser.next_vec_u8(length)?;
                Ok(RawConstantUtf8(String::from_utf8(vec)?.into()))
            }
            CONSTANT_METHOD_HANDLE => Ok(RawConstantMethodHandle(
                parser.next_u8()?,
                parser.next_u16()?,
            )),
            CONSTANT_METHOD_TYPE => Ok(RawConstantMethodType(parser.next_u16()?)),
            CONSTANT_INVOKE_DYNAMIC => Ok(RawConstantInvokeDynamic(
                parser.next_u16()?,
                parser.next_u16()?,
            )),
            _ => Err(ConstantFormatError::IllegalTag(tag)),
        }
        .map_err(|e| e.into())
    }

    pub fn parse_vec(parser: &mut Parser<'_>) -> Fallible<Vec<RawConstant>> {
        #[derive(Fail, Debug)]
        #[fail(display = "An error occurred.")]
        struct ParseConstantPoolError {
            index: usize,
            #[cause]
            cause: failure::Error,
        }
        let count = parser.next_u16()? as usize;
        let mut vec: Vec<RawConstant> = Vec::with_capacity(count + 1);
        vec.push(RawConstant::RawConstantNone);
        let mut index = 1;
        while index < count {
            let constant = RawConstant::parse(parser)
                .map_err(|cause| ParseConstantPoolError { index, cause })?;
            let wide = match &constant {
                RawConstant::RawConstantLong(_) | RawConstant::RawConstantDouble(_) => true,
                _ => false,
            };
            vec.push(constant);
            if wide {
                vec.push(RawConstant::RawConstantNone);
                index += 2;
            } else {
                index += 1;
            }
        }
        Ok(vec)
    }
}
#[derive(Clone, Debug)]
pub enum Constant {
    ConstantNone,
    ConstantUtf8(PooledStr),
    ConstantInteger(i32),
    ConstantFloat(f32),
    ConstantLong(i64),
    ConstantDouble(f64),
    ConstantClass(Rc<ConstantClassInfoImpl>),
    ConstantNameAndType(Rc<ConstantNameAndTypeImpl>),
    ConstantString(PooledStr),
    ConstantFieldRef(Rc<ConstantFieldRefImpl>),
    ConstantMethodRef(Rc<ConstantMethodRefImpl>),
    ConstantInterfaceMethodRef(Rc<ConstantInterfaceMethodRefImpl>),
    ConstantInvokeDynamic(Rc<ConstantInvokeDynamicImpl>),
    ConstantMethodType(Rc<ConstantMethodTypeImpl>),
    ConstantMethodHandle(Rc<ConstantMethodHandleImpl>),
}
impl Constant {
    pub fn try_as_utf8(&self) -> Fallible<PooledStr> {
        match self {
            ConstantUtf8(s) => Ok(s.clone()),
            _ => Err(TypeError())?,
        }
    }

    pub fn try_as_class(&self) -> Fallible<Rc<ConstantClassInfoImpl>> {
        match self {
            ConstantClass(inner) => Ok(inner.clone()),
            _ => Err(TypeError())?,
        }
    }

    pub fn try_as_name_and_type_index(&self) -> Fallible<Rc<ConstantNameAndTypeImpl>> {
        match self {
            ConstantNameAndType(inner) => Ok(inner.clone()),
            _ => Err(TypeError())?,
        }
    }

    pub fn try_as_interface_method_ref(&self) -> Fallible<Rc<ConstantInterfaceMethodRefImpl>> {
        match self {
            ConstantInterfaceMethodRef(inner) => Ok(inner.clone()),
            _ => Err(TypeError())?,
        }
    }

    pub fn try_as_method_ref(&self) -> Fallible<Rc<ConstantMethodRefImpl>> {
        match self {
            ConstantMethodRef(inner) => Ok(inner.clone()),
            _ => Err(TypeError())?,
        }
    }

    pub fn try_as_field_ref(&self) -> Fallible<Rc<ConstantFieldRefImpl>> {
        match self {
            ConstantFieldRef(inner) => Ok(inner.clone()),
            _ => Err(TypeError())?,
        }
    }

    pub fn check_not_constructor(&self) -> Fallible<()> {
        let symbol = match self {
            ConstantInterfaceMethodRef(inner) => &inner.symbol,
            ConstantMethodRef(inner) => &inner.symbol,
            _ => Err(TypeError())?,
        };
        if !symbol.not_initialization() {
            Err(UnexpectedInitializationMethod())?;
        }
        Ok(())
    }

    pub fn check_is_constructor(&self) -> Fallible<()> {
        let symbol = match self {
            ConstantInterfaceMethodRef(inner) => &inner.symbol,
            ConstantMethodRef(inner) => &inner.symbol,
            _ => Err(TypeError())?,
        };
        if symbol.not_initialization() {
            Err(ExpectedInitializationMethod())?;
        }
        Ok(())
    }

    pub fn try_as_class_name_in_utf8(&self) -> Fallible<PooledStr> {
        let s = self.try_as_utf8()?;
        if check_is_class_name(&*s) {
            Ok(s)
        } else {
            Err(IllegalClassName(s).into())
        }
    }

    pub fn try_as_field_name_in_utf8(&self) -> Fallible<PooledStr> {
        let s = self.try_as_utf8()?;
        if check_is_field_name(&*s) {
            Ok(s)
        } else {
            Err(IllegalClassName(s).into())
        }
    }

    pub fn try_as_method_name_in_utf8(&self) -> Fallible<PooledStr> {
        let s = self.try_as_utf8()?;
        if check_is_method_name(&*s) {
            Ok(s)
        } else {
            Err(IllegalClassName(s).into())
        }
    }

    pub fn try_as_name_and_type_of_field(&self) -> Fallible<Rc<ConstantNameAndTypeImpl>> {
        let name_and_type = self.try_as_name_and_type_index()?;
        if !check_is_field_name(&*name_and_type.name) {
            Err(IllegalFieldName(name_and_type.name.clone()).into())
        } else if !check_is_field_descriptor(&*name_and_type.descriptor) {
            Err(IllegalFieldDescriptor(name_and_type.descriptor.clone()).into())
        } else {
            Ok(name_and_type)
        }
    }

    pub fn try_as_name_and_type_of_method(&self) -> Fallible<Rc<ConstantNameAndTypeImpl>> {
        let name_and_type = self.try_as_name_and_type_index()?;
        if (&*name_and_type.name).bytes().next() == Some(b'<') {
            match &*name_and_type.name {
                "<init>" | "<clinit>" => {
                    if !check_is_initialization_method_descriptor(&*name_and_type.descriptor) {
                        Err(IllegalMethodDescriptor(name_and_type.descriptor.clone()).into())
                    } else {
                        Ok(name_and_type)
                    }
                }
                _ => Err(IllegalMethodName(name_and_type.name.clone()).into()),
            }
        } else if !check_is_method_name(&*name_and_type.name) {
            Err(IllegalMethodName(name_and_type.name.clone()).into())
        } else if !check_is_method_descriptor(&*name_and_type.descriptor) {
            Err(IllegalMethodDescriptor(name_and_type.descriptor.clone()).into())
        } else {
            Ok(name_and_type)
        }
    }

    pub fn try_as_name_and_type_of_interface_method(
        &self,
    ) -> Fallible<Rc<ConstantNameAndTypeImpl>> {
        let name_and_type = self.try_as_name_and_type_index()?;
        if !check_is_method_name(&*name_and_type.name) {
            Err(IllegalMethodName(name_and_type.name.clone()).into())
        } else if !check_is_method_descriptor(&*name_and_type.descriptor) {
            Err(IllegalMethodDescriptor(name_and_type.descriptor.clone()).into())
        } else {
            Ok(name_and_type)
        }
    }

    pub fn try_as_method_descriptor(&self) -> Fallible<PooledStr> {
        let s = self.try_as_utf8()?;
        if check_is_method_descriptor(&*s) {
            Ok(s)
        } else {
            Err(IllegalMethodDescriptor(s).into())
        }
    }

    pub fn try_as_field_descriptor(&self) -> Fallible<PooledStr> {
        let s = self.try_as_utf8()?;
        if check_is_field_descriptor(&*s) {
            Ok(s)
        } else {
            Err(IllegalFieldDescriptor(s).into())
        }
    }

    pub fn try_as_method_handle(&self) -> Fallible<Rc<ConstantMethodHandleImpl>> {
        match self {
            ConstantMethodHandle(inner) => Ok(inner.clone()),
            _ => Err(TypeError().into()),
        }
    }

    pub fn try_as_integer(&self) -> Fallible<i32> {
        match self {
            ConstantInteger(inner) => Ok(*inner),
            _ => Err(TypeError())?,
        }
    }

    pub fn try_as_double(&self) -> Fallible<f64> {
        match self {
            ConstantDouble(inner) => Ok(*inner),
            _ => Err(TypeError())?,
        }
    }

    pub fn try_as_float(&self) -> Fallible<f32> {
        match self {
            ConstantFloat(inner) => Ok(*inner),
            _ => Err(TypeError())?,
        }
    }

    pub fn try_as_long(&self) -> Fallible<i64> {
        match self {
            ConstantLong(inner) => Ok(*inner),
            _ => Err(TypeError())?,
        }
    }

    pub fn try_as_unqualified_name(&self) -> Fallible<PooledStr> {
        let name = self.try_as_utf8()?;
        if check_is_unqualified_name(&*name) {
            Ok(name)
        } else {
            Err(IllegalUnqualifiedName(name).into())
        }
    }
}
#[derive(Clone, Debug)]
pub struct ConstantClassInfoImpl {
    pub symbol: TypeSymbol,
}
#[derive(Clone, Debug)]
pub struct ConstantNameAndTypeImpl {
    pub name: PooledStr,
    pub descriptor: PooledStr,
}
#[derive(Clone, Debug)]
pub struct ConstantFieldRefImpl {
    pub class: Rc<ConstantClassInfoImpl>,
    pub symbol: FieldSymbol,
}
impl ConstantFieldRefImpl {
    pub fn descriptor(&self) -> &PooledStr {
        &self.symbol.descriptor.name
    }

    pub fn name(&self) -> &PooledStr {
        &self.symbol.name
    }
}
#[derive(Clone, Debug)]
pub struct ConstantMethodRefImpl {
    pub class: Rc<ConstantClassInfoImpl>,
    pub symbol: MethodSymbol,
}
#[derive(Clone, Debug)]
pub struct ConstantInterfaceMethodRefImpl {
    pub class: Rc<ConstantClassInfoImpl>,
    pub symbol: MethodSymbol,
}
#[derive(Clone, Debug)]
pub enum ConstantMethodHandleImpl {
    GetField(Rc<ConstantFieldRefImpl>),
    GetStatic(Rc<ConstantFieldRefImpl>),
    PutField(Rc<ConstantFieldRefImpl>),
    PutStatic(Rc<ConstantFieldRefImpl>),

    InvokeVirtual(Rc<ConstantMethodRefImpl>),
    NewInvokeSpecial(Rc<ConstantMethodRefImpl>),

    InvokeStaticMethodRef(Rc<ConstantMethodRefImpl>),
    InvokeStaticInterfaceMethodRef(Rc<ConstantInterfaceMethodRefImpl>),
    InvokeSpecialMethodRef(Rc<ConstantMethodRefImpl>),
    InvokeSpecialInterfaceMethodRef(Rc<ConstantInterfaceMethodRefImpl>),

    InvokeInterface(Rc<ConstantInterfaceMethodRefImpl>),
}
#[derive(Clone, Debug)]
pub struct ConstantMethodTypeImpl {
    pub descriptor: PooledStr,
}
#[derive(Clone, Debug)]
pub struct ConstantInvokeDynamicImpl {
    pub bootstrap_method_attr_index: u16,
    pub name_and_type: Rc<ConstantNameAndTypeImpl>,
}
pub mod tag {
    pub const CONSTANT_CLASS: u8 = 7;
    pub const CONSTANT_FIELD_REF: u8 = 9;
    pub const CONSTANT_METHOD_REF: u8 = 10;
    pub const CONSTANT_INTERFACE_METHOD_REF: u8 = 11;
    pub const CONSTANT_STRING: u8 = 8;
    pub const CONSTANT_INTEGER: u8 = 3;
    pub const CONSTANT_FLOAT: u8 = 4;
    pub const CONSTANT_LONG: u8 = 5;
    pub const CONSTANT_DOUBLE: u8 = 6;
    pub const CONSTANT_NAME_AND_TYPE: u8 = 12;
    pub const CONSTANT_UTF8: u8 = 1;
    pub const CONSTANT_METHOD_HANDLE: u8 = 15;
    pub const CONSTANT_METHOD_TYPE: u8 = 16;
    pub const CONSTANT_INVOKE_DYNAMIC: u8 = 18;
    pub const REF_GET_FIELD: u8 = 1;
    pub const REF_GET_STATIC: u8 = 2;
    pub const REF_PUT_FIELD: u8 = 3;
    pub const REF_PUT_STATIC: u8 = 4;
    pub const REF_INVOKE_VIRTUAL: u8 = 5;
    pub const REF_INVOKE_STATIC: u8 = 6;
    pub const REF_INVOKE_SPECIAL: u8 = 7;
    pub const REF_NEW_INVOKE_SPECIAL: u8 = 8;
    pub const REF_INVOKE_INTERFACE: u8 = 9;
}
pub type TypeName = PooledStr;
#[derive(Fail, Debug)]
pub enum ConstantFormatError {
    #[fail(display = "Constant type Error.")]
    TypeError(),
    #[fail(display = "Constant {} not found", _0)]
    NotFoundError(u16),
    #[fail(display = "Illegal class name \"{}\"", _0)]
    IllegalUnqualifiedName(PooledStr),
    #[fail(display = "Illegal unqualified name \"{}\"", _0)]
    IllegalClassName(PooledStr),
    #[fail(display = "Illegal faild name \"{}\"", _0)]
    IllegalFieldName(PooledStr),
    #[fail(display = "Illegal method name \"{}\"", _0)]
    IllegalMethodName(PooledStr),
    #[fail(display = "Illegal field descriptor \"{}\"", _0)]
    IllegalFieldDescriptor(PooledStr),
    #[fail(display = "Illegal method descriptor \"{}\"", _0)]
    IllegalMethodDescriptor(PooledStr),
    #[fail(display = "Illegal constant format \"{}\"", _0)]
    IllegalFormatError(PooledStr),
    #[fail(display = "Illegal constant tag:{} .", _0)]
    IllegalTag(u8),
    #[fail(display = "Illegal fiald name_and_type \"{:?}\" \"{:?}\"", _0, _1)]
    IllegalFieldNameAndType(PooledStr, PooledStr),
    #[fail(display = "Illegal method name_and_type \"{:?}\" \"{:?}\"", _0, _1)]
    IllegalMethodNameAndType(PooledStr, PooledStr),
    #[fail(
        display = "Illegal interface method name_and_type \"{:?}\" \"{:?}\"",
        _0, _1
    )]
    IllegalInterfaceMethodNameAndType(PooledStr, PooledStr),
    #[fail(display = "Unexected initialization method.")]
    UnexpectedInitializationMethod(),
    #[fail(display = "Exected initialization method.")]
    ExpectedInitializationMethod(),
}
