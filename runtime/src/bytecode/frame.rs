use std::{collections::HashMap, convert::TryInto, sync::Arc};

use super::Environment;
use classfile::{
    attributes::{StackMapFrame, StackMapTable, VerificationTypeInfo},
    symbol::TypeSymbol,
};
use failure::format_err;
use getset::{CopyGetters, Getters, MutGetters, Setters};
use jvm_core::{
    ClassLoaderRef, ExecutableRef, GeneraicTypeTrait, JavaClassTrait, PrimitiveType, Type,
};
use util::Result;
use FrameDataType::*;
#[derive(Clone, Debug)]
pub enum FrameDataType {
    Top,
    Float,
    Double,
    Int,
    Long,
    Reference(Arc<dyn JavaClassTrait>),
    ReturnAddress,
    Null,
    UninitializedThis(Arc<dyn JavaClassTrait>),
    UninitializedVariable(Option<Arc<dyn JavaClassTrait>>, u16),
}
impl PartialEq for FrameDataType {
    fn eq(&self, other: &Self) -> bool {
        todo!()
    }
}
impl Eq for FrameDataType {}
impl FrameDataType {
    pub fn is_assignable(&self, other: &Self) -> Result<bool> {
      // TODO:fix me
        Ok(self == other
            || match (self, other) {
                (_, Top)
                | (Null, Reference(_))
                | (Float, Float)
                | (Double, Double)
                | (Int, Int)
                | (Long, Long)
                | (Null, Null)
                | (ReturnAddress, ReturnAddress) => true,
                (UninitializedVariable(None, x), UninitializedVariable(_, y)) => x == y,
                (UninitializedVariable(Some(c0), x), UninitializedVariable(Some(c1), y)) => {
                    x == y && c0.equal(&**c1)?
                }
                (Reference(c0), Reference(c1)) => c0.is_assignable(c1)?,
                (UninitializedThis(c0), UninitializedThis(c1)) => c0.equal(&**c1)?,
                _ => false,
            })
    }

    pub fn is_two_word(&self) -> bool {
        match self {
            Double | Long => true,
            _ => false,
        }
    }

    pub fn from_verification_type_info(
        environment: &Environment,
        verification_type_info: &VerificationTypeInfo,
    ) -> Result<FrameDataType> {
        Ok(match verification_type_info {
            VerificationTypeInfo::TopVariableInfo => Top,
            VerificationTypeInfo::IntegerVariableInfo => Int,
            VerificationTypeInfo::LongVariableInfo => Long,
            VerificationTypeInfo::FloatVariableInfo => Float,
            VerificationTypeInfo::DoubleVariableInfo => Double,
            VerificationTypeInfo::NullVariableInfo => Null,
            VerificationTypeInfo::UninitializedThisVariableInfo => {
                UninitializedThis(environment.class.clone())
            }
            VerificationTypeInfo::ObjectVariableInfo(c) => {
                Reference(environment.class_loader.get_class(&c.symbol.name)?)
            }
            VerificationTypeInfo::UninitializedVariableInfo(offset) => {
                UninitializedVariable(None, offset)
            }
        })
    }

    pub fn from_generic_type(generic_type: &dyn GeneraicTypeTrait) -> Self {
        match generic_type.java_type() {
            Type::Primitive(PrimitiveType::Double) => Double,
            Type::Primitive(PrimitiveType::Float) => Float,
            Type::Primitive(PrimitiveType::Byte)
            | Type::Primitive(PrimitiveType::Char)
            | Type::Primitive(PrimitiveType::Short)
            | Type::Primitive(PrimitiveType::Boolean)
            | Type::Primitive(PrimitiveType::Int) => Int,
            Type::Primitive(PrimitiveType::Long) => (Long),
            Type::Void => panic!(),
            _ => Reference(generic_type.raw_class_owned()),
        }
    }

    pub fn from_type_symbol(
        class_loader: &ClassLoaderRef,
        type_symbol: &TypeSymbol,
    ) -> Result<Self> {
        let name = &type_symbol.name;
        Ok(match &**name {
            "B" | "C" | "I" | "S" | "Z" => Int,
            "L" => Long,
            "F" => Float,
            "D" => Double,
            "V" => Top,
            _ => Reference(class_loader.get_class(name)?),
        })
    }

    pub fn is_reference(&self) -> bool {
        match self {
            Reference(_) => true,
            _ => false,
        }
    }

    pub fn try_as_reference(&self) -> Option<&Arc<dyn JavaClassTrait>> {
        match self {
            Reference(c) => Some(c),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Getters)]
#[getset(get = "pub")]
pub struct FrameState {
    stack: Vec<FrameDataType>,
    locals: Vec<FrameDataType>,
    flag_this_uninit: bool,
}
impl FrameState {
    pub fn is_assignable(&self, other: &FrameState) -> Result<bool> {
        Ok(self.stack.len() == other.stack.len()
            && self
                .stack
                .iter()
                .zip(other.stack.iter())
                .map(|(a, b)| a.is_assignable(b))
                .try_fold(true, |l, c| c.map(|b| l & b))?
            && self.locals.len() == other.locals.len()
            && self
                .locals
                .iter()
                .zip(other.locals.iter())
                .map(|(a, b)| a.is_assignable(b))
                .try_fold(true, |l, c| c.map(|b| l & b))?
            && (!self.flag_this_uninit | other.flag_this_uninit))
    }

    pub fn stack_top_is_assignable(&self, data_type: &FrameDataType) -> Result<bool> {
        self.stack
            .last()
            .map(|t| t.is_assignable(data_type))
            .unwrap_or(Ok(true))
    }

    fn store_one_word(&mut self, index: u16, data_type: FrameDataType) -> Result<()> {
        self.locals
            .get_mut(index as usize)
            .ok_or_else(|| format_err!("local variable out of range"))
            .map(|p| *p = data_type)
    }

    pub fn store(&mut self, index: u16, data_type: FrameDataType) -> Result<()> {
        let is_two_word = data_type.is_two_word();
        if index > 0 && self.locals[index as usize - 1].is_two_word() {
            self.locals[index as usize - 1] = Top;
        }
        self.store_one_word(index, data_type).and_then(|_| {
            if is_two_word {
                self.store_one_word(index + 1, FrameDataType::Top)
            } else {
                Ok(())
            }
        })
    }

    fn load_match_one_word(&mut self, index: u16, data_type: &FrameDataType) -> Result<()> {
        self.locals
            .get(index as usize)
            .ok_or_else(|| format_err!("local variable out of range"))
            .and_then(|e| {
                e.is_assignable(data_type)
                    .map(|assignable| {
                        assignable
                            .then_some(())
                            .ok_or_else(|| format_err!("local variable is not assignable"))
                    })
                    .flatten()
            })
    }

    pub fn load_match(&mut self, index: u16, data_type: &FrameDataType) -> Result<()> {
        self.load_match_one_word(index, data_type).and_then(|_| {
            if data_type.is_two_word() {
                self.load_match_one_word(index + 1, &FrameDataType::Top)
            } else {
                Ok(())
            }
        })
    }

    pub fn push(&mut self, max_stack: u16, data_type: FrameDataType) -> Result<()> {
        let stack = &mut self.stack;
        match data_type {
            Top => Ok(()),
            _ => {
                stack.push(data_type);
                if stack.len() <= max_stack as usize {
                    Ok(())
                } else {
                    Err(format_err!("stack in stack_map_table is too long"))?
                }
            }
        }
    }

    pub fn pop_match(&mut self, data_type: &FrameDataType) -> Result<()> {
        self.stack
            .pop()
            .map(|p| p.is_assignable(data_type))
            .unwrap_or(Ok(true))
            .and_then(|a| a.then_some(()).ok_or(format_err!("stack not assignable")))
    }

    pub fn pop_match_slice(&mut self, types: &[FrameDataType]) -> Result<()> {
        types.iter().rev().try_for_each(|cell| self.pop_match(cell))
    }

    pub fn push_locals(
        &mut self,
        environment: &mut Environment<'_>,
        data_type: FrameDataType,
    ) -> Result<()> {
        self.locals.push(data_type);
        if self.locals.len() <= environment.code.max_locals as usize {
            Ok(())
        } else {
            Err(format_err!("local array in stack_map_table is too long"))?
        }
    }

    pub fn new(
        this_class: Option<&Arc<dyn JavaClassTrait>>,
        is_constructor: bool,
        max_stack: u16,
        max_locals: u16,
        method: &ExecutableRef,
    ) -> Self {
        Self {
            stack: Vec::with_capacity(max_stack as usize),
            flag_this_uninit: is_constructor,
            locals: {
                let mut locals = Vec::with_capacity(max_locals as usize);
                if let Some(this) = this_class {
                    locals.push(if is_constructor {
                        UninitializedThis(this.clone())
                    } else {
                        Reference(this.clone())
                    });
                }
                method.try_for_each_parameters(&mut |p| {
                    let frame_data_type = FrameDataType::from_generic_type(p.get_type());
                    let is_two_word = frame_data_type.is_two_word();
                    locals.push(frame_data_type);
                    if is_two_word {
                        locals.push(Top);
                    }
                    Ok(())
                });
                locals.resize(max_locals as usize, Top);
                locals
            },
        }
    }

    pub fn from_stack_map_table(
        stack_map_table: &StackMapTable,
        environment: &mut Environment<'_>,
    ) -> Result<HashMap<u32, Self>> {
        let mut frame = environment.frame.clone();
        let mut map: HashMap<u32, Self> = HashMap::with_capacity(stack_map_table.entries.len() + 1);
        let mut offset: i64 = -1;
        map.insert(0, frame.clone());
        for e in &stack_map_table.entries {
            let offset_delta = e.get_offset_delta();
            match e {
                StackMapFrame::SameFrame { .. } | StackMapFrame::SameFrameExtended { .. } => {
                    frame.stack.clear();
                }
                StackMapFrame::SameLocals1StackItemFrame { stack, .. }
                | StackMapFrame::SameLocals1StackItemFrameExtended { stack, .. } => {
                    frame.stack.clear();
                    push(
                        environment.frame,
                        environment.code.max_stack,
                        FrameDataType::from_verification_type_info(environment, stack)?,
                    )?;
                }
                StackMapFrame::ChopFrame { frame_type, .. } => {
                    frame.stack.clear();
                    for _ in 0..251 - frame_type {
                        frame
                            .locals
                            .pop()
                            .ok_or_else(|| format_err!("stack is too short"))?;
                    }
                }
                StackMapFrame::AppendFrame { locals, .. } => {
                    for l in locals {
                        let data_type = FrameDataType::from_verification_type_info(environment, l)?;
                        let is_two_word = data_type.is_two_word();
                        frame.locals.push(data_type);
                        if is_two_word {
                            frame.push_locals(environment, Top)?;
                        }
                    }
                }
                StackMapFrame::FullFrame { locals, stack, .. } => {
                    frame.stack.clear();
                    for s in stack {
                        push(
                            environment.frame,
                            environment.code.max_stack,
                            FrameDataType::from_verification_type_info(environment, s)?,
                        )?;
                    }
                    frame.locals.clear();
                    for l in locals {
                        let data_type = FrameDataType::from_verification_type_info(environment, l)?;
                        let is_two_word = data_type.is_two_word();
                        frame.locals.push(data_type);
                        if is_two_word {
                            frame.push_locals(environment, Top)?;
                        }
                    }
                }
            }
            offset = offset + offset_delta as i64 + 1;
            map.insert(offset.try_into()?, frame.clone());
        }
        Ok(map)
    }

    pub fn merge_assign(&mut self, other: &Self) -> Result<bool> {
        todo!()
    }
}

pub fn push(frame: &mut FrameState, max_stack: u16, data_type: FrameDataType) -> Result<()> {
    frame.push(max_stack, data_type)
}

pub fn pop_match(frame: &mut FrameState, data_type: &FrameDataType) -> Result<()> {
    frame.pop_match(data_type)
}
pub fn pop_match_slice(frame: &mut FrameState, data_type: &[FrameDataType]) -> Result<()> {
    frame.pop_match_slice(data_type)
}
pub fn load_match(frame: &mut FrameState, index: u16, data_type: &FrameDataType) -> Result<()> {
    frame.load_match(index, data_type)
}

pub fn store(frame: &mut FrameState, index: u16, data_type: FrameDataType) -> Result<()> {
    frame.store(index, data_type)
}
pub fn load<'l>(frame: &'l mut FrameState, index: u16) -> Result<&'l FrameDataType> {
    frame
        .locals
        .get(index as usize)
        .ok_or_else(|| format_err!("invalid local variable index"))
}

pub fn match_locals(frame: &mut FrameState, index: u16, exception: &FrameDataType) -> Result<()> {
    let actually = frame
        .locals
        .get(index as usize)
        .ok_or_else(|| format_err!("invalid index:{}", index))?;
    (actually == exception).then_some(()).ok_or_else(|| {
        format_err!(
            "invalid local variable type,except:{:?},actually:{:?}",
            exception,
            actually
        )
    })
}
pub fn pop(frame: &mut FrameState) -> Result<FrameDataType> {
    frame
        .stack
        .pop()
        .ok_or_else(|| format_err!("invalid stack size"))
}
pub fn stack_top_is_assignable(frame: &mut FrameState, data_type: &FrameDataType) -> Result<bool> {
    frame.stack_top_is_assignable(data_type)
}
pub fn peek<'l>(frame: &'l mut FrameState) -> Result<&'l FrameDataType> {
    frame
        .stack
        .last()
        .ok_or_else(|| format_err!("invalid stack size"))
}

pub fn replice_cell(frame: &mut FrameState, from: &FrameDataType, to: FrameDataType) {
    for cell in &mut frame.stack {
        if cell == from {
            *cell = to.clone();
        }
    }
    for cell in &mut frame.locals {
        if cell == from {
            *cell = to.clone();
        }
    }
}
