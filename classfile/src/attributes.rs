use super::{
    constants::{Constant, ConstantMethodHandleImpl},
    parser::Parser,
};
use crate::{
    constants::{Constant::*, *},
    util::PooledStr,
    Version,
};
use failure::{format_err, Fallible};
use std::{collections::HashMap, rc::Rc};
// Attribute:ConstantValue
#[derive(Debug)]
pub struct ConstantValue {
    pub constant_value: Constant,
}
impl ConstantValue {
    fn parse(
        attribute_length: usize,
        parser: &mut Parser<'_>,
        constant_pool: &Vec<Constant>,
    ) -> Fallible<Self> {
        if attribute_length != 2 {
            Err(AttributeFormatError::IllegalFormatError)?;
        };
        let constant = parser.next_constant_index(constant_pool)?;
        match constant {
            ConstantLong(_) => {}
            ConstantFloat(_) => {}
            ConstantDouble(_) => {}
            ConstantInteger(_) => {}
            ConstantString(_) => {}
            _ => Err(AttributeFormatError::IllegalConstant)?,
        };
        Ok(Self {
            constant_value: constant,
        })
    }
}
// Attribute: Code
#[derive(Debug)]
pub struct Code {
    pub max_stack: u16,
    pub max_locals: u16,
    pub code: Vec<u8>,
    pub exception_table: Vec<ExceptionTable>,
    pub attributes: HashMap<PooledStr, Attribute>,
}
impl Code {
    pub fn parse(
        _attribute_length: usize,
        parser: &mut Parser<'_>,
        constant_pool: &Vec<Constant>,
        version: Version,
    ) -> Fallible<Self> {
        let max_stack = parser.next_u16()?;
        let max_locals = parser.next_u16()?;
        let code_len = parser.next_u32()?;
        let code = parser.next_vec_u8(code_len as usize)?;
        let exception_table_length = parser.next_u16()?;
        let mut exception_table = Vec::with_capacity(exception_table_length as usize);
        for _ in 0..exception_table_length {
            exception_table.push(ExceptionTable::parse(parser, constant_pool)?);
        }
        let attributes = Attribute::parse_hashmap(parser, constant_pool, version, Location::Code)?;
        Ok(Self {
            max_stack,
            max_locals,
            code,
            exception_table,
            attributes,
        })
    }
}
#[derive(Debug)]
pub struct ExceptionTable {
    pub start_pc: u16,
    pub end_pc: u16,
    pub handler_pc: u16,
    pub catch_type: Option<Rc<ConstantClassInfoImpl>>,
}
impl ExceptionTable {
    pub fn parse(parser: &mut Parser<'_>, constant_pool: &Vec<Constant>) -> Fallible<Self> {
        Ok(ExceptionTable {
            start_pc: parser.next_u16()?,
            end_pc: parser.next_u16()?,
            handler_pc: parser.next_u16()?,
            catch_type: if let Some(c) = parser.next_constant_index_oprional(constant_pool)? {
                Some(c.try_as_class()?)
            } else {
                None
            },
        })
    }
}
// Attribute:StackMapTable
#[derive(Debug)]
pub struct StackMapTable {
    pub entries: Vec<StackMapFrame>,
}
impl StackMapTable {
    pub fn parse(
        _attribute_length: usize,
        parser: &mut Parser<'_>,
        constant_pool: &Vec<Constant>,
    ) -> Fallible<Self> {
        let len = parser.next_u16()? as usize;
        let mut entries = Vec::with_capacity(len);
        for _ in 0..len {
            entries.push(StackMapFrame::parse(parser, constant_pool)?);
        }
        Ok(Self { entries })
    }
}
#[derive(Debug)]
pub enum StackMapFrame {
    SameFrame {
        frame_type: u8,
    },
    SameLocals1StackItemFrame {
        frame_type: u8,
        stack: VerificationTypeInfo,
    },
    SameLocals1StackItemFrameExtended {
        frame_type: u8,
        offset_delta: u16,
        stack: VerificationTypeInfo,
    },
    ChopFrame {
        frame_type: u8,
        offset_delta: u16,
    },
    SameFrameExtended {
        offset_delta: u16,
    },
    AppendFrame {
        frame_type: u8,
        offset_delta: u16,
        locals: Vec<VerificationTypeInfo>,
    },
    FullFrame {
        offset_delta: u16,
        locals: Vec<VerificationTypeInfo>,
        stack: Vec<VerificationTypeInfo>,
    },
}
impl StackMapFrame {
    pub fn parse(parser: &mut Parser<'_>, constant_pool: &Vec<Constant>) -> Fallible<Self> {
        let frame_type = parser.next_u8()?;
        Ok(match frame_type {
            0..=63 => StackMapFrame::SameFrame { frame_type },
            64..=127 => StackMapFrame::SameLocals1StackItemFrame {
                frame_type,
                stack: VerificationTypeInfo::parse(parser, constant_pool)?,
            },
            247 => StackMapFrame::SameLocals1StackItemFrameExtended {
                frame_type,
                offset_delta: parser.next_u16()?,
                stack: VerificationTypeInfo::parse(parser, constant_pool)?,
            },
            248..=250 => StackMapFrame::ChopFrame {
                frame_type,
                offset_delta: parser.next_u16()?,
            },
            251 => StackMapFrame::SameFrameExtended {
                offset_delta: parser.next_u16()?,
            },
            252..=254 => {
                let offset_delta = parser.next_u16()?;
                let len = frame_type - 251;
                let mut locals = Vec::with_capacity(len as usize);
                for _ in 0..len {
                    locals.push(VerificationTypeInfo::parse(parser, constant_pool)?);
                }
                StackMapFrame::AppendFrame {
                    frame_type,
                    offset_delta,
                    locals,
                }
            }
            255 => {
                let offset_delta = parser.next_u16()?;
                let locals_len = parser.next_u16()?;
                let mut locals = Vec::with_capacity(locals_len as usize);
                for _ in 0..locals_len {
                    locals.push(VerificationTypeInfo::parse(parser, constant_pool)?);
                }
                let stack_item_len = parser.next_u16()?;
                let mut stack = Vec::with_capacity(stack_item_len as usize);
                for _ in 0..stack_item_len {
                    stack.push(VerificationTypeInfo::parse(parser, constant_pool)?);
                }
                StackMapFrame::FullFrame {
                    offset_delta,
                    locals,
                    stack,
                }
            }
            128..=246 => Err(AttributeFormatError::IllegalFormatError)?,
        })
    }

    pub fn get_offset_delta(&self) -> u16 {
        match self {
            StackMapFrame::SameFrame { frame_type } => *frame_type as u16,
            StackMapFrame::SameLocals1StackItemFrame { frame_type, .. } => *frame_type as u16 - 64,
            StackMapFrame::SameLocals1StackItemFrameExtended { offset_delta, .. }
            | StackMapFrame::ChopFrame { offset_delta, .. }
            | StackMapFrame::SameFrameExtended { offset_delta }
            | StackMapFrame::AppendFrame { offset_delta, .. }
            | StackMapFrame::FullFrame { offset_delta, .. } => *offset_delta,
        }
    }
}
#[derive(Debug)]
pub enum VerificationTypeInfo {
    TopVariableInfo,
    IntegerVariableInfo,
    LongVariableInfo,
    FloatVariableInfo,
    DoubleVariableInfo,
    NullVariableInfo,
    UninitializedThisVariableInfo,
    ObjectVariableInfo(Rc<ConstantClassInfoImpl>),
    UninitializedVariableInfo(u16),
}
pub mod stack_map {
    pub const ITEM_TOP: u8 = 0;
    pub const ITEM_INTEGER: u8 = 1;
    pub const ITEM_FLOAT: u8 = 2;
    pub const ITEM_NULL: u8 = 5;
    pub const ITEM_UNINITIALIZED_THIS: u8 = 6;
    pub const ITEM_OBJECT: u8 = 7;
    pub const ITEM_UNINITIALIZED: u8 = 8;
    pub const ITEM_LONG: u8 = 4;
    pub const ITEM_DOUBLE: u8 = 3;
}
use stack_map::*;
impl VerificationTypeInfo {
    pub fn parse(parser: &mut Parser<'_>, constant_pool: &Vec<Constant>) -> Fallible<Self> {
        let tag = parser.next_u8()?;
        Ok(match tag {
            ITEM_TOP => VerificationTypeInfo::TopVariableInfo,
            ITEM_INTEGER => VerificationTypeInfo::IntegerVariableInfo,
            ITEM_FLOAT => VerificationTypeInfo::FloatVariableInfo,
            ITEM_NULL => VerificationTypeInfo::NullVariableInfo,
            ITEM_UNINITIALIZED_THIS => VerificationTypeInfo::UninitializedThisVariableInfo,
            ITEM_OBJECT => VerificationTypeInfo::ObjectVariableInfo(
                parser.next_constant_index(constant_pool)?.try_as_class()?,
            ),
            ITEM_UNINITIALIZED => {
                VerificationTypeInfo::UninitializedVariableInfo(parser.next_u16()?)
            }
            ITEM_LONG => VerificationTypeInfo::LongVariableInfo,
            ITEM_DOUBLE => VerificationTypeInfo::DoubleVariableInfo,
            _ => Err(AttributeFormatError::IllegalFormatError)?,
        })
    }
}
// Attribute:Exception
#[derive(Debug)]
pub struct Exception {
    pub exception_table: Vec<Rc<ConstantClassInfoImpl>>,
}
impl Exception {
    pub fn parse(
        _attribute_length: usize,
        parser: &mut Parser<'_>,
        constant_pool: &Vec<Constant>,
    ) -> Fallible<Self> {
        let len = parser.next_u16()? as usize;
        let mut exception_table = Vec::with_capacity(len);
        for _ in 0..len {
            exception_table.push(parser.next_constant_index(constant_pool)?.try_as_class()?);
        }
        Ok(Self { exception_table })
    }
}
// Attribute:InnerClasses
#[derive(Debug)]
pub struct InnerClasses {
    pub calsses: Vec<InnerClass>,
}
impl InnerClasses {
    pub fn parse(
        _attribute_length: usize,
        parser: &mut Parser<'_>,
        constant_pool: &Vec<Constant>,
    ) -> Fallible<Self> {
        let len = parser.next_u16()? as usize;
        let mut calsses = Vec::with_capacity(len);
        for _ in 0..len {
            calsses.push(InnerClass::parse(parser, constant_pool)?);
        }
        Ok(Self { calsses })
    }
}
#[derive(Debug)]
pub struct InnerClass {
    pub inner_class_info: Rc<ConstantClassInfoImpl>,
    pub outer_class_info: Option<Rc<ConstantClassInfoImpl>>,
    pub inner_name: Option<PooledStr>,
    pub inner_class_access_flags: u16,
}
impl InnerClass {
    pub fn parse(parser: &mut Parser<'_>, constant_pool: &Vec<Constant>) -> Fallible<Self> {
        let inner_class_info = parser.next_constant_index(constant_pool)?.try_as_class()?;
        let outer_class_info = match parser.next_u16()? {
            0 => None,
            i => Some(
                constant_pool
                    .get(i as usize)
                    .ok_or_else(|| format_err!("NoneError"))?
                    .try_as_class()?,
            ),
        };
        let inner_name = match parser.next_u16()? {
            0 => None,
            i => Some(
                constant_pool
                    .get(i as usize)
                    .ok_or_else(|| format_err!("NoneError"))?
                    .try_as_utf8()?,
            ),
        };
        let inner_class_access_flags = parser.next_u16()?;
        Ok(Self {
            inner_class_info,
            outer_class_info,
            inner_name,
            inner_class_access_flags,
        })
    }
}
// Attribute:EnclosingMethod
#[derive(Debug)]
pub struct EnclosingMethod {
    pub class: Rc<ConstantClassInfoImpl>,
    pub method: Option<Rc<ConstantNameAndTypeImpl>>,
}
impl EnclosingMethod {
    pub fn parse(
        _attribute_length: usize,
        parser: &mut Parser<'_>,
        constant_pool: &Vec<Constant>,
    ) -> Fallible<Self> {
        let class = parser.next_constant_index(constant_pool)?.try_as_class()?;
        let method = match parser.next_u16()? {
            0 => None,
            i => Some(
                constant_pool
                    .get(i as usize)
                    .ok_or_else(|| format_err!("NoneError"))?
                    .try_as_name_and_type_of_method()?,
            ),
        };
        Ok(Self { class, method })
    }
}
// Attribute:Synthetic
#[derive(Debug)]
pub struct Synthetic {}
impl Synthetic {
    pub fn parse(
        _attribute_length: usize,
        _parser: &mut Parser<'_>,
        _constant_pool: &Vec<Constant>,
    ) -> Fallible<Self> {
        Ok(Self {})
    }
}
// Attribute:Signature
#[derive(Debug)]
pub struct Signature {
    pub signature: PooledStr,
}
impl Signature {
    pub fn parse(
        _attribute_length: usize,
        parser: &mut Parser<'_>,
        constant_pool: &Vec<Constant>,
    ) -> Fallible<Self> {
        Ok(Self {
            signature: parser.next_constant_index(constant_pool)?.try_as_utf8()?,
        })
    }
}
// Attribute:SourceFile
#[derive(Debug)]
pub struct SourceFile {
    pub source_file: PooledStr,
}
impl SourceFile {
    pub fn parse(
        _attribute_length: usize,
        parser: &mut Parser<'_>,
        constant_pool: &Vec<Constant>,
    ) -> Fallible<Self> {
        Ok(Self {
            source_file: parser.next_constant_index(constant_pool)?.try_as_utf8()?,
        })
    }
}
// Attribute:SourceDebugExtension
#[derive(Debug)]
pub struct SourceDebugExtension {
    pub debug_extension: Vec<u8>,
}
impl SourceDebugExtension {
    pub fn parse(
        attribute_length: usize,
        parser: &mut Parser<'_>,
        _constant_pool: &Vec<Constant>,
    ) -> Fallible<Self> {
        Ok(Self {
            debug_extension: parser.next_vec_u8(attribute_length)?,
        })
    }
}
// Attribute:LineNumberTable
#[derive(Debug)]
pub struct LineNumberTable {
    pub line_numbers: Vec<LineNumber>,
}
impl LineNumberTable {
    pub fn parse(
        _attribute_length: usize,
        parser: &mut Parser<'_>,
        constant_pool: &Vec<Constant>,
    ) -> Fallible<Self> {
        let len = parser.next_u16()? as usize;
        let mut line_numbers = Vec::with_capacity(len);
        for _ in 0..len {
            line_numbers.push(LineNumber::parse(parser, constant_pool)?);
        }
        Ok(Self { line_numbers })
    }

    pub fn merge(&mut self, other: Self) {
        self.line_numbers.extend(other.line_numbers)
    }
}
#[derive(Debug)]
pub struct LineNumber {
    pub start_pc: u16,
    pub line_number: u16,
}
impl LineNumber {
    pub fn parse(parser: &mut Parser<'_>, _constant_pool: &Vec<Constant>) -> Fallible<Self> {
        Ok(Self {
            start_pc: parser.next_u16()?,
            line_number: parser.next_u16()?,
        })
    }
}
// Attribute:LocalVariableTable
#[derive(Debug)]
pub struct LocalVariableTable {
    pub local_variables: Vec<LocalVariable>,
}
impl LocalVariableTable {
    pub fn parse(
        _attribute_length: usize,
        parser: &mut Parser<'_>,
        constant_pool: &Vec<Constant>,
    ) -> Fallible<Self> {
        let len = parser.next_u16()? as usize;
        let mut local_variables = Vec::with_capacity(len);
        for _ in 0..len {
            local_variables.push(LocalVariable::parse(parser, constant_pool)?);
        }
        Ok(Self { local_variables })
    }

    pub fn merge(&mut self, other: Self) {
        self.local_variables.extend(other.local_variables)
    }
}
#[derive(Debug)]
pub struct LocalVariable {
    pub start_pc: u16,
    pub length: u16,
    pub name: PooledStr,
    pub descriptor: PooledStr,
    pub index: u16,
}
impl LocalVariable {
    pub fn parse(parser: &mut Parser<'_>, constant_pool: &Vec<Constant>) -> Fallible<Self> {
        Ok(Self {
            start_pc: parser.next_u16()?,
            length: parser.next_u16()?,
            name: parser
                .next_constant_index(constant_pool)?
                .try_as_unqualified_name()?,
            descriptor: parser
                .next_constant_index(constant_pool)?
                .try_as_field_descriptor()?,
            index: parser.next_u16()?,
        })
    }
}
// Attribute:LocalVariableTypeTable
#[derive(Debug)]
pub struct LocalVariableTypeTable {
    pub local_variable_types: Vec<LocalVariableType>,
}
impl LocalVariableTypeTable {
    pub fn parse(
        _attribute_length: usize,
        parser: &mut Parser<'_>,
        constant_pool: &Vec<Constant>,
    ) -> Fallible<Self> {
        let len = parser.next_u16()? as usize;
        let mut local_variable_types = Vec::with_capacity(len);
        for _ in 0..len {
            local_variable_types.push(LocalVariableType::parse(parser, constant_pool)?);
        }
        Ok(Self {
            local_variable_types,
        })
    }

    pub fn merge(&mut self, other: Self) {
        self.local_variable_types.extend(other.local_variable_types)
    }
}
#[derive(Debug)]
pub struct LocalVariableType {
    pub start_pc: u16,
    pub length: u16,
    pub name: PooledStr,
    pub descriptor: PooledStr,
    pub index: u16,
}
impl LocalVariableType {
    pub fn parse(parser: &mut Parser<'_>, constant_pool: &Vec<Constant>) -> Fallible<Self> {
        Ok(Self {
            start_pc: parser.next_u16()?,
            length: parser.next_u16()?,
            name: parser
                .next_constant_index(constant_pool)?
                .try_as_unqualified_name()?,
            descriptor: parser.next_constant_index(constant_pool)?.try_as_utf8()?,
            index: parser.next_u16()?,
        })
    }
}
// Attribute:Deprecated
#[derive(Debug)]
pub struct Deprecated {}
impl Deprecated {
    pub fn parse(
        _attribute_length: usize,
        _parser: &mut Parser<'_>,
        _constant_pool: &Vec<Constant>,
    ) -> Fallible<Self> {
        Ok(Self {})
    }
}
// Attribute: RuntimeVisibleAnnotations RuntimeInvisibleAnnotations
#[derive(Debug)]
pub struct RuntimeVisibleAnnotations {
    pub annotations: Vec<Annotation>,
}
impl RuntimeVisibleAnnotations {
    pub fn parse(
        _attribute_length: usize,
        parser: &mut Parser<'_>,
        constant_pool: &Vec<Constant>,
    ) -> Fallible<Self> {
        let len = parser.next_u16()? as usize;
        let mut annotations = Vec::with_capacity(len);
        for _ in 0..len {
            annotations.push(Annotation::parse(parser, constant_pool)?);
        }
        Ok(Self { annotations })
    }
}
#[derive(Debug)]
pub struct RuntimeInvisibleAnnotations {
    pub annotations: Vec<Annotation>,
}
impl RuntimeInvisibleAnnotations {
    pub fn parse(
        _attribute_length: usize,
        parser: &mut Parser<'_>,
        constant_pool: &Vec<Constant>,
    ) -> Fallible<Self> {
        let len = parser.next_u16()? as usize;
        let mut annotations = Vec::with_capacity(len);
        for _ in 0..len {
            annotations.push(Annotation::parse(parser, constant_pool)?);
        }
        Ok(Self { annotations })
    }
}
#[derive(Debug)]
pub struct Annotation {
    pub type_descriptor: PooledStr,
    pub elements: Vec<(PooledStr, ElementValue)>,
}
impl Annotation {
    pub fn parse(parser: &mut Parser<'_>, constant_pool: &Vec<Constant>) -> Fallible<Self> {
        let type_descriptor = parser.next_constant_index(constant_pool)?.try_as_utf8()?;
        let len = parser.next_u16()? as usize;
        let mut elements = Vec::with_capacity(len);
        for _ in 0..len {
            let element_name = parser.next_constant_index(constant_pool)?.try_as_utf8()?;
            let value = ElementValue::parse(parser, constant_pool)?;
            elements.push((element_name, value));
        }
        Ok(Self {
            type_descriptor,
            elements,
        })
    }
}
#[derive(Debug)]
pub enum ElementValue {
    ConstValue(u8, Constant),
    EnumConstValue {
        type_name: PooledStr,
        const_name: PooledStr,
    },
    ClassInfo(PooledStr),
    AnnotationValue(Box<Annotation>),
    ArrayValue(Vec<ElementValue>),
}
impl ElementValue {
    pub fn parse(parser: &mut Parser<'_>, constant_pool: &Vec<Constant>) -> Fallible<Self> {
        let tag = parser.next_u8()?;
        Ok(match tag {
            b'B' => ElementValue::ConstValue(
                tag,
                ConstantInteger(
                    parser
                        .next_constant_index(constant_pool)?
                        .try_as_integer()?,
                ),
            ),
            b'C' => ElementValue::ConstValue(
                tag,
                ConstantInteger(
                    parser
                        .next_constant_index(constant_pool)?
                        .try_as_integer()?,
                ),
            ),
            b'D' => ElementValue::ConstValue(
                tag,
                ConstantDouble(parser.next_constant_index(constant_pool)?.try_as_double()?),
            ),
            b'F' => ElementValue::ConstValue(
                tag,
                ConstantFloat(parser.next_constant_index(constant_pool)?.try_as_float()?),
            ),
            b'I' => ElementValue::ConstValue(
                tag,
                ConstantInteger(
                    parser
                        .next_constant_index(constant_pool)?
                        .try_as_integer()?,
                ),
            ),
            b'J' => ElementValue::ConstValue(
                tag,
                ConstantLong(parser.next_constant_index(constant_pool)?.try_as_long()?),
            ),
            b'S' => ElementValue::ConstValue(
                tag,
                ConstantInteger(
                    parser
                        .next_constant_index(constant_pool)?
                        .try_as_integer()?,
                ),
            ),
            b'Z' => ElementValue::ConstValue(
                tag,
                ConstantInteger(
                    parser
                        .next_constant_index(constant_pool)?
                        .try_as_integer()?,
                ),
            ),
            b's' => ElementValue::ConstValue(
                tag,
                ConstantString(parser.next_constant_index(constant_pool)?.try_as_utf8()?),
            ),
            b'e' => ElementValue::EnumConstValue {
                type_name: parser.next_constant_index(constant_pool)?.try_as_utf8()?,
                const_name: parser.next_constant_index(constant_pool)?.try_as_utf8()?,
            },
            b'c' => {
                ElementValue::ClassInfo(parser.next_constant_index(constant_pool)?.try_as_utf8()?)
            }
            b'@' => {
                ElementValue::AnnotationValue(Box::new(Annotation::parse(parser, constant_pool)?))
            }
            b'[' => {
                let len = parser.next_u16()? as usize;
                let mut vec = Vec::with_capacity(len);
                for _ in 0..len {
                    vec.push(ElementValue::parse(parser, constant_pool)?);
                }
                ElementValue::ArrayValue(vec)
            }
            _ => Err(AttributeFormatError::IllegalFormatError)?,
        })
    }
}
// Attribute:RuntimeVisibleParameterAnnotations RuntimeInvisibleParameterAnnotations
#[derive(Debug)]
pub struct RuntimeVisibleParameterAnnotations {
    pub parameters_annotation: Vec<Vec<Annotation>>,
}
impl RuntimeVisibleParameterAnnotations {
    pub fn parse(
        _attribute_length: usize,
        parser: &mut Parser<'_>,
        constant_pool: &Vec<Constant>,
    ) -> Fallible<Self> {
        let len = parser.next_u8()? as usize;
        let mut parameters_annotation = Vec::with_capacity(len);
        for _ in 0..len {
            let len_inner = parser.next_u16()? as usize;
            let mut vec = Vec::with_capacity(len);
            for _ in 0..len_inner {
                vec.push(Annotation::parse(parser, constant_pool)?);
            }
            parameters_annotation.push(vec);
        }
        Ok(Self {
            parameters_annotation,
        })
    }
}
#[derive(Debug)]
pub struct RuntimeInvisibleParameterAnnotations {
    pub parameters_annotation: Vec<Vec<Annotation>>,
}
impl RuntimeInvisibleParameterAnnotations {
    pub fn parse(
        _attribute_length: usize,
        parser: &mut Parser<'_>,
        constant_pool: &Vec<Constant>,
    ) -> Fallible<Self> {
        let len = parser.next_u16()? as usize;
        let mut parameters_annotation = Vec::with_capacity(len);
        for _ in 0..len {
            let len_inner = parser.next_u16()? as usize;
            let mut vec = Vec::with_capacity(len);
            for _ in 0..len_inner {
                vec.push(Annotation::parse(parser, constant_pool)?);
            }
            parameters_annotation.push(vec);
        }
        Ok(Self {
            parameters_annotation,
        })
    }
}
// Attribute:RuntimeVisibleTypeAnnotations RuntimeInvisibleTypeAnnotations
#[derive(Debug)]
pub struct RuntimeVisibleTypeAnnotations {
    pub annotations: Vec<TypeAnnotation>,
}
impl RuntimeVisibleTypeAnnotations {
    pub fn parse(
        _attribute_length: usize,
        parser: &mut Parser<'_>,
        constant_pool: &Vec<Constant>,
    ) -> Fallible<Self> {
        let len = parser.next_u16()? as usize;
        let mut annotations = Vec::with_capacity(len);
        for _ in 0..len {
            annotations.push(TypeAnnotation::parse(parser, constant_pool)?);
        }
        Ok(Self { annotations })
    }
}
#[derive(Debug)]
pub struct RuntimeInvisibleTypeAnnotations {
    pub annotations: Vec<TypeAnnotation>,
}
impl RuntimeInvisibleTypeAnnotations {
    pub fn parse(
        _attribute_length: usize,
        parser: &mut Parser<'_>,
        constant_pool: &Vec<Constant>,
    ) -> Fallible<Self> {
        let len = parser.next_u16()? as usize;
        let mut annotations = Vec::with_capacity(len);
        for _ in 0..len {
            annotations.push(TypeAnnotation::parse(parser, constant_pool)?);
        }
        Ok(Self { annotations })
    }
}
#[derive(Debug)]
pub struct LocalVarEntity {
    pub start_pc: u16,
    pub length: u16,
    pub index: u16,
}
impl LocalVarEntity {
    pub fn parse(parser: &mut Parser<'_>, _constant_pool: &Vec<Constant>) -> Fallible<Self> {
        Ok(Self {
            start_pc: parser.next_u16()?,
            length: parser.next_u16()?,
            index: parser.next_u16()?,
        })
    }
}
#[derive(Debug)]
pub enum TargetInfo {
    TypeParameterTarget {
        target_type: u8,
        type_parameter_index: u8,
    },
    SupertypeTarget {
        target_type: u8,
        supertype_index: u16,
    },
    TypeParameterBoundTarget {
        target_type: u8,
        type_parameter_index: u8,
        bound_index: u8,
    },
    EmptyTarget {
        target_type: u8,
    },
    FormalParameterTarget {
        target_type: u8,
        formal_parameter_index: u8,
    },
    ThrowsTarget {
        target_type: u8,
        throws_type_index: u16,
    },
    LocalvarTarget {
        target_type: u8,
        table: Vec<LocalVarEntity>,
    },
    CatchTarget {
        target_type: u8,
        exception_table_index: u16,
    },
    OffsetTarget {
        target_type: u8,
        offset: u16,
    },
    TypeArgumentTarget {
        target_type: u8,
        offset: u16,
        type_parameter_index: u8,
    },
}
impl TargetInfo {
    pub fn parse(parser: &mut Parser<'_>, constant_pool: &Vec<Constant>) -> Fallible<Self> {
        let target_type = parser.next_u8()?;
        Ok(match target_type {
            0x00 | 0x01 => TargetInfo::TypeParameterTarget {
                target_type,
                type_parameter_index: parser.next_u8()?,
            },
            0x10 => TargetInfo::SupertypeTarget {
                target_type,
                supertype_index: parser.next_u16()?,
            },
            0x11 | 0x12 => TargetInfo::TypeParameterBoundTarget {
                target_type,
                type_parameter_index: parser.next_u8()?,
                bound_index: parser.next_u8()?,
            },
            0x13 | 0x14 | 0x15 => TargetInfo::EmptyTarget { target_type },
            0x16 => TargetInfo::FormalParameterTarget {
                target_type,
                formal_parameter_index: parser.next_u8()?,
            },
            0x17 => TargetInfo::ThrowsTarget {
                target_type,
                throws_type_index: parser.next_u16()?,
            },
            0x40 | 0x41 => {
                let len = parser.next_u16()? as usize;
                let mut table = Vec::with_capacity(len);
                for _ in 0..len {
                    table.push(LocalVarEntity::parse(parser, constant_pool)?);
                }
                TargetInfo::LocalvarTarget { target_type, table }
            }
            0x42 => TargetInfo::CatchTarget {
                target_type,
                exception_table_index: parser.next_u16()?,
            },
            0x43 | 0x44 | 0x45 | 0x46 => TargetInfo::OffsetTarget {
                target_type,
                offset: parser.next_u16()?,
            },
            0x37 | 0x48 | 0x49 | 0x4a | 0x4b => TargetInfo::TypeArgumentTarget {
                target_type,
                offset: parser.next_u16()?,
                type_parameter_index: parser.next_u8()?,
            },

            _ => Err(AttributeFormatError::IllegalFormatError)?,
        })
    }
}
#[derive(Debug)]
pub struct PathEntity {
    pub type_path_kind: u8,
    pub type_argument_index: u8,
}
impl PathEntity {
    pub fn parse(parser: &mut Parser<'_>, _constant_pool: &Vec<Constant>) -> Fallible<Self> {
        Ok(Self {
            type_path_kind: parser.next_u8()?,
            type_argument_index: parser.next_u8()?,
        })
    }
}
#[derive(Debug)]
pub struct TargetPath {
    pub path: Vec<PathEntity>,
}
impl TargetPath {
    pub fn parse(parser: &mut Parser<'_>, constant_pool: &Vec<Constant>) -> Fallible<Self> {
        let len = parser.next_u8()? as usize;
        let mut path = Vec::with_capacity(len);
        for _ in 0..len {
            path.push(PathEntity::parse(parser, constant_pool)?);
        }
        Ok(Self { path })
    }
}
#[derive(Debug)]
pub struct TypeAnnotation {
    pub target_info: TargetInfo,
    pub target_path: TargetPath,
    pub annotation: Annotation,
}
impl TypeAnnotation {
    pub fn parse(parser: &mut Parser<'_>, constant_pool: &Vec<Constant>) -> Fallible<Self> {
        Ok(Self {
            target_info: TargetInfo::parse(parser, constant_pool)?,
            target_path: TargetPath::parse(parser, constant_pool)?,
            annotation: Annotation::parse(parser, constant_pool)?,
        })
    }
}
// Attribute:AnnotationDefault
#[derive(Debug)]
pub struct AnnotationDefault {
    pub default_value: ElementValue,
}
impl AnnotationDefault {
    pub fn parse(
        _attribute_length: usize,
        parser: &mut Parser<'_>,
        constant_pool: &Vec<Constant>,
    ) -> Fallible<Self> {
        Ok(Self {
            default_value: ElementValue::parse(parser, constant_pool)?,
        })
    }
}
// Attribute:BootstrapMethods
#[derive(Debug)]
pub struct BootstrapMethods {
    pub bootstrap_methods: Vec<BootstrapMethod>,
}
impl BootstrapMethods {
    pub fn parse(
        _attribute_length: usize,
        parser: &mut Parser<'_>,
        constant_pool: &Vec<Constant>,
    ) -> Fallible<Self> {
        let len = parser.next_u16()? as usize;
        let mut bootstrap_methods = Vec::with_capacity(len);
        for _ in 0..len {
            bootstrap_methods.push(BootstrapMethod::parse(parser, constant_pool)?);
        }
        Ok(Self { bootstrap_methods })
    }
}
#[derive(Debug)]
pub struct BootstrapMethod {
    pub bootstrap_method_ref: Rc<ConstantMethodHandleImpl>,
    pub bootstrap_arguments: Vec<Constant>,
}
impl BootstrapMethod {
    pub fn parse(parser: &mut Parser<'_>, constant_pool: &Vec<Constant>) -> Fallible<Self> {
        let bootstrap_method_ref = parser
            .next_constant_index(constant_pool)?
            .try_as_method_handle()?;
        let len = parser.next_u16()? as usize;
        let mut bootstrap_arguments = Vec::with_capacity(len);
        for _ in 0..len {
            let constant = parser.next_constant_index(constant_pool)?;
            match constant {
                ConstantString(_) => {}
                ConstantClass(_) => {}
                ConstantInteger(_) => {}
                ConstantLong(_) => {}
                ConstantFloat(_) => {}
                ConstantDouble(_) => {}
                ConstantMethodHandle(_) => {}
                ConstantMethodType(_) => {}
                _ => Err(AttributeFormatError::IllegalConstant)?,
            }
            bootstrap_arguments.push(constant);
        }
        Ok(Self {
            bootstrap_arguments,
            bootstrap_method_ref,
        })
    }
}
// Attribute:MethodParameters
#[derive(Debug)]
pub struct MethodParameters {
    pub parameters: Vec<MethodParameter>,
}
impl MethodParameters {
    pub fn parse(
        _attribute_length: usize,
        parser: &mut Parser<'_>,
        constant_pool: &Vec<Constant>,
    ) -> Fallible<Self> {
        let len = parser.next_u8()?;
        let mut vec = Vec::with_capacity(len as usize);
        for _ in 0..len {
            vec.push(MethodParameter::parse(parser, constant_pool)?);
        }
        Ok(Self { parameters: vec })
    }
}
#[derive(Debug)]
pub struct MethodParameter {
    pub name: PooledStr,
    pub access_flags: u16,
}
impl MethodParameter {
    pub fn parse(parser: &mut Parser<'_>, constant_pool: &Vec<Constant>) -> Fallible<Self> {
        Ok(Self {
            name: parser.next_constant_index(constant_pool)?.try_as_utf8()?,
            access_flags: parser.next_u16()?,
        })
    }
}
// enum:Attribute
#[derive(Debug)]
pub enum Attribute {
    ConstantValue(ConstantValue),
    Code(Box<Code>),
    StackMapTable(StackMapTable),
    Exception(Exception),
    InnerClasses(InnerClasses),
    EnclosingMethod(EnclosingMethod),
    Synthetic(Synthetic),
    Signature(Signature),
    SourceFile(SourceFile),
    SourceDebugExtension(SourceDebugExtension),
    LineNumberTable(LineNumberTable),
    LocalVariableTable(LocalVariableTable),
    LocalVariableTypeTable(LocalVariableTypeTable),
    Deprecated(Deprecated),
    RuntimeVisibleAnnotations(RuntimeVisibleAnnotations),
    RuntimeInvisibleAnnotations(RuntimeInvisibleAnnotations),
    RuntimeVisibleParameterAnnotations(RuntimeVisibleParameterAnnotations),
    RuntimeInvisibleParameterAnnotations(RuntimeInvisibleParameterAnnotations),
    RuntimeVisibleTypeAnnotations(RuntimeVisibleTypeAnnotations),
    RuntimeInvisibleTypeAnnotations(RuntimeInvisibleTypeAnnotations),
    AnnotationDefault(AnnotationDefault),
    BootstrapMethods(BootstrapMethods),
    MethodParameters(MethodParameters),
    UnknownAttribute,
}
#[derive(Debug, Clone, Copy)]
pub enum Location {
    ClassFile,
    Field,
    Method,
    Code,
}
use Location::*;
impl Attribute {
    pub fn parse_hashmap(
        parser: &mut Parser<'_>,
        constant_pool: &Vec<Constant>,
        version: Version,
        location: Location,
    ) -> Fallible<HashMap<PooledStr, Attribute>> {
        let mut map = HashMap::new();
        let attributes_count = parser.next_u16()? as usize;
        for _ in 0..attributes_count {
            let name = parser.next_constant_index(constant_pool)?.try_as_utf8()?;
            let length = parser.next_u32()? as usize;
            let info = &mut parser.take(length as usize)?;
            let attribute = match (&*name, location, version) {
                ("ConstantValue", Field, v) if v >= Version(45, 3) => {
                    Attribute::ConstantValue(ConstantValue::parse(length, info, constant_pool)?)
                }
                ("Code", Method, v) if v >= Version(45, 3) => {
                    Attribute::Code(Box::new(Code::parse(length, info, constant_pool, version)?))
                }
                ("StackMapTable", Code, v) if v >= Version(50, 0) => {
                    Attribute::StackMapTable(StackMapTable::parse(length, info, constant_pool)?)
                }
                ("Exceptions", Method, v) if v >= Version(45, 3) => {
                    Attribute::Exception(Exception::parse(length, info, constant_pool)?)
                }
                ("InnerClasses", ClassFile, v) if v >= Version(45, 3) => {
                    Attribute::InnerClasses(InnerClasses::parse(length, info, constant_pool)?)
                }
                ("EnclosingMethod", ClassFile, v) if v >= Version(49, 0) => {
                    Attribute::EnclosingMethod(EnclosingMethod::parse(length, info, constant_pool)?)
                }
                ("Synthetic", ClassFile | Field | Method, v) if v >= Version(45, 3) => {
                    Attribute::Synthetic(Synthetic::parse(length, info, constant_pool)?)
                }
                ("Signature", ClassFile | Field | Method, v) if v >= Version(49, 0) => {
                    Attribute::Signature(Signature::parse(length, info, constant_pool)?)
                }
                ("SourceFile", ClassFile, v) if v >= Version(45, 3) => {
                    Attribute::SourceFile(SourceFile::parse(length, info, constant_pool)?)
                }
                ("SourceDebugExtension", ClassFile, v) if v >= Version(49, 0) => {
                    Attribute::SourceDebugExtension(SourceDebugExtension::parse(
                        length,
                        info,
                        constant_pool,
                    )?)
                }
                ("LineNumberTable", Code, v) if v >= Version(45, 3) => {
                    let attribute = LineNumberTable::parse(length, info, constant_pool)?;
                    if let Some(Attribute::LineNumberTable(old)) = map.get_mut("LineNumberTable") {
                        old.merge(attribute);
                        continue;
                    } else {
                        Attribute::LineNumberTable(attribute)
                    }
                }
                ("LocalVariableTable", Code, v) if v >= Version(45, 3) => {
                    let attribute = LocalVariableTable::parse(length, info, constant_pool)?;
                    if let Some(Attribute::LocalVariableTable(old)) =
                        map.get_mut("LocalVariableTable")
                    {
                        old.merge(attribute);
                        continue;
                    } else {
                        Attribute::LocalVariableTable(attribute)
                    }
                }
                ("LocalVariableTypeTable", Code, v) if v >= Version(49, 0) => {
                    let attribute = LocalVariableTypeTable::parse(length, info, constant_pool)?;
                    if let Some(Attribute::LocalVariableTypeTable(old)) =
                        map.get_mut("LocalVariableTypeTable")
                    {
                        old.merge(attribute);
                        continue;
                    } else {
                        Attribute::LocalVariableTypeTable(attribute)
                    }
                }
                ("Deprecated", ClassFile | Field | Method, v) if v >= Version(45, 3) => {
                    Attribute::Deprecated(Deprecated::parse(length, info, constant_pool)?)
                }
                ("RuntimeVisibleAnnotations", ClassFile | Field | Method, v)
                    if v >= Version(49, 0) =>
                {
                    Attribute::RuntimeVisibleAnnotations(RuntimeVisibleAnnotations::parse(
                        length,
                        info,
                        constant_pool,
                    )?)
                }
                ("RuntimeInvisibleAnnotations", ClassFile | Field | Method, v)
                    if v >= Version(49, 0) =>
                {
                    Attribute::RuntimeInvisibleAnnotations(RuntimeInvisibleAnnotations::parse(
                        length,
                        info,
                        constant_pool,
                    )?)
                }
                ("RuntimeVisibleParameterAnnotations", Method, v) if v >= Version(49, 0) => {
                    Attribute::RuntimeVisibleParameterAnnotations(
                        RuntimeVisibleParameterAnnotations::parse(length, info, constant_pool)?,
                    )
                }
                ("RuntimeInvisibleParameterAnnotations", Method, v) if v >= Version(49, 0) => {
                    Attribute::RuntimeInvisibleParameterAnnotations(
                        RuntimeInvisibleParameterAnnotations::parse(length, info, constant_pool)?,
                    )
                }
                ("RuntimeVisibleTypeAnnotations", ClassFile | Field | Method | Code, v)
                    if v >= Version(52, 0) =>
                {
                    Attribute::RuntimeVisibleTypeAnnotations(RuntimeVisibleTypeAnnotations::parse(
                        length,
                        info,
                        constant_pool,
                    )?)
                }
                ("RuntimeInvisibleTypeAnnotations", ClassFile | Field | Method | Code, v)
                    if v >= Version(52, 0) =>
                {
                    Attribute::RuntimeInvisibleTypeAnnotations(
                        RuntimeInvisibleTypeAnnotations::parse(length, info, constant_pool)?,
                    )
                }
                ("AnnotationDefault", Method, v) if v >= Version(49, 0) => {
                    Attribute::AnnotationDefault(AnnotationDefault::parse(
                        length,
                        info,
                        constant_pool,
                    )?)
                }
                ("BootstrapMethods", ClassFile, v) if v >= Version(51, 0) => {
                    Attribute::BootstrapMethods(BootstrapMethods::parse(
                        length,
                        info,
                        constant_pool,
                    )?)
                }
                ("MethodParameters", Method, v) if v >= Version(52, 0) => {
                    Attribute::MethodParameters(MethodParameters::parse(
                        length,
                        info,
                        constant_pool,
                    )?)
                }
                _ => {
                    let _data = info.take(info.len())?;
                    Attribute::UnknownAttribute
                }
            };
            if info.len() > 0 {
                Err(AttributeFormatError::LengthError)?
            }
            map.insert(name, attribute);
        }
        Ok(map)
    }
}
#[derive(Fail, Debug)]
pub enum AttributeFormatError {
    #[fail(display = "Attribute type error,except {} , found {} .", _0, _1)]
    TypeError(PooledStr, PooledStr),
    #[fail(display = "Constant {} not found.", _0)]
    NotFoundError(u16),
    #[fail(display = "Illegal Constant.")]
    IllegalConstant,
    #[fail(display = "Illegal attribute format.")]
    IllegalFormatError,
    #[fail(display = "Illegal length")]
    LengthError,
}
