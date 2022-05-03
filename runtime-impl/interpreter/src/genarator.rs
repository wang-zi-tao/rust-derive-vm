use std::{
    alloc::{Layout, LayoutError},
    borrow::Cow,
    cell::RefCell,
    collections::{HashMap, HashSet},
    convert::TryFrom,
    iter::FromIterator,
    mem::{align_of, size_of},
    num::TryFromIntError,
    rc::Rc,
    sync::Arc,
};

use failure::{format_err, Error, Fail, Fallible};
use inkwell::{
    basic_block::BasicBlock,
    builder::Builder,
    context::Context,
    module::Module,
    types::{AnyType, AnyTypeEnum, BasicType, BasicTypeEnum, FloatType, FunctionType, IntType},
    values::{
        AnyValue, BasicValue, BasicValueEnum, CallableValue, FloatValue, FunctionValue, GlobalValue, InstructionOpcode, IntValue, PhiValue, PointerValue,
    },
    AddressSpace, AtomicOrdering, FloatPredicate, IntPredicate,
};
use vm_core::{FloatKind, IntKind, MaybeDefinedResource, Tuple, Type, TypeResource};
use std::convert::TryInto;
use util::{CowArc, CowSlice};

use runtime::instructions::{*};
#[derive(Debug, Fail)]
pub enum InstructionError {
    #[fail(display = "{}", _0)]
    OtherError(#[cause] Error),
    #[fail(display = "llvm error:{}", _0)]
    OtherLLVMError(String),
    #[fail(display = "llvm verify failed: \n{}", _0)]
    LLVMVerifyFailed(String),
    #[fail(display = "not supported")]
    NotSupported(),
    #[fail(display = "not type is unknown")]
    TypeIsUnnkown(),
    #[fail(display = "Argument was not found in index {}", _0)]
    ArgumentIndexOutOfRange(usize),
    #[fail(display = "Generic was not found in index {}", _0)]
    GenericIndexOutOfRange(usize),
    #[fail(display = "Generic with name {:?} was not found", _0)]
    MissGeneric(String),
    #[fail(display = "Invalid operand metadata")]
    InvalidOperandMetadata(),
    #[fail(display = "Illegal int kind: {}", _0)]
    IllegalIntKind(usize),
    #[fail(display = "Illegal float kind: {}", _0)]
    IllegalFloatKind(usize),
    #[fail(display = "Illegal `SetState` instruction")]
    IllegalSetStateInstructin(),
    #[fail(display = "Illegal generics to use as a argument")]
    ThisGenericCanNotUseAsArgument(),
    #[fail(display = "Wrone generic kind")]
    WroneGenericKind(),
    #[fail(display = "Except a GenericArgumentKind::Type")]
    ExceptTypeGeneric(),
    #[fail(display = "Variable {:?} was not found", _0)]
    VariableNotFound(String),
    #[fail(display = "State {:?} was not found", _0)]
    StateNotFound(String),
    #[fail(display = "Variable was not readable")]
    VariableNotReadable(),
    #[fail(display = "Variable {} was not initialized", _0)]
    VariableNotInitialized(String),
    #[fail(display = "Operand was not initialized")]
    OperandNotInitialized(),
    #[fail(display = "Too many sub instructions, max: 65535, got {}", _0)]
    TooManySubInstructions(usize),
    #[fail(display = "Wrone instruction count, except: {}, got {}", _0, _1)]
    WroneInstructionCount(usize, usize),
    #[fail(display = "Cannot get type of the generic metadata")]
    CannotGetTypeOfTheGeneric(),
    #[fail(display = "The size of return value is large than `size_of::<usize>()`")]
    ReturnValueTooLarge(),
    #[fail(display = "Error while finding operand with index {}:\nerror: {}", _0, _1)]
    ErrorWhileFindingOperand(usize, Box<Self>),
    #[fail(display = "Error while write back operand with index {}:\nerror: {}", _0, _1)]
    ErrorWhileWriteBackOperand(usize, Box<Self>),
    #[fail(display = "Error while generate instruction with index {}:\nerror: {}", _0, _1)]
    ErrorWhileGenerateInstruction(usize, Box<Self>),
    #[fail(display = "Error while generate bootstrap instruction {:?}:\nerror: {}", _0, _1)]
    ErrorWhileGenerateBoostrapInstruction(BootstrapInstruction, Box<Self>),
    #[fail(display = "Error while generate basic block with index {}, \nblock:({}),\nerror: {}\n", _0, _2, _1)]
    ErrorWhileGenerateBasicBlock(usize, Box<Self>, String),
    #[fail(display = "Error while generate complex instruction\ninstruction:({}),\nerror: {}\n", _1, _0)]
    ErrorWhileGenerateComplexInstruction(Box<Self>, String),
    #[fail(display = "Error while generate instruction call with index {},\ninstruction:{},\ncall:({}),\nerror:{}\n", _0, _3, _2, _1)]
    ErrorWhileGenerateInstructionCall(usize, Box<Self>, String, String),
    #[fail(display = "Terminator found in the middle of a basic block")]
    TerminedBasicBlock(),
    #[fail(display = "boostrap::CallStatful must be use in the stateful instruction")]
    NotInStatefulInstruction(),
    #[fail(display = "Except lit")]
    ExceptLit(),
    #[fail(display = "Except llvm value")]
    ExceptLLVMValue(),
    #[fail(display = "Except basic block")]
    ExceptBasciBlock(),
    #[fail(display = "Except reference type, got {:?}", _0)]
    ExceptReferenceType(Type),
    #[fail(display = "Except array type, got {:?}", _0)]
    ExceptArrayType(Type),
    #[fail(display = "Except tuple type, got {:?}", _0)]
    ExceptTupleType(Type),
    #[fail(display = "Except union type, got {:?}", _0)]
    ExceptUnionType(Type),
    #[fail(display = "Except enum type, got {:?}", _0)]
    ExceptEnumType(Type),
    #[fail(display = "Except funtion type, got {:?}", _0)]
    ExceptFunctionType(Type),
    #[fail(display = "Except metadata type, got {:?}", _0)]
    ExceptMetadataType(Type),
    #[fail(display = "Except normal tuple type, got {:?}", _0)]
    ExceptNormalTupleType(Type),
    #[fail(display = "Except normal enum type, got {:?}", _0)]
    ExceptNormalEnumType(Type),
    #[fail(display = "Except compose enum type, got {:?}", _0)]
    ExceptComposeEnumType(Type),
    #[fail(display = "Except state")]
    ExceptState(),
    #[fail(display = "The tuple type {:?} has no field with index {}", _0, _1)]
    FieldIndexOutOfRange(Type, usize),
    #[fail(display = "The enum type {:?} has no variant with index {}", _0, _1)]
    VariantIndexOutOfRange(Type, usize),
    #[fail(display = "The metadata type {:?} has no element with index {}", _0, _1)]
    MetadataIndexOutOfRange(Type, usize),
    #[fail(display = "except register to get pointer")]
    ExceptRegister(),
    #[fail(display = "phi instruction {} miss block {}", _0, _1)]
    PhiInstructionMissBlock(String, String),
    #[fail(display = "type not match, except :{:?},\ngot :{:?}\n", _0, _1)]
    TypeNotMatch(Type, Type),
    #[fail(display = "llvm type not match, except :{},\ngot :{}\n", _0, _1)]
    LLVMTypeNotMatch(String, String),
}
impl From<Error> for InstructionError {
    fn from(error: Error) -> Self {
        OtherError(error)
    }
}
impl From<TryFromIntError> for InstructionError {
    fn from(error: TryFromIntError) -> Self {
        OtherError(error.into())
    }
}
impl From<LayoutError> for InstructionError {
    fn from(error: LayoutError) -> Self {
        OtherError(error.into())
    }
}
type Result<T> = std::result::Result<T, InstructionError>;

use InstructionError::*;
#[derive(Clone, Debug)]
pub(crate) enum Operand<'ctx> {
    Register(PointerValue<'ctx>, Type),
    Value(BasicValueEnum<'ctx>, Type),
    Uninitialized(Type),
}
impl<'ctx> Operand<'ctx> {
    fn load(&self, builder: &Builder<'ctx>, _value_type: BasicTypeEnum<'ctx>) -> Result<BasicValueEnum<'ctx>> {
        match self {
            Self::Register(ptr, _ty) => {
                let value = builder.build_load(*ptr, "reg");
                Ok(value)
            }
            Self::Value(value, _ty) => Ok(*value),
            Self::Uninitialized(_ty) => Err(VariableNotReadable()),
        }
    }

    fn store(&mut self, builder: &Builder<'ctx>, value: BasicValueEnum<'ctx>) -> Result<()> {
        match self {
            Self::Register(ptr, _ty) => {
                builder.build_store(*ptr, value);
                Ok(())
            }
            Self::Value(_, ty) | Self::Uninitialized(ty) => {
                *self = Self::Value(value, ty.clone());
                Ok(())
            }
        }
    }

    pub(crate) fn get_ptr(&self, builder: &Builder<'ctx>, _registers: PointerValue<'ctx>, value_type: BasicTypeEnum<'ctx>) -> Result<PointerValue<'ctx>> {
        match self {
            Operand::Register(ptr, _ty) => Ok(builder.build_address_space_cast(*ptr, value_type.ptr_type(AddressSpace::Generic), "ptr")),
            _ => Err(ExceptRegister()),
        }
    }

    pub(crate) fn get_type(&self) -> &Type {
        match self {
            Operand::Register(_, ty) | Operand::Value(_, ty) | Operand::Uninitialized(ty) => ty,
        }
    }

    pub(crate) fn get_llvm_type(&self) -> Result<AnyTypeEnum<'ctx>> {
        match self {
            Operand::Value(v, _) => Ok(v.get_type().as_any_type_enum()),
            Operand::Register(r, _) => Ok(r.get_type().get_element_type()),
            Operand::Uninitialized(_) => Err(OperandNotInitialized()),
        }
    }

    pub(crate) fn cast_to_reference(&mut self, context: &'ctx Context, builder: &Builder<'ctx>, value_type: &Type) -> Result<()> {
        match self {
            Operand::Value(v, _ty) => {
                *v = builder.build_pointer_cast(v.into_pointer_value(), context.i8_type().ptr_type(AddressSpace::Generic), "casted").into();
            }
            Operand::Register(v, _ty) => {
                *v = builder.build_pointer_cast(*v, context.i8_type().ptr_type(AddressSpace::Generic).ptr_type(AddressSpace::Local), "casted");
            }
            Operand::Uninitialized(_ty) => {}
        }
        match self {
            Operand::Register(_, ty) | Operand::Value(_, ty) | Operand::Uninitialized(ty) => *ty = value_type.clone(),
        }
        Ok(())
    }

    pub(crate) fn pointer_cast(&mut self, context: &'ctx Context, builder: &Builder<'ctx>, value_type: Type) -> Result<()> {
        match self {
            Operand::Value(v, _ty) => {
                *v = builder
                    .build_pointer_cast(v.into_pointer_value(), vm_type_to_llvm_type(&value_type, context)?.ptr_type(AddressSpace::Generic), "casted")
                    .into();
            }
            Operand::Register(v, _ty) => {
                *v = builder.build_pointer_cast(
                    *v,
                    vm_type_to_llvm_type(&value_type, context)?.ptr_type(AddressSpace::Generic).ptr_type(AddressSpace::Local),
                    "casted",
                );
            }
            Operand::Uninitialized(_ty) => {}
        }
        match self {
            Operand::Register(_, ty) | Operand::Value(_, ty) | Operand::Uninitialized(ty) => *ty = Type::Pointer(CowArc::new(value_type)),
        }
        Ok(())
    }
}
#[derive(Clone)]
pub(crate) enum TargetBlock<'ctx> {
    Offset(PointerValue<'ctx>),
    Block { from: LLVMBasicBlockBuilderRef<'ctx>, block: LLVMBasicBlockBuilderRef<'ctx>,  },
}
#[derive(Clone)]
pub(crate) enum Constant<'ctx> {
    Value(BasicValueEnum<'ctx>, Type),
    Ptr(PointerValue<'ctx>, Type),
    BasicBlock(TargetBlock<'ctx>),
    Type(Type, BasicTypeEnum<'ctx>),
    Instruction(InstructionType),
    State(String),
}
impl<'ctx> Constant<'ctx> {
    fn as_type(&self, _context: &'ctx Context) -> Result<(Type, BasicTypeEnum<'ctx>)> {
        Ok(match self {
            Constant::Type(ty, llvm_type) => (ty.clone(), *llvm_type),
            _ => return Err(ExceptTypeGeneric()),
        })
    }

    fn as_int_type(&self, context: &'ctx Context) -> Result<IntType<'ctx>> {
        Ok(match self {
            Constant::Value(v, _ty) => {
                let intkind = v.into_int_value().get_zero_extended_constant().ok_or_else(ExceptLit)?;
                match intkind {
                    0 => context.bool_type(),
                    1 => context.i8_type(),
                    3 => context.i16_type(),
                    5 => context.i32_type(),
                    7 => context.i64_type(),
                    9 => context.i128_type(),
                    11 => context.custom_width_int_type(isize::BITS),
                    2 => context.i8_type(),
                    4 => context.i16_type(),
                    6 => context.i32_type(),
                    8 => context.i64_type(),
                    10 => context.i128_type(),
                    12 => context.custom_width_int_type(isize::BITS),
                    o => return Err(IllegalIntKind(o as usize)),
                }
            }
            _ => {
                return Err(ExceptLit());
            }
        })
    }

    fn as_float_type(&self, context: &'ctx Context) -> Result<FloatType<'ctx>> {
        Ok(match self {
            Constant::Value(v, _ty) => match v.into_int_value().get_zero_extended_constant() {
                Some(32) => context.f32_type(),
                Some(64) => context.f64_type(),
                Some(o) => return Err(IllegalFloatKind(o as usize)),
                None => return Err(ExceptLit()),
            },
            _ => {
                return Err(ExceptLit());
            }
        })
    }

    fn get_value(&self, builder: &Builder<'ctx>) -> Result<BasicValueEnum<'ctx>> {
        Ok(match self {
            Constant::Value(v, _ty) => *v,
            Constant::Ptr(p, _ty) => builder.build_load(*p, "constant"),
            _ => {
                return Err(ExceptLLVMValue());
            }
        })
    }

    pub fn as_ptr(&self, _builder: &Builder<'ctx>) -> Result<PointerValue<'ctx>> {
        match self {
            Constant::Value(_v, _ty) => Err(ExceptRegister()),
            Constant::Ptr(p, _ty) => Ok(*p),
            _ => {
                panic!("int width code must be a const value");
            }
        }
    }

    fn get_int_value(&self, builder: &Builder<'ctx>) -> Fallible<IntValue<'ctx>> {
        Ok(self.get_value(builder)?.into_int_value())
    }

    fn get_basic_block_value(&self) -> Result<TargetBlock<'ctx>> {
        if let Self::BasicBlock(b) = self {
            Ok(b.clone())
        } else {
            Err(ExceptBasciBlock())
        }
    }

    fn get_state(&self) -> Result<&str> {
        match self {
            Constant::State(s) => Ok(&**s),
            _ => Err(ExceptState()),
        }
    }
}
pub(crate) fn convert_value<'ctx>(value: &Value, context: &'ctx Context) -> Result<(BasicValueEnum<'ctx>, Type)> {
    Ok(match value {
        Value::Str(s) => (
            context.i8_type().const_array(&(&**s).bytes().map(|b| context.i8_type().const_int(b as u64, false)).collect::<Vec<_>>()).into(),
            Type::Array(CowArc::new(Type::Int(IntKind::U8)), Some(s.bytes().len())),
        ),
        Value::ByteStr(_) => todo!(),
        Value::F32(v) => (context.f32_type().const_float(*v as f64).into(), Type::Float(FloatKind::F32)),
        Value::F64(v) => (context.f64_type().const_float(*v).into(), Type::Float(FloatKind::F64)),
        Value::I64(v) => (context.i64_type().const_int(*v as u64, true).into(), Type::Int(IntKind::I64)),
        Value::U8(v) => (context.i8_type().const_int(*v as u64, false).into(), Type::Int(IntKind::U8)),
        Value::Bool(v) => (context.bool_type().const_int(*v as u64, false).into(), Type::Int(IntKind::Bool)),
        Value::RustFn(f) => (context.custom_width_int_type(usize::BITS).const_int(*f as u64, false).into(), Type::Int(IntKind::Usize)),
        Value::Type(_) => todo!(),
        Value::Instruction(_) => todo!(),
    })
}
pub(crate) fn bitcast_to_int<'ctx>(
    value: BasicValueEnum<'ctx>,
    _ty: &Type,
    _context: &'ctx Context,
    builder: &Builder<'ctx>,
    to: IntType<'ctx>,
) -> Result<IntValue<'ctx>> {
    let from_type = value.get_type();
    Ok(match from_type {
        BasicTypeEnum::IntType(_) => builder.build_int_cast(value.into_int_value(), to, "casted"),
        BasicTypeEnum::FloatType(_) => builder.build_bitcast(value, to, "casted").into_int_value(),
        BasicTypeEnum::ArrayType(_) => return Err(NotSupported()),
        BasicTypeEnum::PointerType(_) => builder.build_ptr_to_int(value.into_pointer_value(), to, "casted"),
        BasicTypeEnum::StructType(_s) => {
            return Err(NotSupported());
        }
        _ => return Err(NotSupported()),
    })
}
pub(crate) fn bitcast_from_int<'ctx>(
    value: IntValue<'ctx>,
    _context: &'ctx Context,
    builder: &Builder<'ctx>,
    to: BasicTypeEnum<'ctx>,
) -> Result<BasicValueEnum<'ctx>> {
    let _from_type = value.get_type();
    Ok(match to {
        BasicTypeEnum::IntType(to) => builder.build_int_cast(value, to, "casted").into(),
        BasicTypeEnum::FloatType(_) => builder.build_bitcast(value, to, "casted"),
        BasicTypeEnum::ArrayType(_) => todo!(),
        BasicTypeEnum::PointerType(to) => builder.build_int_to_ptr(value, to, "casted").into(),
        BasicTypeEnum::StructType(_) => todo!(),
        _ => return Err(NotSupported()),
    })
}
pub(crate) fn vm_type_to_llvm_type<'ctx>(ty: &Type, context: &'ctx Context) -> Result<BasicTypeEnum<'ctx>> {
    let layout = ty.get_layout()?;
    Ok(match ty {
        Type::Float(f) => match f {
            FloatKind::F32 => context.f32_type(),
            FloatKind::F64 => context.f64_type(),
        }
        .into(),
        Type::Int(i) => match i {
            IntKind::Bool => context.bool_type(),
            IntKind::I8 => context.i8_type(),
            IntKind::I16 => context.i16_type(),
            IntKind::I32 => context.i32_type(),
            IntKind::I64 => context.i64_type(),
            IntKind::I128 => context.i128_type(),
            IntKind::Isize => context.custom_width_int_type(isize::BITS),
            IntKind::U8 => context.i8_type(),
            IntKind::U16 => context.i16_type(),
            IntKind::U32 => context.i32_type(),
            IntKind::U64 => context.i64_type(),
            IntKind::U128 => context.i128_type(),
            IntKind::Usize => context.custom_width_int_type(isize::BITS),
        }
        .into(),
        Type::MetaData(_) => context.struct_type(&[], false).into(),
        Type::Const(_, _) => context.struct_type(&[], false).into(),
        Type::Enum(e) => {
            let mut variants_size = 0;
            for variant in e.variants.iter() {
                let layout = variant.get_layout()?;
                variants_size = usize::max(variants_size, layout.size());
            }
            let variants_type = context.custom_width_int_type((u8::BITS as usize * variants_size).try_into()?).into();
            if let Some(tag_bytes) = e.tag_bytes()? {
                context.struct_type(&[variants_type, context.custom_width_int_type((8 * tag_bytes).try_into()?).into()], false).into()
            } else {
                variants_type
            }
        }
        Type::Tuple(Tuple::Normal(fields)) => {
            let mut fields_type = Vec::with_capacity(fields.len());
            for field_type in fields.iter() {
                fields_type.push(vm_type_to_llvm_type(field_type, context)?);
            }
            context.struct_type(&*fields_type, false).into()
        }
        Type::Tuple(Tuple::Compose(_fields)) => context.custom_width_int_type(layout.size().try_into()?).into(),
        Type::Function(f) => function_type_to_llvm_type(f, context)?.ptr_type(AddressSpace::Generic).into(),
        Type::Native(_) | Type::Embed(_) | Type::Union(_) => context
            .struct_type(
                &[
                    context.i8_type().array_type(layout.size().try_into()?).into(),
                    context.custom_width_int_type((u8::BITS as usize * layout.align()).try_into()?).array_type(0).into(),
                ],
                false,
            )
            .into(),
        Type::Reference(_inner) => context.i8_type().ptr_type(AddressSpace::Generic).into(),
        Type::Pointer(inner) => vm_type_to_llvm_type(inner, context)?.ptr_type(AddressSpace::Generic).into(),
        Type::Array(element, option_size) => {
            if let Some(size) = option_size {
                vm_type_to_llvm_type(element, context)?.array_type((*size).try_into()?).into()
            } else {
                context
                    .struct_type(&[context.custom_width_int_type(usize::BITS).into(), vm_type_to_llvm_type(element, context)?.array_type(0).into()], true)
                    .into()
            }
        }
    })
}
pub(crate) fn function_type_to_llvm_type<'ctx>(function: &vm_core::FunctionType, context: &'ctx Context) -> Result<FunctionType<'ctx>> {
    let mut args = Vec::new();
    for arg in function.args() {
        args.push(vm_type_to_llvm_type(arg, context)?.into());
    }
    if let Some(va_arg) = function.va_arg() {
        args.push(
            context
                .struct_type(
                    &[vm_type_to_llvm_type(va_arg, context)?.ptr_type(AddressSpace::Generic).into(), context.custom_width_int_type(usize::BITS).into()],
                    false,
                )
                .into(),
        );
    };
    Ok(if let Some(return_type) = function.return_type() {
        vm_type_to_llvm_type(return_type, context)?.fn_type(&*args, false)
    } else {
        context.void_type().fn_type(&*args, false)
    })
}
pub(crate) fn get_constant_type<'ctx>(metadata: &GenericsMetadata, _context: &'ctx Context) -> Result<Type> {
    Ok(match &metadata.kind {
        GenericsMetadataKind::Constant { value_type, writable: _ } => value_type.clone(),
        GenericsMetadataKind::BasicBlock => Type::Int(IntKind::I32),
        _ => return Err(CannotGetTypeOfTheGeneric()),
    })
}
pub(crate) fn convert_memory_instruciton_operands<'ctx>(
    instruction_type: BootstrapInstruction,
    global_builder: &Rc<RefCell<GlobalBuilder<'ctx>>>,
    builder: &Builder<'ctx>,
    ty: &MaybeDefinedResource<dyn TypeResource>,
    operands: &mut [Operand<'ctx>],
) -> Result<Vec<Operand<'ctx>>> {
    let mut new_operands = Vec::new();
    let mut global_ref = global_builder.borrow_mut();
    let context = global_ref.context;
    let global = global_ref.module.add_global(
        context.opaque_struct_type(&format!("type_resource_{}", global_ref.symbol_maps.len())),
        Some(AddressSpace::Generic),
        &format!("type_{}", global_ref.symbol_maps.len()),
    );
    global.set_alignment(align_of::<CowArc<dyn TypeResource>>().try_into()?);
    global.set_constant(true);
    global.set_unnamed_addr(true);
    ty.try_map(|ty| {
        global_ref.symbol_maps.insert(global, CowArc::into_raw(ty.clone()).cast());
        Ok(())
    })?;
    new_operands.push(Operand::Value(
        builder.build_pointer_cast(global.as_pointer_value(), context.i8_type().ptr_type(AddressSpace::Generic), "type_resource").into(),
        Type::Pointer(CowArc::new(Type::Int(IntKind::U8))),
    ));
    use BootstrapInstruction::*;
    match instruction_type {
        Deref | Clone => {
            let mut operand0 = operands.get(0).ok_or(ArgumentIndexOutOfRange(0))?.clone();
            operand0.pointer_cast(context, builder, Type::Int(IntKind::U8))?;
            new_operands.push(operand0);
            let mut operand1 = operands.get(1).ok_or(ArgumentIndexOutOfRange(1))?.clone();
            operand1.pointer_cast(context, builder, Type::Int(IntKind::U8))?;
            new_operands.push(operand1);
        }
        Drop | AllocSized | NonGCAllocSized | NonGCFree => {
            let mut operand = operands.get(0).ok_or(ArgumentIndexOutOfRange(0))?.clone();
            operand.pointer_cast(context, builder, Type::Int(IntKind::U8))?;
            new_operands.push(operand);
        }
        AllocUnsized | NonGCAllocUnsized => {
            new_operands.push(operands.get(0).ok_or(ArgumentIndexOutOfRange(0))?.clone());
            let mut operand = operands.get(1).ok_or(ArgumentIndexOutOfRange(1))?.clone();
            operand.pointer_cast(context, builder, Type::Int(IntKind::U8))?;
            new_operands.push(operand);
        }
        _ => unreachable!(),
    };
    Ok(new_operands)
}
pub(crate) fn write_back_memory_instruciton_operands<'ctx>(
    instruction_type: BootstrapInstruction,
    global_builder: &Rc<RefCell<GlobalBuilder<'ctx>>>,
    builder: &Builder<'ctx>,
    ty: &MaybeDefinedResource<dyn TypeResource>,
    operands: &mut [Operand<'ctx>],
    new_operands: Vec<Operand<'ctx>>,
) -> Result<()> {
    let global_ref = global_builder.borrow_mut();
    let context = global_ref.context;
    use BootstrapInstruction::*;
    match instruction_type {
        AllocSized => {
            let mut new = new_operands.get(1).ok_or(ArgumentIndexOutOfRange(1))?.clone();
            new.cast_to_reference(context, builder, &Type::Reference(ty.clone()))?;
            *operands.get_mut(0).ok_or(ArgumentIndexOutOfRange(0))? = new;
        }
        NonGCAllocSized => ty.try_map(|ty| {
            let mut new = new_operands.get(1).ok_or(ArgumentIndexOutOfRange(1))?.clone();
            new.pointer_cast(context, builder, ty.get_type()?.clone())?;
            *operands.get_mut(0).ok_or(ArgumentIndexOutOfRange(0))? = new;
            Ok(())
        })?,
        Drop | NonGCFree => {}
        Clone | AllocUnsized => {
            let mut new = new_operands.get(2).ok_or(ArgumentIndexOutOfRange(2))?.clone();
            new.cast_to_reference(context, builder, &Type::Reference(ty.clone()))?;
            *operands.get_mut(1).ok_or(ArgumentIndexOutOfRange(1))? = new;
        }
        Deref | NonGCAllocUnsized => ty.try_map(|ty| {
            let mut new = new_operands.get(2).ok_or(ArgumentIndexOutOfRange(2))?.clone();
            new.pointer_cast(context, builder, ty.get_type()?.clone())?;
            *operands.get_mut(1).ok_or(ArgumentIndexOutOfRange(1))? = new;
            Ok(())
        })?,
        _ => unreachable!(),
    };
    Ok(())
}

#[derive(Clone)]
pub(crate) enum StateKind<'ctx> {
    StateConstant(Constant<'ctx>),
    Opcode(PointerValue<'ctx>, usize),
}
#[derive(Clone)]
pub(crate) struct StateInstructionBuilder<'ctx> {
    instruction: StatefulInstruction,
    state_kind: StateKind<'ctx>,
}
pub(crate) type LLVMBasicBlockBuilderRef<'ctx> = Rc<RefCell<LLVMBasicBlockBuilder<'ctx>>>;
#[derive(Clone, Debug)]
pub(crate) struct LLVMBasicBlockBuilder<'ctx> {
    name: String,
    block: BasicBlock<'ctx>,
    end_block: Option<BasicBlock<'ctx>>,
    phis: HashMap<String, (PhiValue<'ctx>, Type, Option<runtime::instructions::Phi>)>,
    termined: bool,
    variables: HashMap<String, Operand<'ctx>>,
    branchs: Vec<(BasicBlock<'ctx>, LLVMBasicBlockBuilderRef<'ctx>)>,
}
#[derive(Clone, Debug)]
pub(crate) struct GlobalBuilder<'ctx> {
    pub(crate) symbol_maps: HashMap<GlobalValue<'ctx>, *const u8>,
    pub(crate) module: Arc<Module<'ctx>>,
    pub(crate) context: &'ctx Context,
}
pub(crate) struct LLVMFunctionBuilder<'ctx> {
    global: Rc<RefCell<GlobalBuilder<'ctx>>>,
    context: &'ctx Context,
    builder: Builder<'ctx>,
    ip: PointerValue<'ctx>,
    deploy_table: PointerValue<'ctx>,
    function: FunctionValue<'ctx>,
    instruction_type: InstructionType,
    state_stack: Vec<StateInstructionBuilder<'ctx>>,
    registers: PointerValue<'ctx>,
    memory_instruction_set: &'ctx MemoryInstructionSet,
    termined: bool,
    returned: bool,
    ip_phi: PhiValue<'ctx>,
    exit: BasicBlock<'ctx>,
}
impl<'ctx, 'm> LLVMFunctionBuilder<'ctx> {
    fn generate_boostrap_instruction_core(
        &mut self,
        bootstrap: &BootstrapInstruction,
        constants: &[Constant<'ctx>],
        operands: &mut [Operand<'ctx>],
    ) -> Result<()> {
        (|| {
            use BootstrapInstruction::*;
            let context = self.context;
            let builder = &self.builder;
            let _registers = &self.registers;
            let usize_type = context.custom_width_int_type(usize::BITS);
            macro_rules! get_type {
                ($index:expr) => {
                    constants.get($index).ok_or_else(|| GenericIndexOutOfRange($index))?.as_type(&self.context)?
                };
                () => {
                    get_type!(0)
                };
            }
            macro_rules! get_int_type {
                ($index:expr) => {
                    constants.get($index).ok_or_else(|| GenericIndexOutOfRange($index))?.as_int_type(&self.context)?
                };
                () => {
                    get_int_type!(0)
                };
            }
            macro_rules! get_float_type {
                ($index:expr) => {
                    constants.get($index).ok_or_else(|| GenericIndexOutOfRange($index))?.as_float_type(&self.context)?
                };
                () => {
                    get_float_type!(0)
                };
            }
            macro_rules! get {
                (args,$value_type:expr,$index:expr) => {
                    operands.get($index).ok_or_else(|| ArgumentIndexOutOfRange($index))?.load(&self.builder, $value_type)?
                };
                (constant,$value_type:expr,$index:expr) => {
                    constants.get($index).ok_or_else(|| ArgumentIndexOutOfRange($index))?.get_value(&self.builder)?
                };
            }
            macro_rules! load_int_operand {
                ($int_type:expr,$index:expr) => {
                    get!(args, $int_type.into(), $index).into_int_value()
                };
            }
            macro_rules! load_float_operand {
                ($float_type:expr,$index:expr) => {
                    get!(args, $float_type.into(), $index).into_float_value()
                };
            }
            macro_rules! store_operand {
                ($index:expr,$value:expr) => {{
                    let value: BasicValueEnum<'ctx> = $value;
                    operands.get_mut($index).ok_or_else(|| ArgumentIndexOutOfRange($index))?.store(&self.builder, value)?;
                }};
            }
            macro_rules! store_int_operand {
                ($index:expr,$value:expr) => {{
                    let value: IntValue<'ctx> = $value;
                    store_operand!($index, value.into());
                }};
            }
            macro_rules! store_float_operand {
                ($index:expr,$value:expr) => {{
                    let value: FloatValue<'ctx> = $value;
                    store_operand!($index, value.into());
                }};
            }
            macro_rules! get_int_constant {
                ($int_type:expr,$index:expr) => {
                    get!(constant, $int_type, $index).into_int_value()
                };
            }
            macro_rules! int_binary_instruction {
                ($backend_function:ident) => {{
                    let int_type = get_int_type!();
                    let arg0 = load_int_operand!(int_type, 0);
                    let arg1 = load_int_operand!(int_type, 1);
                    store_int_operand!(1, builder.$backend_function(arg0, arg1, &format!("{:?}", bootstrap)).into());
                }};
            }
            macro_rules! float_binary_instruction {
                ($backend_function:ident) => {{
                    let float_type = get_float_type!();
                    let arg0 = load_float_operand!(float_type, 0);
                    let arg1 = load_float_operand!(float_type, 1);
                    store_float_operand!(1, builder.$backend_function(arg0, arg1, &format!("{:?}", bootstrap)).into());
                }};
            }
            macro_rules! int_compare_instruction {
                ($predicate:ident) => {{
                    let int_type = get_int_type!();
                    let arg0 = load_int_operand!(int_type, 0);
                    let arg1 = load_int_operand!(int_type, 1);
                    store_int_operand!(2, builder.build_int_compare(IntPredicate::$predicate, arg0, arg1, &format!("{:?}", bootstrap)).into());
                }};
            }
            macro_rules! float_compare_instruction {
                ($predicate:ident) => {{
                    let float_type = get_float_type!();
                    let arg0 = load_float_operand!(float_type, 0);
                    let arg1 = load_float_operand!(float_type, 1);
                    store_int_operand!(2, builder.build_float_compare(FloatPredicate::$predicate, arg0, arg1, &format!("{:?}", bootstrap),).into());
                }};
            }
            match bootstrap {
                Nop => {}
                Move => {
                    let (_, llvm_type) = get_type!();
                    let src = get!(args, llvm_type, 0);
                    store_operand!(1, src);
                }
                Add => int_binary_instruction!(build_int_add),
                Sub => int_binary_instruction!(build_int_sub),
                Mul => int_binary_instruction!(build_int_mul),
                Div => int_binary_instruction!(build_int_signed_div),
                Rem => int_binary_instruction!(build_int_signed_rem),
                Neg => {
                    let int_type = get_int_type!();
                    let arg0 = load_int_operand!(int_type, 0);
                    store_int_operand!(0, builder.build_int_neg(arg0, "result"));
                }

                And => int_binary_instruction!(build_and),
                Or => int_binary_instruction!(build_or),
                Xor => int_binary_instruction!(build_xor),
                Not => {
                    let int_type = get_int_type!();
                    let arg0 = load_int_operand!(int_type, 0);
                    store_int_operand!(0, builder.build_not(arg0, "result"));
                }
                Shl => int_binary_instruction!(build_left_shift),
                Shr => {
                    let int_type = get_int_type!();
                    let arg0 = load_int_operand!(int_type, 0);
                    let arg1 = load_int_operand!(int_type, 1);
                    store_int_operand!(1, builder.build_right_shift(arg0, arg1, true, "result"));
                }
                Ushr => {
                    let int_type = get_int_type!();
                    let arg0 = load_int_operand!(int_type, 0);
                    let arg1 = load_int_operand!(int_type, 1);
                    store_int_operand!(1, builder.build_right_shift(arg0, arg1, false, "result"));
                }

                CmpLt => int_compare_instruction!(SLT),
                CmpLe => int_compare_instruction!(SLE),
                CmpGt => int_compare_instruction!(SGT),
                CmpGe => int_compare_instruction!(SGE),
                CmpEq => int_compare_instruction!(EQ),
                CmpNe => int_compare_instruction!(NE),
                UcmpLt => int_compare_instruction!(ULT),
                UcmpLe => int_compare_instruction!(ULE),
                UcmpGt => int_compare_instruction!(UGT),
                UcmpGe => int_compare_instruction!(UGE),
                UcmpEq => int_compare_instruction!(EQ),
                UcmpNe => int_compare_instruction!(NE),

                IntToFloat => {
                    let float_type = get_float_type!();
                    let int_type = get_int_type!(1);
                    let arg0 = load_int_operand!(int_type, 0);
                    store_float_operand!(1, builder.build_signed_int_to_float(arg0, float_type, "result"))
                }
                FloatToFloat => {
                    let target_float_type = get_float_type!();
                    let float_type = get_float_type!(1);
                    let arg0 = load_float_operand!(float_type, 0);
                    store_float_operand!(1, builder.build_float_ext(arg0, target_float_type, "result"))
                }
                FloatToInt => {
                    let int_type = get_int_type!();
                    let float_type = get_float_type!(1);
                    let arg0 = load_float_operand!(float_type, 0);
                    store_int_operand!(1, builder.build_float_to_signed_int(arg0, int_type, "result"))
                }

                IntExtend => {
                    let target_int_type = get_int_type!();
                    let int_type = get_int_type!(1);
                    let arg0 = load_int_operand!(int_type, 0);
                    store_int_operand!(1, builder.build_int_s_extend(arg0, target_int_type, "result"))
                }
                UIntExtend => {
                    let target_int_type = get_int_type!();
                    let int_type = get_int_type!(1);
                    let arg0 = load_int_operand!(int_type, 0);
                    store_int_operand!(1, builder.build_int_z_extend(arg0, target_int_type, "result"))
                }
                IntTruncate => {
                    let target_int_type = get_int_type!();
                    let int_type = get_int_type!(1);
                    let arg0 = load_int_operand!(int_type, 0);
                    store_int_operand!(1, builder.build_int_truncate(arg0, target_int_type, "result"))
                }

                FAdd => float_binary_instruction!(build_float_add),
                FSub => float_binary_instruction!(build_float_sub),
                FMul => float_binary_instruction!(build_float_mul),
                FDiv => float_binary_instruction!(build_float_div),
                FRem => float_binary_instruction!(build_float_rem),
                FNeg => {
                    let float_type = get_float_type!();
                    let arg0 = load_float_operand!(float_type, 0);
                    store_float_operand!(0, builder.build_float_neg(arg0, "FNeg"));
                }

                FcmpLt => float_compare_instruction!(OLT),
                FcmpLe => float_compare_instruction!(OLE),
                FcmpGt => float_compare_instruction!(OGT),
                FcmpGe => float_compare_instruction!(OGE),
                FcmpEq => float_compare_instruction!(OEQ),
                FcmpNe => float_compare_instruction!(ONE),
                Branch => {
                    builder.build_unconditional_branch(self.branch(constants.get(0).ok_or(GenericIndexOutOfRange(0))?)?);
                    builder.clear_insertion_position();
                }
                BranchIf => {
                    builder.build_conditional_branch(
                        load_int_operand!(context.bool_type(), 0),
                        self.branch(constants.get(0).ok_or(GenericIndexOutOfRange(0))?)?,
                        self.branch(constants.get(1).ok_or(GenericIndexOutOfRange(1))?)?,
                    );
                }
                CastUnchecked => {
                    let (_, dst_llvm_type) = get_type!();
                    let (_, src_llvm_type) = get_type!(1);
                    let src = get!(args, src_llvm_type, 0);
                    store_operand!(1, builder.build_bitcast(src, dst_llvm_type, "CastUnchecked"));
                }
                // fn<type fn_type,const ptr:Pointer<U8>,const symbol:&str>(...)->(r);
                NativeCall => {
                    let (value_type, _llvm_type) = get_type!();
                    let function_ptr_int = get!(constant, llvm_type, 1).into_int_value();
                    let _function_name = get!(constant, llvm_type, 2);
                    match value_type {
                        Type::Function(f) => {
                            let function_llvm_type = function_type_to_llvm_type(&*f, context)?;
                            let function_ptr = builder.build_int_to_ptr(function_ptr_int, function_llvm_type.ptr_type(AddressSpace::Shared), "function_ptr");
                            let mut args = Vec::with_capacity(function_llvm_type.count_param_types() as usize);
                            for (index, arg_type) in function_llvm_type.get_param_types().into_iter().enumerate() {
                                args.push(get!(args, arg_type, index).into());
                            }
                            let call = builder.build_call(CallableValue::try_from(function_ptr).unwrap(), &*args, "NativeCall");
                            if let Some(_return_type) = function_llvm_type.get_return_type() {
                                store_operand!(function_llvm_type.count_param_types() as usize, call.try_as_basic_value().left().unwrap());
                            }
                        }
                        o => return Err(ExceptFunctionType(o)),
                    }
                }
                // fn<type fn_type>(fn:Pointer<U8>,args...)->(o)
                Call => {
                    let (value_type, llvm_type) = get_type!();
                    let function_ptr = get!(args, llvm_type, 0).into_pointer_value();
                    match value_type {
                        Type::Function(f) => {
                            let function_llvm_type = function_type_to_llvm_type(&*f, context)?;
                            let mut args = Vec::with_capacity(function_llvm_type.count_param_types() as usize);
                            for (index, arg_type) in function_llvm_type.get_param_types().into_iter().enumerate() {
                                args.push(get!(args, arg_type, index + 1).into());
                            }
                            let call = builder.build_call(CallableValue::try_from(function_ptr).unwrap(), &*args, "Call");
                            if let Some(_return_type) = function_llvm_type.get_return_type() {
                                store_operand!(function_llvm_type.count_param_types() as usize + 1, call.try_as_basic_value().left().unwrap());
                            }
                        }
                        o => return Err(ExceptFunctionType(o)),
                    }
                }
                // fn<type fn_type,block then,block catch>(fn:Pointer<U8>,args...,vaargs:Slice<U8>)->(o)
                Invoke => {
                    let (value_type, llvm_type) = get_type!();
                    let function_ptr = get!(args, llvm_type, 0).into_pointer_value();
                    match value_type {
                        Type::Function(f) => {
                            let function_llvm_type = function_type_to_llvm_type(&*f, context)?;
                            let mut args = Vec::with_capacity(function_llvm_type.count_param_types() as usize);
                            for (index, arg_type) in function_llvm_type.get_param_types().into_iter().enumerate() {
                                args.push(get!(args, arg_type, index + 1));
                            }
                            let then_block = self.branch(constants.get(0).ok_or(GenericIndexOutOfRange(0))?)?;
                            let catch_block = self.branch(constants.get(1).ok_or(GenericIndexOutOfRange(1))?)?;
                            let call = builder.build_invoke(CallableValue::try_from(function_ptr).unwrap(), &*args, then_block, catch_block, "Invoke");
                            if let Some(_return_type) = function_llvm_type.get_return_type() {
                                store_operand!(function_llvm_type.count_param_types() as usize + 1, call.try_as_basic_value().left().unwrap());
                            }
                        }
                        o => return Err(ExceptFunctionType(o)),
                    }
                }
                Return => {
                    let (value_type, llvm_type) = get_type!();
                    let layout = value_type.get_layout()?;
                    if layout.size() > size_of::<usize>() {
                        return Err(ReturnValueTooLarge());
                    }
                    let value = get!(args, llvm_type, 0);
                    let value_int = bitcast_to_int(value, &value_type, context, builder, usize_type)?;
                    let value_i64 = builder.build_int_cast(value_int, usize_type, "value_casted");
                    builder.build_return(Some(&value_i64));
                }
                // fn<const ty:TYPE>()->(out:Pointer<Type>)
                AllocSized |
                // fn<const ty:TYPE>(in:Usize)->(out:Pointer<Type>)
                AllocUnsized |
                // fn<const ty:TYPE>()->(out:Pointer<Type>)
                NonGCAllocSized |
                // fn<const ty:TYPE>(in:Usize)->(out:Pointer<Type>)
                NonGCAllocUnsized |
                // fn<const ty:TYPE>(ptr:Pointer<Type>)->()
                NonGCFree |
                // fn<type ty>(in:Reference<TYPE>)->(out:Pointer<TYPE>)
                Deref |
                // fn<type ty:Type::Reference>(in:Reference<TYPE>)->(out:Reference<TYPE>)
                Clone |
                // fn<type ty:Type::Reference>(in:Reference<TYPE>)->()
                Drop => {
                    let (value_type, _llvm_type) = get_type!();
                    match value_type {
                        Type::Reference(ref r) => {
                            let mut new_operands = convert_memory_instruciton_operands(*bootstrap, &self.global, builder, r, operands)?;
                            self.generate_instruction_core(self.memory_instruction_set.get(*bootstrap).unwrap(), &[], &mut new_operands)?;
                            write_back_memory_instruciton_operands(*bootstrap, &self.global, &self.builder, r, operands, new_operands)?;
                        }
                        _ => return Err(ExceptReferenceType(value_type.clone())),
                    }
                }
                // fn<type ty>(in:Enum)->(out:Usize)
                GetTag => {
                    let (enum_type, enum_llvm_type) = get_type!();
                    let enum_value = get!(args, enum_llvm_type, 0);
                    match &enum_type {
                        Type::Enum(enum_ty) => match enum_ty.tag_layout {
                            vm_core::EnumTagLayout::UndefinedValue { end, start } => {
                                let raw_tag = builder.build_int_sub(enum_value.into_int_value(),usize_type.const_int(start as u64, false), "raw_enum");
                                let is_zero = builder.build_int_compare(IntPredicate::UGE, raw_tag, context.i64_type().const_int((end - start) as u64, false), "is_zero");
                                let tag = builder.build_select(is_zero, usize_type.const_int(0, false), builder.build_int_add(raw_tag,usize_type.const_int(1, false),"non_zero_tag"), "tag");
                                store_operand!(1, tag);
                            }
                            vm_core::EnumTagLayout::SmallField(layout) => {
                                let enum_llvm_int_type = enum_llvm_type.into_int_type();
                                let mask = enum_llvm_int_type.const_int(layout.mask() as u64, true);
                                let masked = builder.build_and(mask, enum_value.into_int_value(), "masked");
                                let shifted_value = if layout.bit_offset() > 0 {
                                    builder.build_right_shift(masked, enum_llvm_int_type.const_int(layout.bit_offset() as u64, false), false, "tag")
                                } else {
                                    builder.build_left_shift(masked, enum_llvm_int_type.const_int(-layout.bit_offset() as u64, false), "tag")
                                };
                                store_operand!(1, shifted_value.into());
                            }
                            vm_core::EnumTagLayout::UnusedBytes { offset: _, size: _ } => return Err(ExceptComposeEnumType(enum_type)),
                            vm_core::EnumTagLayout::AppendTag { offset: _, size: _ } => {
                                store_operand!(1, builder.build_extract_value(enum_value.into_struct_value(), 1, "variant").unwrap());
                            }
                        },
                        _o => return Err(ExceptEnumType(enum_type)),
                    }
                }
                // fn<type ty>(in:Pointer<Enum>)->(out:Usize)
                ReadTag => {
                    let (enum_type, enum_llvm_type) = get_type!();
                    let enum_ptr_value = get!(args, enum_llvm_type.ptr_type(AddressSpace::Generic).into(), 0);
                    match &enum_type {
                        Type::Enum(enum_ty) => match enum_ty.tag_layout {
                            vm_core::EnumTagLayout::UnusedBytes { .. }
                            | vm_core::EnumTagLayout::UndefinedValue { .. }
                            | vm_core::EnumTagLayout::SmallField(_) => return Err(ExceptNormalEnumType(enum_type)),
                            vm_core::EnumTagLayout::AppendTag { offset: _, size: _ } => {
                                let tag = builder.build_load(
                                    builder
                                        .build_struct_gep(enum_ptr_value.into_pointer_value(), 1, "tag_ptr")
                                        .map_err(|_e| OtherLLVMError("error while inkwell::builder::Builder::build_struct_gep".to_string()))?,
                                    "tag",
                                );
                                store_operand!(1, tag);
                            }
                        },
                        _o => return Err(ExceptEnumType(enum_type)),
                    }
                }
                // fn<type ty>(in:Pointer<Enum>,tag:Usize)
                WriteTag => {
                    let (enum_type, enum_llvm_type) = get_type!();
                    let e = get!(args, enum_llvm_type.ptr_type(AddressSpace::Generic).into(), 0);
                    match &enum_type {
                        Type::Enum(enum_ty) => match enum_ty.tag_layout {
                            vm_core::EnumTagLayout::UnusedBytes { .. }
                            | vm_core::EnumTagLayout::UndefinedValue { .. }
                            | vm_core::EnumTagLayout::SmallField(_) => return Err(ExceptNormalEnumType(enum_type)),
                            vm_core::EnumTagLayout::AppendTag { offset: _, size: _ } => {
                                let tag = get!(args, usize_type.into(), 1);
                                builder.build_store(
                                    builder
                                        .build_struct_gep(e.into_pointer_value(), 1, "tag_ptr")
                                        .map_err(|_e| OtherLLVMError("error while inkwell::builder::Builder::build_struct_gep".to_string()))?,
                                    tag,
                                );
                            }
                        },
                        _o => return Err(ExceptEnumType(enum_type)),
                    }
                }
                // fn<type ty,const TAG:Usize>(in:Enum)->(out:Variant)
                DecodeVariantUnchecked => {
                    let (enum_type, enum_llvm_type) = get_type!();
                    let index = get_int_constant!(context.i32_type(), 1).get_zero_extended_constant().ok_or_else(WroneGenericKind)? as usize;
                    let enum_value = get!(args, enum_llvm_type, 0);
                    match &enum_type {
                        Type::Enum(enum_ty) => match enum_ty.tag_layout {
                            vm_core::EnumTagLayout::UndefinedValue { end: _, start: _ } => {
                                let variant_type = enum_ty.variants.get(index).ok_or_else(|| VariantIndexOutOfRange(enum_type.clone(), index))?;
                                let variant_llvm_type = vm_type_to_llvm_type(variant_type, context)?;
                                if index != 0 {
                                    store_operand!(1, variant_llvm_type.const_zero());
                                } else {
                                    let value = bitcast_from_int(enum_value.into_int_value(), context, builder, variant_llvm_type)?;
                                    store_operand!(1, value);
                                }
                            }
                            vm_core::EnumTagLayout::SmallField(layout) => {
                                let variant_type = enum_ty.variants.get(index).ok_or_else(|| VariantIndexOutOfRange(enum_type.clone(), index))?;
                                let variant_llvm_type = vm_type_to_llvm_type(variant_type, context)?;
                                let variant = builder.build_and(
                                    enum_value.into_int_value(),
                                    enum_llvm_type.into_int_type().const_int(!layout.mask() as u64, false),
                                    "compose_tag",
                                );
                                let variant_value = bitcast_from_int(variant, context, builder, variant_llvm_type)?;
                                store_operand!(1, variant_value);
                            }
                            vm_core::EnumTagLayout::UnusedBytes { offset: _, size: _ } => return Err(ExceptComposeEnumType(enum_type)),
                            vm_core::EnumTagLayout::AppendTag { offset: _, size: _ } => {
                                store_operand!(1, builder.build_extract_value(enum_value.into_struct_value(), 0, "variant").unwrap());
                            }
                        },
                        _o => return Err(ExceptEnumType(enum_type)),
                    }
                }
                // fn<type ty,const TAG:Usize>(in:Variant)->(out:Enum)
                EncodeVariant => {
                    let (enum_type, enum_llvm_type) = get_type!();
                    let index = get_int_constant!(context.i32_type(), 1).get_zero_extended_constant().ok_or_else(WroneGenericKind)? as usize;
                    match &enum_type {
                        Type::Enum(enum_ty) => {
                            let variant_type = enum_ty.variants.get(index).ok_or_else(|| VariantIndexOutOfRange(enum_type.clone(), index))?;
                            let variant_layout = variant_type.get_layout()?;
                            let variant_llvm_type = vm_type_to_llvm_type(variant_type, context)?;
                            let variant = get!(args, variant_llvm_type, 0);
                            match enum_ty.tag_layout {
                                vm_core::EnumTagLayout::UndefinedValue { end: _, start } => {
                                    if index != 0 {
                                        store_operand!(1, enum_llvm_type.into_int_type().const_int((start + index - 1) as u64, false).into());
                                    } else {
                                        let enum_value = if variant_layout.size() == 0 {
                                            enum_llvm_type.into_int_type().const_zero()
                                        } else {
                                            builder.build_int_cast(
                                                bitcast_to_int(variant, variant_type, context, builder, enum_llvm_type.into_int_type())?,
                                                enum_llvm_type.into_int_type(),
                                                "variant",
                                            )
                                        };
                                        store_operand!(1, enum_value.into());
                                    }
                                }
                                vm_core::EnumTagLayout::SmallField(layout) => {
                                    let enum_llvm_int_type = enum_llvm_type.into_int_type();
                                    let variant_int = if variant_layout.size() == 0 {
                                        enum_llvm_type.into_int_type().const_zero()
                                    } else {
                                        bitcast_to_int(
                                            variant,
                                            variant_type,
                                            context,
                                            builder,
                                            context.custom_width_int_type(u8::BITS * u32::try_from(variant_layout.size())?),
                                        )?
                                    };
                                    let enum_value = builder.build_or(
                                        builder.build_and(
                                            if layout.bit_offset() > 0 {
                                                builder.build_left_shift(
                                                    enum_llvm_int_type.const_int(index as u64, false),
                                                    enum_llvm_int_type.const_int(layout.bit_offset() as u64, false),
                                                    "tag_shift",
                                                )
                                            } else {
                                                builder.build_right_shift(
                                                    enum_llvm_int_type.const_int(index as u64, false),
                                                    enum_llvm_int_type.const_int(-layout.bit_offset() as u64, false),
                                                    false,
                                                    "tag_shift",
                                                )
                                            },
                                            enum_llvm_type.into_int_type().const_int(layout.mask() as u64, false),
                                            "compose_tag",
                                        ),
                                        builder.build_and(
                                            builder.build_int_cast(variant_int, enum_llvm_type.into_int_type(), "variant_int_casted"),
                                            enum_llvm_type.into_int_type().const_int(!layout.mask() as u64, false),
                                            "variant_casted",
                                        ),
                                        "enum",
                                    );
                                    store_operand!(1, enum_value.into());
                                }
                                vm_core::EnumTagLayout::UnusedBytes { offset: _, size: _ } => return Err(ExceptComposeEnumType(enum_type)),
                                vm_core::EnumTagLayout::AppendTag { offset: _, size: _ } => {
                                    let enum_value = enum_llvm_type.into_struct_type().const_zero();
                                    let enum_value = builder.build_insert_value(enum_value, variant, 0, "variant_cast").unwrap();
                                    let enum_value =
                                        builder.build_insert_value(enum_value, usize_type.const_int(index as u64, false), 0, "variant_cast").unwrap();
                                    store_operand!(1, enum_value.into_struct_value().into());
                                }
                            }
                        }
                        _o => return Err(ExceptEnumType(enum_type)),
                    }
                }
                // fn<type ty,const INDEX:Usize>(in:Pointer<Struct>)->(out:Pointer<Field>)
                LocateField => {
                    let (struct_type, struct_llvm_type) = get_type!();
                    let index = get_int_constant!(context.i32_type(), 1).get_zero_extended_constant().ok_or_else(WroneGenericKind)? as usize;
                    let _value = match &struct_type {
                        Type::Tuple(Tuple::Normal(f)) => {
                            let _field_type = f.get(index).ok_or_else(|| FieldIndexOutOfRange(struct_type.clone(), index))?;
                            let ptr = get!(args, struct_llvm_type.ptr_type(AddressSpace::Generic).into(), 0).into_pointer_value();
                            assert!((index as u32) < struct_llvm_type.into_struct_type().count_fields());
                            let field = builder
                                .build_struct_gep(ptr, index.try_into()?, "field_ptr")
                                .map_err(|_e| OtherLLVMError("error while inkwell::builder::Builder::build_struct_gep".to_string()))?;
                            store_operand!(1, field.into());
                        }
                        Type::Tuple(Tuple::Compose(_f)) => return Err(ExceptNormalTupleType(struct_type)),
                        _o => return Err(ExceptTupleType(struct_type)),
                    };
                }
                // fn<type ty,const INDEX:Usize>(in:Struct)->(out:Field)
                GetField => {
                    let (struct_type, struct_llvm_type) = get_type!();
                    let index = get_int_constant!(context.i32_type(), 1).get_zero_extended_constant().ok_or_else(WroneGenericKind)? as usize;
                    let _value = match &struct_type {
                        Type::Tuple(Tuple::Normal(f)) => {
                            let _field_type = f.get(index).ok_or_else(|| FieldIndexOutOfRange(struct_type.clone(), index))?;
                            let value = get!(args, struct_llvm_type, 0);
                            store_operand!(1, builder.build_extract_value(value.into_struct_value(), index.try_into()?, "field").unwrap());
                        }
                        Type::Tuple(Tuple::Compose(f)) => {
                            let (_field_type, layout) = f.get(index).ok_or_else(|| FieldIndexOutOfRange(struct_type.clone(), index))?;
                            let struct_llvm_int_type = struct_llvm_type.into_int_type();
                            let value = get!(args, struct_llvm_type, 0);
                            let mask = struct_llvm_int_type.const_int(layout.mask() as u64, false);
                            let masked = builder.build_and(mask, value.into_int_value(), "masked");
                            let shifted_value = if layout.bit_offset() > 0 {
                                builder.build_right_shift(masked, struct_llvm_int_type.const_int(layout.bit_offset() as u64, false), false, "field")
                            } else {
                                builder.build_left_shift(masked, struct_llvm_int_type.const_int(-layout.bit_offset() as u64, false), "field")
                            };
                            store_operand!(1, shifted_value.into());
                        }
                        _o => return Err(ExceptTupleType(struct_type)),
                    };
                }
                // fn<type ty,const INDEX:Usize>(s:Struct,f:Field)->(s:Struct)
                SetField => {
                    let (struct_type, struct_llvm_type) = get_type!();
                    let index = get_int_constant!(context.i32_type(), 1).get_zero_extended_constant().ok_or_else(WroneGenericKind)? as usize;
                    let _value = match &struct_type {
                        Type::Tuple(Tuple::Normal(f)) => {
                            let field_type = f.get(index).ok_or_else(|| FieldIndexOutOfRange(struct_type.clone(), index))?;
                            let field_llvm_type = vm_type_to_llvm_type(field_type, context)?;
                            let value = get!(args, struct_llvm_type, 0);
                            let field = get!(args, field_llvm_type, 1);
                            store_operand!(
                                0,
                                builder
                                    .build_insert_value(value.into_struct_value(), field, index.try_into()?, "field")
                                    .unwrap()
                                    .into_struct_value()
                                    .as_basic_value_enum()
                            );
                        }
                        Type::Tuple(Tuple::Compose(f)) => {
                            let (field_type, layout) = f.get(index).ok_or_else(|| FieldIndexOutOfRange(struct_type.clone(), index))?;
                            let field_llvm_type = vm_type_to_llvm_type(field_type, context)?;
                            let struct_llvm_int_type = struct_llvm_type.into_int_type();
                            let value = get!(args, struct_llvm_type, 0);
                            let field = get!(args, field_llvm_type, 1).into_int_value();
                            let mask = struct_llvm_int_type.const_int(layout.mask() as u64, true);
                            let shifted_field = if layout.bit_offset() > 0 {
                                builder.build_left_shift(field, struct_llvm_int_type.const_int(layout.bit_offset() as u64, false), "shifted")
                            } else {
                                builder.build_right_shift(field, struct_llvm_int_type.const_int(-layout.bit_offset() as u64, false), false, "shifted")
                            };
                            let value = builder.build_or(
                                builder.build_and(builder.build_not(mask, "flip_mask"), value.into_int_value(), "flip_masked"),
                                builder.build_and(mask, shifted_field, "masked"),
                                "new_value",
                            );
                            store_operand!(0, value.into());
                        }
                        _o => return Err(ExceptTupleType(struct_type)),
                    };
                }
                // fn<type ty,const TAG:Usize>(in:Reference<Union>)->(out:Reference<Variant>)
                LocateUnion => {
                    let (struct_type, struct_llvm_type) = get_type!();
                    let index = get_int_constant!(context.i32_type(), 1).get_zero_extended_constant().ok_or_else(WroneGenericKind)? as usize;
                    let _value = match &struct_type {
                        Type::Union(f) => {
                            let field_type = f.get(index).ok_or_else(|| FieldIndexOutOfRange(struct_type.clone(), index))?;
                            let field_llvm_type = vm_type_to_llvm_type(field_type, context)?;
                            let ptr = get!(args, struct_llvm_type.ptr_type(AddressSpace::Generic).into(), 0).into_pointer_value();
                            store_operand!(1, builder.build_pointer_cast(ptr, field_llvm_type.ptr_type(AddressSpace::Generic), "field_ptr").into());
                        }
                        _o => return Err(ExceptUnionType(struct_type)),
                    };
                }
                // fn<type ty>(array:Pointer<Array>,index:Usize)->(out:Pointer<Element>)
                LocateElement => {
                    let (ty, llvm_type) = get_type!();
                    let index = load_int_operand!(usize_type, 1);
                    let value = match &ty {
                        Type::Array(_element, Some(_size)) => {
                            let array_type = llvm_type.ptr_type(AddressSpace::Generic);
                            let ptr = get!(args, array_type.ptr_type(AddressSpace::Generic).into(), 0).into_pointer_value();
                            unsafe { builder.build_gep(ptr, &[index], "element_ptr") }
                        }
                        Type::Array(element, None) => {
                            let _element_llvm_type = vm_type_to_llvm_type(element, context)?;
                            let array_type = context.struct_type(&[usize_type.into()], true);
                            let ptr = get!(args, array_type.ptr_type(AddressSpace::Generic).into(), 0).into_pointer_value();
                            let start = builder.build_struct_gep(ptr, 1, "element_ptr").unwrap();
                            unsafe { builder.build_gep(start, &[context.i64_type().const_int(0, false), index], "element_ptr") }
                        }
                        Type::Pointer(_ptr) => {
                            let ptr = get!(args, llvm_type, 0).into_pointer_value();
                            unsafe { builder.build_gep(ptr, &[index], "element_ptr") }
                        }
                        _o => return Err(ExceptArrayType(ty)),
                    };
                    store_operand!(2, value.into());
                }
                // fn<type ty,const TIRE:Usize>(in:Pointer<Object>)->(out:Pointer<Metadata>)
                LocateMetadata => {
                    let (ty, llvm_type) = get_type!();
                    let ptr = get!(args, llvm_type.ptr_type(AddressSpace::Generic).into(), 0);
                    let index = get_int_constant!(context.i32_type(), 1).get_zero_extended_constant().ok_or_else(WroneGenericKind)? as usize;
                    match &ty {
                        Type::MetaData(m) => {
                            let (_, metadata_type) = m.get(index).ok_or_else(|| MetadataIndexOutOfRange(ty.clone(), index))?;
                            let metadata_llvm_type = vm_type_to_llvm_type(metadata_type.get_type()?, context)?;
                            let ptr_int = builder.build_ptr_to_int(ptr.into_pointer_value(), usize_type, "ptr_int");
                            let shift_ptr = builder.build_left_shift(ptr_int, usize_type.const_int(6, false), "shift_ptr");
                            let metadata_ptr_ptr_int = builder.build_int_add(shift_ptr, usize_type.const_int(index as u64, false), "metadata_ptr_ptr_int");
                            let metadata_ptr_ptr = builder.build_int_to_ptr(
                                metadata_ptr_ptr_int,
                                metadata_llvm_type.ptr_type(AddressSpace::Generic).ptr_type(AddressSpace::Generic),
                                "metadata_ptr_ptr",
                            );
                            let metadata_ptr = builder.build_load(metadata_ptr_ptr, "metadata_ptr");
                            store_operand!(1, metadata_ptr);
                        }
                        _o => return Err(ExceptMetadataType(ty)),
                    }
                }
                // fn<type ty>(in:Pointer<Type>)->(out:Type)
                Read => {
                    let (_, llvm_type) = get_type!();
                    let ptr = get!(args, llvm_type.ptr_type(AddressSpace::Generic).into(), 0);
                    store_operand!(1, builder.build_load(ptr.into_pointer_value(), "value"));
                }
                // fn<type ty>(ptr:Pointer<Type>,value:Type)
                Write => {
                    let (_ty, llvm_type) = get_type!();
                    let ptr = get!(args, llvm_type.ptr_type(AddressSpace::Generic).into(), 0);
                    let value = get!(args, llvm_type, 1);
                    builder.build_store(ptr.into_pointer_value(), value);
                }
                // fn<type ty>(ptr:Pointer<Type>,exception:Type,value:Type)->(Bool)
                CompareAndSwap => {
                    let (_ty, llvm_type) = get_type!();
                    let ptr = get!(args, llvm_type.ptr_type(AddressSpace::Generic).into(), 0).into_pointer_value();
                    let exception = get!(args, llvm_type, 1);
                    let value = get!(args, llvm_type, 2);
                    let r = builder
                        .build_cmpxchg(ptr, exception, value, AtomicOrdering::Unordered, AtomicOrdering::Unordered)
                        .map_err(|e| OtherLLVMError(e.to_string()))?;
                    let sucess = builder.build_extract_value(r, 1, "sucess").unwrap();
                    store_operand!(3, sucess);
                }
                FenceReleased => {
                    builder.build_fence(AtomicOrdering::Release, 0, "fence");
                }
                FenceAcquire => {
                    builder.build_fence(AtomicOrdering::Acquire, 0, "fence");
                }
                FenceAcqrel => {
                    builder.build_fence(AtomicOrdering::AcquireRelease, 0, "fence");
                }
                FenceSeqcst => {
                    builder.build_fence(AtomicOrdering::SequentiallyConsistent, 0, "fence");
                }
                // fn<const ty:TYPE>(in:Pointer<Type>)
                Free => {
                    self.generate_instruction_core(&self.memory_instruction_set.free, constants, operands)?;
                }
                // fn<const ty:TYPE>(dst:Pointer<Type>,src:Pointer<Type>,size:Usize)
                MemoryCopy => {
                    let (ty, llvm_type) = get_type!();
                    let layout = ty.get_layout()?;
                    let dst = get!(args, llvm_type.ptr_type(AddressSpace::Generic).into(), 0);
                    let src = get!(args, llvm_type.ptr_type(AddressSpace::Generic).into(), 1);
                    let len = get!(args, usize_type.into(), 2).into_int_value();
                    let size = builder.build_int_mul(len, usize_type.const_int(layout.into_flexible_array().flexible_size() as u64, false), "size");
                    builder
                        .build_memcpy(dst.into_pointer_value(), layout.align().try_into()?, src.into_pointer_value(), layout.align().try_into()?, size)
                        .map_err(|e| OtherLLVMError(e.to_string()))?;
                }
                // fn<type Element,const len:Usize>(elems...:Element)->(o:Array<Element,len>)
                MakeSlice => {
                    if let InstructionType::Bootstrap(MakeSlice) = &self.instruction_type {
                        let len = get_int_constant!(usize_type, 0);
                        let size = get_int_constant!(usize_type, 1);
                        let last_constant_ptr = constants[1].as_ptr(builder)?;
                        let operand_start = builder.build_pointer_cast(
                            unsafe { builder.build_gep(last_constant_ptr, &[usize_type.const_int(1, false)], "operand_start") },
                            context.i16_type().ptr_type(AddressSpace::Shared),
                            "operand_start",
                        );
                        let index_init = usize_type.const_int(0, false);
                        let pre_block = builder.get_insert_block().unwrap();
                        let loop_block = context.append_basic_block(self.function, "loop");
                        let emit_block = context.append_basic_block(self.function, "emit");
                        let regs = self.function.get_nth_param(0).unwrap().into_pointer_value();
                        let array_ptr = 
                                builder.build_pointer_cast(
                                    unsafe {
                                        builder.build_gep(
                                            regs,
                                            &[builder.build_load(builder.build_gep(operand_start, &[len], "array_reg_ptr"), "array_reg").into_int_value()],
                                            "array_ptr",
                                        )
                                    },
                                    context.i8_type().ptr_type(AddressSpace::Local),
                                    "array_ptr",
                                );
                        let slice_ptr = 
                                builder.build_pointer_cast(
                                    unsafe {
                                        builder.build_gep(
                                            regs,
                                            &[builder.build_load(builder.build_gep(operand_start, &[builder.build_int_add(len,usize_type.const_int(1, false),"slice_operand_index")], "slice_reg_ptr"), "slice_reg").into_int_value()],
                                            "slice_ptr",
                                        )
                                    },
                                    context.struct_type(&[context.i8_type().ptr_type(AddressSpace::Local).into(),usize_type.into()],false).ptr_type(AddressSpace::Local),
                                    "slice_ptr",
                                );
                        builder.build_store(builder.build_struct_gep(slice_ptr, 0, "ptr_in_slice").unwrap(), array_ptr);
                        builder.build_store(builder.build_struct_gep(slice_ptr, 1, "len_in_slice").unwrap(), len);
                        builder.build_conditional_branch(builder.build_int_compare(IntPredicate::ULT, index_init, len, "has_element"), loop_block, emit_block);
                        builder.position_at_end(loop_block);
                        let index_phi = builder.build_phi(usize_type, "index");
                        let index = index_phi.as_basic_value().into_int_value();
                        unsafe {
                            builder
                                .build_memcpy(
                                    builder.build_gep(array_ptr, &[builder.build_int_mul(index, size, "offset")], "dest"),
                                    1,
                                    builder.build_gep(
                                        regs,
                                        &[builder.build_load(builder.build_gep(operand_start, &[index], "value_reg_ptr"), "value_reg").into_int_value()],
                                        "value_ptr",
                                    ),
                                    8,
                                    size,
                                )
                                .map_err(|e| OtherLLVMError(e.to_string()))?;
                        }
                        let next_index = builder.build_int_add(index, usize_type.const_int(1, false), "next_index");
                        builder.build_conditional_branch(builder.build_int_compare(IntPredicate::ULT, next_index, len, "has_element"), loop_block, emit_block);
                        index_phi.add_incoming(&[(&index_init, pre_block), (&next_index, loop_block)]);
                        builder.position_at_end(emit_block);
                    } else {
                        let (ty, llvm_type) = get_type!();
                        let layout = ty.get_layout()?;
                        let len = get_int_constant!(usize_type, 1);
                        let len_value = len;
                        let len = len.get_zero_extended_constant().unwrap();
                        let array_ptr = builder.build_array_alloca(llvm_type, len_value, "array_ptr");
                        array_ptr.as_instruction_value().unwrap().set_alignment(layout.align().try_into()?).map_err(|e| OtherLLVMError(e.to_string()))?;
                        for i in 0..len {
                            let value = get!(args, llvm_type, i as usize);
                            unsafe {
                                builder.build_store(builder.build_gep(array_ptr, &[usize_type.const_int(i as u64, false)], "dest"), value);
                            }
                        }
                        let slice_type = context.struct_type(&[llvm_type.ptr_type(AddressSpace::Generic).into(), usize_type.into()], false);
                        let slice = builder.build_insert_value(slice_type.const_zero(), len_value, 1, "slice").unwrap();
                        let slice = builder.build_insert_value(slice, array_ptr, 0, "slice").unwrap().into_struct_value();
                        store_operand!(len as usize, slice.into());
                    }
                }
                // fn<type ty>()->(out:Struct)
                UninitedStruct => {
                    let (_, llvm_type) = get_type!();
                    store_operand!(0, llvm_type.const_zero());
                }
                // fn<type ty>(array:Pointer<Array>)->(len:Usize)
                GetLength => {
                    let (ty, llvm_type) = get_type!();
                    let value = match ty {
                        Type::Array(_element, Some(size)) => usize_type.const_int(size as u64, false),
                        Type::Array(_element, None) => {
                            let ptr = get!(args, llvm_type.ptr_type(AddressSpace::Generic).into(), 0);
                            builder
                                .build_load(
                                    builder.build_pointer_cast(ptr.into_pointer_value(), usize_type.ptr_type(AddressSpace::Generic), "length_ptr"),
                                    "length",
                                )
                                .into_int_value()
                        }
                        o => return Err(ExceptArrayType(o)),
                    };
                    store_int_operand!(1, value);
                }
                // fn<type ty>(array:Pointer<Array>,len:Usize)
                SetLength => {
                    let (ty, llvm_type) = get_type!();
                    match ty {
                        Type::Array(_element, None) => {
                            let ptr = get!(args, llvm_type.ptr_type(AddressSpace::Generic).into(), 0);
                            let length = get!(args, usize_type.into(), 1);
                            builder.build_store(
                                builder.build_pointer_cast(ptr.into_pointer_value(), usize_type.ptr_type(AddressSpace::Generic), "length_ptr"),
                                length.into_int_value(),
                            );
                        }
                        o => return Err(ExceptArrayType(o)),
                    };
                }
                SetState => match self.state_stack.last() {
                    Some(StateInstructionBuilder { instruction: instructoin, state_kind }) => {
                        let state_name = constants.get(0).ok_or(GenericIndexOutOfRange(0))?.get_state()?;
                        let (state_index, _state) = instructoin
                            .statuses
                            .iter()
                            .enumerate()
                            .find(|(_i, s)| &s.name == state_name)
                            .ok_or_else(|| StateNotFound(state_name.to_string()))?;
                        match state_kind {
                            StateKind::StateConstant(constant) => {
                                let ip = constant.as_ptr(builder)?;
                                builder.build_store(ip, ip.get_type().get_element_type().into_int_type().const_int(state_index as u64, false));
                            }
                            StateKind::Opcode(ip, start) => {
                                builder.build_store(*ip, ip.get_type().get_element_type().into_int_type().const_int((start + state_index) as u64, false));
                            }
                        }
                    }
                    None => return Err(IllegalSetStateInstructin()),
                },
                CallState => match self.state_stack.last() {
                    Some(StateInstructionBuilder { instruction, state_kind: _ }) => {
                        let state_name = constants.get(0).ok_or(GenericIndexOutOfRange(0))?.get_state()?;
                        let (_state_index, state) = instruction
                            .statuses
                            .iter()
                            .enumerate()
                            .find(|(_i, s)| &s.name == state_name)
                            .ok_or_else(|| StateNotFound(state_name.to_string()))?;
                        let state_instruction=&state.instruction.clone();
                        self.generate_complex_instruction_core(state_instruction, &constants[1..], operands)?;
                    }
                    None => return Err(IllegalSetStateInstructin()),
                },
                // fn<type ty>()->(out:Pointer<Type>)
                StackAlloc => {
                    let (_, llvm_type) = get_type!();
                    let ptr = builder.build_alloca(llvm_type, "stack_value_ptr");
                    store_operand!(0, ptr.into());
                }
                StackAllocUnsized => {
                    let (ty, llvm_type) = get_type!();
                    let len = get!(args, usize_type.into(), 0).into_int_value();
                    let layout = ty.get_layout()?;
                    let size = builder.build_int_add(
                        usize_type.const_int(layout.size() as u64, false),
                        builder.build_int_mul(len, usize_type.const_int(layout.flexible_size() as u64, false), "var_size"),
                        "size",
                    );
                    let ptr = builder.build_array_alloca(context.i8_type(), size, "stack_value_ptr");
                    let ptr_cast = builder.build_pointer_cast(ptr, llvm_type.ptr_type(AddressSpace::Generic), "stack_value_ptr_cast");
                    store_operand!(1, ptr_cast.into());
                }
                GetPointer => {
                    let (_, llvm_type) = get_type!();
                    let ptr = operands.get(0).ok_or(ArgumentIndexOutOfRange(0))?.get_ptr(&self.builder, self.registers, llvm_type)?;
                    store_operand!(1, ptr.into());
                }
            }
            match bootstrap {
                Return => {
                    self.returned = true;
                }
                _ => {}
            }
            match bootstrap {
                Return | Branch | BranchIf | Invoke => {
                    self.termined = true;
                }
                _ => {}
            }
            Ok(())
        })()
        .map_err(|e| ErrorWhileGenerateBoostrapInstruction(*bootstrap, Box::new(e)))?;
        Ok(())
    }

    fn generate_complex_instruction_core(
        &mut self,
        complex_instruction: &ComplexInstruction,
        constants: &[Constant<'ctx>],
        operands: &mut [Operand<'ctx>],
    ) -> Result<()> {
        let instruction_name = complex_instruction.name.to_string();
        (|| {
            let context = self.context;
            let mut operands_map = HashMap::<String, Operand>::from_iter(
                complex_instruction.metadata.operands.iter().map(|operand_metadata| (operand_metadata.name.to_string())).zip(operands.iter().cloned()),
            );
            let constants_map: HashMap<String, Constant<'ctx>> = HashMap::<String, Constant<'ctx>>::from_iter(
                complex_instruction.metadata.generics.iter().map(|constant: &GenericsMetadata| (constant.name.to_string())).zip(constants.iter().cloned()),
            );
            let mut basic_blocks = HashMap::<Cow<str>, LLVMBasicBlockBuilderRef<'ctx>>::new();
            for (_block_index, basic_block) in complex_instruction.blocks.iter().enumerate() {
                let variables;
                let mut phis = HashMap::new();
                let block = if &basic_block.id == "entry" {
                    variables = operands_map.clone();
                    self.builder.get_insert_block().unwrap()
                } else {
                    variables = HashMap::new();
                    context.append_basic_block(self.function, &format!("{}_{}", &instruction_name, &*basic_block.id))
                };
                self.builder.position_at_end(block);
                for phi in &basic_block.phi {
                    match phis.entry(phi.variable.to_string()) {
                        std::collections::hash_map::Entry::Occupied(_) => todo!(),
                        std::collections::hash_map::Entry::Vacant(v) => {
                            v.insert((
                                self.builder.build_phi(
                                    vm_type_to_llvm_type(&phi.ty, context)?,
                                    &format!("{}__{}___phi_{}", &instruction_name, &basic_block.id, &*phi.variable),
                                ),
                                phi.ty.clone(),
                                Some(phi.clone()),
                            ));
                        }
                    }
                }
                let builder = Rc::new(RefCell::new(LLVMBasicBlockBuilder {
                    block,
                    end_block: None,
                    phis,
                    termined: false,
                    variables,
                    branchs: Default::default(),
                    name: basic_block.id.to_string(),
                }));
                basic_blocks.insert(basic_block.id.to_owned(), builder);
            }
            let mut all_block_termined = true;
            let mut all_block_returned = true;
            let mut exit = None;
            for (block_index, basic_block) in complex_instruction.blocks.iter().enumerate() {
                let basic_block: &runtime::instructions::BasicBlock = basic_block;
                let llvm_basic_block_builder = basic_blocks.get(&*basic_block.id).unwrap();
                (|| {
                    let mut variables: HashMap<String, Operand> = llvm_basic_block_builder.borrow().variables.clone();
                    let mut builder = context.create_builder();
                    let phi_builder = context.create_builder();
                    builder.position_at_end(llvm_basic_block_builder.borrow().block);
                    phi_builder.position_at_end(llvm_basic_block_builder.borrow().block);
                    let mut termined = false;
                    let mut returned = false;
                    for (stat_index, stat) in basic_block.stat.iter().enumerate() {
                        if termined || returned {
                            return Err(TerminedBasicBlock());
                        }
                        match stat {
                            Stat::Move(to, from) => {
                                let name = from;
                                let operand: Operand = {
                                    if let Some(variable) = variables.get(&**name) {
                                        variable.clone()
                                    } else if let Some(operand) = operands_map.get(&**name) {
                                        operand.clone()
                                    } else if let Some(constant) = constants_map.get(&**name) {
                                        match constant {
                                            Constant::Ptr(p, t) => Operand::Value((*p).into(), Type::Pointer(CowArc::new(t.clone()))),
                                            Constant::Value(v, t) => Operand::Value(*v, t.clone()),
                                            _ => return Err(ThisGenericCanNotUseAsArgument()),
                                        }
                                    } else {
                                        return Err(TypeIsUnnkown());
                                    }
                                };
                                variables.insert(to.to_string(), operand);
                                return Err(NotSupported());
                            }
                            Stat::Lit(name, value) => {
                                let (value, ty) = convert_value(value, context)?;
                                let operand = Operand::Value(value, ty);
                                if let Some((operand_index, _operand_metadata)) =
                                    complex_instruction.metadata.operands.iter().enumerate().find(|(_i, m)| m.output && &*m.name == &*name)
                                {
                                    operands_map.get_mut(&**name).ok_or(ArgumentIndexOutOfRange(operand_index))?.store(&builder, value)?;
                                    operands.get_mut(operand_index).ok_or(ArgumentIndexOutOfRange(operand_index))?.store(&builder, value)?;
                                    variables
                                        .insert(name.to_string(), operands.get(operand_index).ok_or(ArgumentIndexOutOfRange(operand_index))?.clone());
                                } else {
                                    variables.insert(name.to_string(), operand);
                                }
                            }
                            Stat::InstructionCall(call) => {
                                let instruction_type = &call.instruction;
                                let mut new_constants: Vec<Constant<'ctx>> = Vec::with_capacity(call.generics.len());
                                (|| {
                                    for gen in &*call.generics {
                                        let constant: Constant<'ctx> = match gen {
                                            GenericArgument::Var(name) => {
                                                if let Some(constant) = constants_map.get(&**name) {
                                                    constant.clone()
                                                } else if let Some(basic_block) = basic_blocks.get(&**name) {
                                                    Constant::BasicBlock(TargetBlock::Block {
                                                        block: basic_block.clone(),
                                                        from: llvm_basic_block_builder.clone(),
                                                    })
                                                } else if let Some(_state) = self
                                                    .state_stack
                                                    .last()
                                                    .and_then(|stateful| stateful.instruction.statuses.iter().find(|state| &state.name == name))
                                                {
                                                    Constant::State(name.to_string())
                                                } else {
                                                    return Err(MissGeneric(name.to_string()));
                                                }
                                            }
                                            GenericArgument::Value(Value::Type(ty)) => Constant::Type(ty.clone(), vm_type_to_llvm_type(ty, context)?),
                                            GenericArgument::Value(Value::Instruction(instruction)) => Constant::Instruction(instruction.clone()),
                                            GenericArgument::Value(value) => {
                                                let (value, ty) = convert_value(value, context)?;
                                                Constant::Value(value, ty)
                                            }
                                        };
                                        new_constants.push(constant);
                                    }
                                    Ok(())
                                })()
                                .map_err(|e| ErrorWhileGenerateInstructionCall(stat_index, Box::new(e), format!("{:?}", call), String::new()))?;
                                let instruction_matedata =
                                    get_instruction_metadata(instruction_type, &*new_constants, self.state_stack.last().map(|s| &s.instruction), false)
                                        .map_err(|e| ErrorWhileGenerateInstructionCall(stat_index, Box::new(e), format!("{:?}", call), String::new()))?;
                                let mut new_operands = Vec::with_capacity(call.args.len() + call.rets.len());
                                (|| {
                                    let mut arg_iter = call.args.iter();
                                    let mut ret_iter = call.rets.iter();
                                    for (operand_index, operand_metadata) in instruction_matedata.operands.iter().enumerate() {
                                        (|| {
                                            let operand: Operand = if operand_metadata.input {
                                                let arg_name = arg_iter.next().ok_or(ArgumentIndexOutOfRange(operand_index))?;
                                                let mut ret_name = None;
                                                if operand_metadata.output {
                                                    ret_name = ret_iter.next();
                                                }
                                                if let Some(variable) = variables.get(&**arg_name) {
                                                    let operand_type = variable.get_type();
                                                    if operand_type != &operand_metadata.value_type {
                                                        return Err(TypeNotMatch(operand_metadata.value_type.clone(), operand_type.clone()));
                                                    }
                                                    variable.clone()
                                                } else if let Some(operand) = operands_map.get(&**arg_name) {
                                                    if operand_metadata.output {}
                                                    let operand_type = operand.get_type();
                                                    if operand_type != &operand_metadata.value_type {
                                                        return Err(TypeNotMatch(operand_metadata.value_type.clone(), operand_type.clone()));
                                                    }
                                                    if ret_name.map(|r| &**r) == Some(&**arg_name) {
                                                        operand.clone()
                                                    } else {
                                                        Operand::Value(
                                                            operand.load(&builder, vm_type_to_llvm_type(operand_type, context)?)?,
                                                            operand_type.clone(),
                                                        )
                                                    }
                                                } else if let Some(constant) = constants_map.get(&**arg_name) {
                                                    match constant {
                                                        Constant::Ptr(p, t) => Operand::Value((*p).into(), Type::Pointer(CowArc::new(t.clone()))),
                                                        Constant::Value(v, t) => Operand::Value(*v, t.clone()),
                                                        _ => return Err(ThisGenericCanNotUseAsArgument()),
                                                    }
                                                } else {
                                                    let _llvm_type = vm_type_to_llvm_type(&operand_metadata.value_type, context)?;
                                                    let mut llvm_basic_block_builder_ref = llvm_basic_block_builder.borrow_mut();
                                                    let llvm_block = llvm_basic_block_builder_ref.block;
                                                    let (phi, phi_type, _phi_instruction) = match llvm_basic_block_builder_ref.phis.entry(arg_name.to_string()) {
                                                        std::collections::hash_map::Entry::Occupied(o) => o.get().clone(),
                                                        std::collections::hash_map::Entry::Vacant(v) => {
                                                            if let Some(first_instruction) = llvm_block.get_first_instruction() {
                                                                phi_builder.position_before(&first_instruction);
                                                            }
                                                            let phi = phi_builder.build_phi(
                                                                vm_type_to_llvm_type(&operand_metadata.value_type, context)?,
                                                                &format!("{}__{}__{}", &instruction_name, &basic_block.id, &*arg_name),
                                                            );
                                                            variables.insert(
                                                                arg_name.to_string(),
                                                                Operand::Value(phi.as_basic_value(), operand_metadata.value_type.clone()),
                                                            );
                                                            v.insert((phi, operand_metadata.value_type.clone(), None)).clone()
                                                        }
                                                    };
                                                    Operand::Value(phi.as_basic_value(), phi_type)
                                                }
                                            } else if operand_metadata.output {
                                                let ret_name = ret_iter.next().ok_or(ArgumentIndexOutOfRange(operand_index))?;
                                                if let Some(operand) = operands_map.get(&**ret_name) {
                                                    operand.clone()
                                                } else {
                                                    Operand::Uninitialized(operand_metadata.value_type.clone())
                                                }
                                            } else {
                                                return Err(InvalidOperandMetadata());
                                            };
                                            new_operands.push(operand);
                                            Ok(())
                                        })()
                                        .map_err(|e| ErrorWhileFindingOperand(operand_index, Box::new(e)))?;
                                    }
                                    Ok(())
                                })()
                                .map_err(|e| {
                                    ErrorWhileGenerateInstructionCall(stat_index, Box::new(e), format!("{:?}", call), format!("{:?}", instruction_matedata))
                                })?;
                                let mut instruction_builder = LLVMFunctionBuilder::<'_> {
                                    instruction_type: self.instruction_type.clone(),
                                    builder,
                                    context: self.context,
                                    ip: self.ip,
                                    deploy_table: self.deploy_table,
                                    global: self.global.clone(),
                                    function: self.function,
                                    registers: self.registers,
                                    termined: false,
                                    state_stack: self.state_stack.clone(),
                                    memory_instruction_set: self.memory_instruction_set,
                                    ip_phi: self.ip_phi,
                                    exit: self.exit,
                                    returned: self.returned,
                                };

                                instruction_builder.generate_instruction_core(instruction_type, &*new_constants, &mut *new_operands).map_err(|e| {
                                    ErrorWhileGenerateInstructionCall(stat_index, Box::new(e), format!("{:?}", call), format!("{:?}", instruction_matedata))
                                })?;
                                (|| {
                                    let mut ret_iter = call.rets.iter();
                                    for (index, (operand_metadata, new)) in instruction_matedata.operands.iter().zip(new_operands.into_iter()).enumerate() {
                                        (|| {
                                            if operand_metadata.output {
                                                let ret_name = ret_iter.next().ok_or(ArgumentIndexOutOfRange(index))?;
                                                match &new {
                                                    Operand::Uninitialized(_ty) => {
                                                        return Err(VariableNotInitialized(ret_name.to_string()));
                                                    }
                                                    Operand::Value(v, _) => {
                                                        if v.as_instruction_value().map(|i| i.get_opcode() == InstructionOpcode::Phi) == Some(false) {
                                                            v.set_name(&format!("{}__{}__{}", &instruction_name, &basic_block.id, &ret_name));
                                                        }
                                                    }
                                                    _ => {}
                                                }
                                                let operand_type = new.get_type();
                                                if operand_type != &operand_metadata.value_type {
                                                    return Err(TypeNotMatch(operand_metadata.value_type.clone(), operand_type.clone()));
                                                }
                                                let llvm_type = new.get_llvm_type()?;
                                                if llvm_type != vm_type_to_llvm_type(&operand_metadata.value_type, context)?.as_any_type_enum() {
                                                    return Err(LLVMTypeNotMatch(
                                                        format!("{:?}", llvm_type),
                                                        format!("{:?}", vm_type_to_llvm_type(&operand_metadata.value_type, context)?),
                                                    ));
                                                }
                                                variables.insert(ret_name.to_string(), new);
                                            };
                                            Ok(())
                                        })()
                                        .map_err(|e| ErrorWhileWriteBackOperand(index, Box::new(e)))?;
                                    }
                                    Ok(())
                                })()
                                .map_err(|e| {
                                    ErrorWhileGenerateInstructionCall(stat_index, Box::new(e), format!("{:?}", call), format!("{:?}", instruction_matedata))
                                })?;
                                builder = instruction_builder.builder;
                                self.ip = instruction_builder.ip;
                                if returned {
                                    let _ = termined;
                                }
                                termined |= instruction_builder.termined;
                                returned |= instruction_builder.returned;
                            }
                        }
                    }
                    llvm_basic_block_builder.borrow_mut().end_block = builder.get_insert_block();
                    if !termined && complex_instruction.blocks.len() != 1 {
                        let exit_block =
                            *exit.get_or_insert_with(|| self.context.append_basic_block(self.function, &format!("{}__exit", &instruction_name)));
                        builder.build_unconditional_branch(exit_block);
                        builder.clear_insertion_position();
                    }
                    all_block_termined &= termined;
                    all_block_returned &= returned;
                    let mut llvm_basic_block_builder_ref = llvm_basic_block_builder.borrow_mut();
                    llvm_basic_block_builder_ref.variables = variables;
                    llvm_basic_block_builder_ref.termined = termined;
                    self.builder = builder;
                    Ok(())
                })()
                .map_err(|e| ErrorWhileGenerateBasicBlock(block_index, Box::new(e), format!("{:#?}", basic_block)))?;
            }
            let builder = &self.builder;

            let mut exit_phis: HashMap<_, _> = HashMap::new();
            if let Some(exit) = exit {
                builder.position_at_end(exit);
                for operand_metadata in complex_instruction.metadata.operands.iter().filter(|operand| operand.output) {
                    match operands_map.get(&*operand_metadata.name).unwrap() {
                        Operand::Register(_, _) => {}
                        Operand::Uninitialized(_) | Operand::Value(_, _) => {
                            exit_phis.insert(
                                operand_metadata.name.to_string(),
                                (
                                    builder.build_phi(
                                        vm_type_to_llvm_type(&operand_metadata.value_type, context)?,
                                        &format!("ret__{}__{}", &instruction_name, &*operand_metadata.name),
                                    ),
                                    operand_metadata.value_type.clone(),
                                ),
                            );
                        }
                    }
                }
            } else {
            }
            let mut scan_stack = Vec::new();
            for (block_index, basic_block) in complex_instruction.blocks.iter().enumerate() {
                let root_block = basic_blocks.get(&*basic_block.id).unwrap();
                (|| {
                    let variables = root_block.borrow().variables.clone();
                    let root_llvm_block = root_block.borrow().block;
                    for (llvm_block, branch) in &root_block.borrow().branchs {
                        scan_stack.push((root_llvm_block, root_block.clone(), variables.clone(), *llvm_block, branch.clone()));
                    }
                    let llvm_basic_block_builder_ref = root_block.borrow();
                    if let Some(_exit) = exit {
                        if !llvm_basic_block_builder_ref.termined {
                            for (exit_phi_name, (exit_phi, _ty)) in exit_phis.iter() {
                                if let Some(variable) = llvm_basic_block_builder_ref.variables.get(exit_phi_name) {
                                    match variable {
                                        Operand::Register(_, _) => {}
                                        Operand::Value(v, _) => {
                                            exit_phi.add_incoming(&[(v, llvm_basic_block_builder_ref.end_block.unwrap())]);
                                        }
                                        Operand::Uninitialized(_) => {
                                            return Err(VariableNotInitialized(exit_phi_name.to_string()));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Ok(())
                })()
                .map_err(|e| ErrorWhileGenerateBasicBlock(block_index, Box::new(e), format!("{:#?}", basic_block)))?;
            }
            let mut mark = HashSet::new();
            while let Some((root, from, mut variables, incoming_llvm_basic_block, to)) = scan_stack.pop() {
                if !mark.contains(&(root, from.borrow().block, to.borrow().block)) {
                    builder.position_before(&incoming_llvm_basic_block.get_last_instruction().unwrap());
                    for (phi_name, (phi, phi_type, phi_instruction)) in &to.borrow().phis.clone() {
                        let var_name = if let Some(phi_instruction) = phi_instruction {
                            let block_name = from.borrow().name.clone();
                            &*phi_instruction
                                .map
                                .iter()
                                .find(|(from, _var)| from == &block_name)
                                .ok_or_else(|| PhiInstructionMissBlock(phi_name.to_string(), block_name))?
                                .1
                        } else {
                            &**phi_name
                        };
                        if let Some(var) = variables.get(var_name) {
                            phi.add_incoming(&[(&var.load(builder, vm_type_to_llvm_type(phi_type, context)?)?, incoming_llvm_basic_block)]);
                        }
                    }
                    {
                        let to = to.borrow();
                        variables.retain(|k, _| !to.variables.contains_key(k));
                    }
                    if let Some(exit) = exit {
                        let to = to.borrow();
                        if !to.termined {
                            for (exit_phi_name, (exit_phi, _ty)) in exit_phis.iter() {
                                if let Some(variable) = variables.get(exit_phi_name) {
                                    match variable {
                                        Operand::Register(_, _) => {}
                                        Operand::Value(v, _) => {
                                            exit_phi.add_incoming(&[(v, exit)]);
                                        }
                                        Operand::Uninitialized(_) => {
                                            return Err(VariableNotInitialized(exit_phi_name.to_string()));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if !variables.is_empty() {
                        for (llvm_basic_block, branch) in &to.borrow().branchs {
                            scan_stack.push((root, to.clone(), variables.clone(), *llvm_basic_block, branch.clone()));
                        }
                    }
                    mark.insert((root, from.borrow().block, to.borrow().block));
                }
            }
            self.termined = all_block_termined;
            self.returned = all_block_returned;
            if let Some(exit) = exit {
                assert!(complex_instruction.blocks.len() != 1);
                for (operand, operand_metadata) in operands.iter_mut().zip(complex_instruction.metadata.operands.iter()) {
                    if let Some((phi, ty)) = exit_phis.remove(&*operand_metadata.name) {
                        *operand = Operand::Value(phi.as_basic_value(), ty);
                    }
                }
                builder.position_at_end(exit);
            } else {
                if let Some(basic_block) = complex_instruction.blocks.first() {
                    let llvm_basic_block_builder = basic_blocks.get(&*basic_block.id).unwrap();
                    (|| {
                        let llvm_basic_block_builder = llvm_basic_block_builder.borrow_mut();
                        if !llvm_basic_block_builder.termined {
                            let variables = &llvm_basic_block_builder.variables;
                            for (operand, operand_metadata) in operands.iter_mut().zip(complex_instruction.metadata.operands.iter()) {
                                if operand_metadata.output {
                                    *operand = variables
                                        .get(&*operand_metadata.name)
                                        .ok_or_else(|| VariableNotFound(operand_metadata.name.to_string()))?
                                        .clone();
                                }
                            }
                        }
                        Ok(())
                    })()
                    .map_err(|e| ErrorWhileGenerateBasicBlock(0, Box::new(e), format!("{:#?}", basic_block)))?;
                }
                if let Some(llvm_basic_block) = builder.get_insert_block() {
                    builder.position_at_end(llvm_basic_block);
                }
            }
            for (operand_metadata, operand) in complex_instruction.metadata.operands.iter().zip(operands.iter()) {
                match operand {
                    Operand::Uninitialized(_) => {
                        return Err(VariableNotInitialized(operand_metadata.name.to_string()));
                    }
                    _ => {}
                }
            }
            Ok(())
        })()
        .map_err(|e| ErrorWhileGenerateComplexInstruction(Box::new(e), format!("{:#?}", complex_instruction)))
    }

    fn generate_compress_instruction_core(
        &mut self,
        compress_instruction: &CompressionInstruction,
        constant: &[Constant<'ctx>],
        operand: &mut [Operand<'ctx>],
    ) -> Result<()> {
        let context = self.context;
        let builder = &self.builder;

        let opcode = constant.get(0).ok_or(ArgumentIndexOutOfRange(0))?.get_value(&self.builder)?.into_int_value();
        if let InstructionType::Complex(_) = &self.instruction_type {
            let (_index, sub_instruction) = &compress_instruction.instructions[opcode.get_zero_extended_constant().unwrap() as usize];
            return self.generate_instruction_core(sub_instruction, &constant[1..], operand);
        } else if !self.termined {
            let table = Self::generate_instruction_set(
                &*compress_instruction.instructions,
                compress_instruction.instruction_count,
                context,
                self.global.clone(),
                self.memory_instruction_set,
                "compress_instruction",
            )?;
            let sub_instructions_function_address = unsafe {
                builder.build_in_bounds_gep(table.as_pointer_value(), &[context.i64_type().const_int(0, true), opcode], "sub_instructions_function_address")
            };
            let sub_instruction: CallableValue<'ctx> =
                builder.build_load(sub_instructions_function_address, "next_instruction_address").into_pointer_value().try_into().unwrap();
            let call = builder.build_call(sub_instruction, &[self.function.get_nth_param(0).unwrap().into(), self.ip.into()], "call_next_instruction");
            call.set_tail_call(true);
            builder.build_return(Some(&call.try_as_basic_value().unwrap_left()));
            self.termined = true;
        }
        Ok(())
    }

    fn generate_stateful_instruction_core(
        &mut self,
        stateful_instruction: &StatefulInstruction,
        constant: &[Constant<'ctx>],
        operand: &mut [Operand<'ctx>],
    ) -> Result<()> {
        let context = self.context;
        let state_constant = constant.last().ok_or(GenericIndexOutOfRange(0))?;
        let state = state_constant.get_int_value(&self.builder)?;
        self.state_stack
            .push(StateInstructionBuilder { instruction: stateful_instruction.clone(), state_kind: StateKind::StateConstant(state_constant.clone()) });
        let mut cases = Vec::new();
        let switch_block = self.builder.get_insert_block().unwrap();
        let post_block = context.append_basic_block(self.function, "post_state_instruction");
        for (i, state) in stateful_instruction.statuses.iter().enumerate() {
            let basic_block = self.context.append_basic_block(self.function, &format!("state_{}", &*state.name));
            let builder = context.create_builder();
            builder.position_at_end(basic_block);
            self.generate_complex_instruction_core(&state.instruction, constant, operand)?;
            cases.push((context.i8_type().const_int(i as u64, false), basic_block));
        }
        let builder = &self.builder;
        builder.position_at_end(switch_block);
        builder.build_switch(state, post_block, &*cases);
        builder.position_at_end(post_block);
        Ok(())
    }

    fn generate_instruction_core(&mut self, instruction_type: &InstructionType, constants: &[Constant<'ctx>], operands: &mut [Operand<'ctx>]) -> Result<()> {
        match instruction_type {
            InstructionType::Bootstrap(bootstrap) => {
                self.generate_boostrap_instruction_core(bootstrap, constants, operands)?;
            }
            InstructionType::Complex(complex_instruction) => {
                self.generate_complex_instruction_core(complex_instruction, constants, operands)?;
            }
            InstructionType::Stateful(stateful_instruction) => {
                self.generate_stateful_instruction_core(stateful_instruction, constants, operands)?;
            }
            InstructionType::Compression(compress_instruction) => {
                self.generate_compress_instruction_core(compress_instruction, constants, operands)?;
            }
        }
        Ok(())
    }

    pub(crate) fn generate_instruction_set(
        instructions: &[(usize, InstructionType)],
        instruction_count: usize,
        context: &'ctx Context,
        global: Rc<RefCell<GlobalBuilder<'ctx>>>,
        memory_instruction_set: &'ctx MemoryInstructionSet,
        name: &str,
    ) -> Result<GlobalValue<'ctx>> {
        let instruction_count = instruction_count.try_into()?;
        let deploy_table_entry_type = get_instruction_function_type(context).ptr_type(AddressSpace::Generic);
        let deploy_table_type = deploy_table_entry_type.array_type(instruction_count);
        let instruction_function_pointers = global.borrow().module.add_global(deploy_table_type, Some(AddressSpace::Generic), name);
        let mut deploy_table_value: Vec<PointerValue<'ctx>> = Vec::with_capacity(instruction_count as usize);
        for (index, (_opcode, instruction)) in instructions.iter().enumerate() {
            match instruction {
                InstructionType::Stateful(stateful_instruction) => {
                    let _metadata = stateful_instruction.metadata.clone();
                    let start = index;
                    for (index, state) in stateful_instruction.statuses.iter().enumerate() {
                        let state_instruction: InstructionType = InstructionType::Complex(CowArc::new(state.instruction.clone()));
                        let instruction_function = Self::generate_instruction(
                            &state_instruction,
                            global.clone(),
                            instruction_function_pointers,
                            Some((&*stateful_instruction, start)),
                            memory_instruction_set,
                            &format!("instruction_{}", instruction.get_name()),
                        )
                        .map_err(|e| ErrorWhileGenerateInstruction(start + index, Box::new(e)))?;
                        deploy_table_value.push(instruction_function.as_global_value().as_pointer_value());
                    }
                }
                _ => {
                    let instruction_function = Self::generate_instruction(
                        &*instruction,
                        global.clone(),
                        instruction_function_pointers,
                        None,
                        memory_instruction_set,
                        &format!("instruction_{}", instruction.get_name()),
                    )
                    .map_err(|e| ErrorWhileGenerateInstruction(index, Box::new(e)))?;
                    if !instruction_function.verify(true) {
                        return Err(LLVMVerifyFailed(instruction_function.print_to_string().to_string()));
                    };
                    deploy_table_value.push(instruction_function.as_global_value().as_pointer_value());
                }
            }
        }
        if deploy_table_value.len() != instruction_count as usize {
            return Err(WroneInstructionCount(deploy_table_value.len(), instruction_count as usize));
        }
        instruction_function_pointers.set_initializer(&deploy_table_entry_type.const_array(&*deploy_table_value));
        Ok(instruction_function_pointers)
    }

    fn generate_instruction(
        instruction_type: &InstructionType,
        global: Rc<RefCell<GlobalBuilder<'ctx>>>,
        deploy_table: GlobalValue<'ctx>,
        state_instruction_type: Option<(&StatefulInstruction, usize)>,
        memory_instruction_set: &'ctx MemoryInstructionSet,
        name: &str,
    ) -> Result<FunctionValue<'ctx>> {
        let (context, module) = {
            let global_ref = global.borrow();
            (global_ref.context, &global_ref.module.clone())
        };
        let metadata = &*get_instruction_metadata(instruction_type, &[], None, true)?;
        let function_type = get_instruction_function_type(context);
        let function = module.add_function(name, function_type, None);
        function.set_call_conventions(8); // fastcc
        let basic_block = context.append_basic_block(function, "entry");
        let builder = context.create_builder();
        builder.position_at_end(basic_block);
        let mut constant_offset_list = Vec::new();
        let mut constant_layout = Layout::new::<()>();
        for constant_metadata in &*metadata.generics {
            let layout = &mut constant_layout;
            let value_type = get_constant_type(constant_metadata, context)?;
            let (new_layout, offset) = layout.extend(value_type.get_layout()?.into())?;
            *layout = new_layout;
            constant_offset_list.push(offset);
        }
        let mut operand_offset_list = Vec::new();
        for _operand_metadata in &*metadata.operands {
            let (new_layout, offset) = constant_layout.extend(Layout::new::<u16>())?;
            constant_layout = new_layout;
            operand_offset_list.push(offset);
        }
        let align = constant_layout.align();
        let registers = function.get_nth_param(0).unwrap().into_pointer_value();
        let ip = function.get_nth_param(1).unwrap().into_pointer_value();
        let ip_int = builder.build_ptr_to_int(ip, context.i64_type(), "ip_int");
        let constant_address = builder.build_int_add(
            builder.build_or(ip_int, context.i64_type().const_int(align as u64 - 1, false), "align_m1"),
            context.i64_type().const_int(1, false),
            "aligned_ip",
        );
        let mut constant_list = Vec::new();
        for (_index, (constant_metadata, offset)) in metadata.generics.iter().zip(&constant_offset_list).enumerate() {
            
            let constant_ptr = builder.build_int_add(constant_address, context.i64_type().const_int(*offset as u64, true), "constant_ptr");
            match &constant_metadata.kind {
                GenericsMetadataKind::Constant { value_type, writable } => {
                    if *writable {
                        let constant_ptr_cast = builder.build_int_to_ptr(
                            constant_ptr,
                            vm_type_to_llvm_type(&get_constant_type(constant_metadata, context)?, context)?.ptr_type(AddressSpace::Generic),
                            "constant_ptr_cast",
                        );
                        let constant = Constant::Ptr(constant_ptr_cast, value_type.clone());
                        constant_list.push(constant);
                    } else {
                        let constant_ptr_cast = builder.build_int_to_ptr(
                            constant_ptr,
                            vm_type_to_llvm_type(&get_constant_type(constant_metadata, context)?, context)?.ptr_type(AddressSpace::Shared),
                            "constant_ptr_cast",
                        );
                        let constant =
                            Constant::Value(builder.build_load(constant_ptr_cast, &format!("constant_{}", &*constant_metadata.name)), value_type.clone());
                        constant_list.push(constant);
                    }
                }
                GenericsMetadataKind::BasicBlock => {
                    let constant_ptr_cast = builder.build_int_to_ptr(constant_ptr, context.i32_type().ptr_type(AddressSpace::Shared), "constant_ptr_cast");
                    let constant = Constant::BasicBlock(TargetBlock::Offset(constant_ptr_cast));
                    constant_list.push(constant);
                }
                GenericsMetadataKind::Type => todo!(),
                GenericsMetadataKind::State => todo!(),
            }
        }
        let mut operand_list = Vec::new();
        for (index, (operand_metadata, offset)) in metadata.operands.iter().zip(&operand_offset_list).enumerate() {
            let operand_index_address = builder.build_int_add(constant_address, context.i64_type().const_int(*offset as u64, true), "remote_constant");

            let operand_index_ptr = builder.build_int_to_ptr(operand_index_address, context.i16_type().ptr_type(AddressSpace::Shared), "constant_ptr_cast");
            let operand_index = builder.build_load(operand_index_ptr, &format!("operand_{}", index)).into_int_value();
            let ptr = unsafe { builder.build_in_bounds_gep(registers, &[operand_index], "reg_ptr") };
            let value_type = vm_type_to_llvm_type(&operand_metadata.value_type, context)?;
            let ptr_cast = builder.build_pointer_cast(ptr, value_type.ptr_type(AddressSpace::Local), "reg_ptr_cast");
            operand_list.push(Operand::Register(ptr_cast, operand_metadata.value_type.clone()));
        }
        let deploy_table_ptr = deploy_table.as_pointer_value();
        let exit = context.append_basic_block(function, "exit");
        let exit_block_builder = context.create_builder();
        exit_block_builder.position_at_end(exit);
        let ip_phi = exit_block_builder.build_phi(context.i8_type().ptr_type(AddressSpace::Global), "ip");
        let mut this = LLVMFunctionBuilder {
            context: &*context,
            builder,
            ip,
            deploy_table: deploy_table_ptr,
            global,
            function,
            instruction_type: instruction_type.clone(),
            registers,
            termined: false,
            state_stack: Default::default(),
            memory_instruction_set,
            exit,
            returned: false,
            ip_phi,
        };
        if let Some((stateful, start)) = state_instruction_type {
            this.state_stack.push(StateInstructionBuilder { instruction: stateful.clone(), state_kind: StateKind::Opcode(ip, start) });
        }
        this.generate_instruction_core(instruction_type, &constant_list, &mut operand_list)?;
        for operand in operand_list {
            if !(matches!(operand, Operand::Register(_, _))) {
                dbg!(operand);
                Err(format_err!("output is not register"))?;
            }
        }
        if !this.termined {
            ip_phi.add_incoming(&[(&this.ip, this.builder.get_insert_block().unwrap())]);
            this.builder.build_unconditional_branch(exit);
            this.builder.clear_insertion_position();
        } else {
        }
        if !this.returned {
            let builder = exit_block_builder;
            let next_ip = builder.build_int_add(constant_address, context.i64_type().const_int(constant_layout.size() as u64, false), "next_ip");
            let next_ip=if matches!(instruction_type,InstructionType::Bootstrap(BootstrapInstruction::MakeSlice)){
                builder.build_int_add(next_ip, builder.build_int_mul(builder.build_int_add(constant_list[0].get_value(&builder)?.into_int_value(), context.custom_width_int_type(usize::BITS).const_int(2, false), "operand_count"),context.custom_width_int_type(usize::BITS).const_int(2, false) , "operand_len"), "next_ip")
            }else{next_ip};
            let next_instruction_opcode_ptr = builder.build_int_to_ptr(next_ip, context.i8_type().ptr_type(AddressSpace::Global), "next_ip_cast");
            let next_instruction_opcode = builder.build_load(next_instruction_opcode_ptr, "next_instruction_opcode").into_int_value();
            let next_instruction_opcode_usize =
                builder.build_int_z_extend(next_instruction_opcode, context.custom_width_int_type(usize::BITS), "next_instruction_opcode_usize");
            let next_instruction_jump_table_address = unsafe {
                builder.build_in_bounds_gep(
                    this.deploy_table,
                    &[context.i64_type().const_int(0, true), next_instruction_opcode_usize],
                    "next_instruction_jump_table_address",
                )
            };
            let next_instruction: CallableValue<'ctx> =
                builder.build_load(next_instruction_jump_table_address, "next_instruction_address").into_pointer_value().try_into().unwrap();
            let call =
                builder.build_call(next_instruction, &[function.get_nth_param(0).unwrap().into(), next_instruction_opcode_ptr.into()], "call_next_instruction");
            call.set_tail_call(true);
            builder.build_return(Some(&call.try_as_basic_value().unwrap_left()));
        } else {
            let _result=exit.remove_from_function();
        }
        Ok(function)
    }

    pub fn branch(&self, target: &Constant<'ctx>) -> Result<BasicBlock<'ctx>> {
        let block_constant = target.get_basic_block_value()?;
        let context = self.context;
        let builder = context.create_builder();
        Ok(match block_constant {
            TargetBlock::Block { from, block,  } => {
                let current_block = self.builder.get_insert_block().unwrap();
                from.borrow_mut().branchs.push((current_block, block.clone()));
                block.borrow_mut().block
            }
            TargetBlock::Offset(offset_ptr) => {
                let block = context.append_basic_block(self.function, "branch");
                let branch_builder = context.create_builder();
                branch_builder.position_at_end(block);
                let offset=branch_builder.build_load(offset_ptr,"offset").into_int_value();
                let offset_isize = branch_builder.build_int_s_extend(offset, context.custom_width_int_type(usize::BITS), "offset_cast");
                let next_ip_base=branch_builder.build_pointer_cast(offset_ptr, context.i8_type().ptr_type(AddressSpace::Global), "next_ip_base");
                let next_ip = unsafe { branch_builder.build_gep(next_ip_base, &[offset_isize], "branch_ip") };
                branch_builder.build_unconditional_branch(self.exit);
                builder.clear_insertion_position();
                self.ip_phi.add_incoming(&[(&next_ip, block)]);
                block
            }
        })
    }
}

pub(crate) fn get_instruction_metadata<'ctx>(
    instruction_type: &InstructionType,
    generics: &[Constant<'ctx>],
    last_stateul: Option<&StatefulInstruction>,
    is_root: bool,
) -> Result<Cow<'static, InstructionMetadata>>
{
    match instruction_type {
        InstructionType::Bootstrap(bootstrap_instruction) => {
            Ok(Cow::Owned(get_boostrap_instruction_metadata(*bootstrap_instruction, generics, last_stateul, is_root)?))
        }
        InstructionType::Complex(instruction) => Ok(Cow::Owned(instruction.metadata.clone())),
        InstructionType::Compression(instruction) => Ok(Cow::Owned(InstructionMetadata {
            operands: vec![].into(),
            generics: vec![GenericsMetadata {
                kind: GenericsMetadataKind::Constant {
                    value_type: match instruction.instruction_count {
                        0..=0xff => Type::Int(IntKind::U8),
                        0x100..=0xffff => Type::Int(IntKind::U16),
                        o => return Err(TooManySubInstructions(o)),
                    },
                    writable: true,
                },
                name: "sub_opcode".into(),
            }]
            .into(),
        })),
        InstructionType::Stateful(instruction) => Ok(Cow::Owned(instruction.metadata.clone())),
    }
}
pub(crate) fn get_boostrap_instruction_metadata<'ctx>(
    bootstrap: BootstrapInstruction,
    generics: &[Constant<'ctx>],
    last_stateul: Option<&StatefulInstruction>,
    is_root: bool,
) -> Result<InstructionMetadata>
{
    use BootstrapInstruction::*;
    let get_type_generic = |index: usize| {
        Ok(match generics.get(index).ok_or(GenericIndexOutOfRange(index))? {
            Constant::Type(ty, _) => ty,
            _ => return Err(ExceptTypeGeneric()),
        })
    };
    let get_int = |index: usize| match generics.get(index).ok_or(GenericIndexOutOfRange(index))? {
        Constant::Value(value, _ty) if value.is_int_value() && value.into_int_value().is_constant_int() => {
            Ok(value.into_int_value().get_sign_extended_constant().unwrap())
        }
        _ => Err(WroneGenericKind()),
    };
    let get_int_type = |index: usize| {
        let value = get_int(index)?;
        use IntKind::*;
        let int_kind = match value as usize {
            0 => Bool,
            1 => I8,
            2 => U8,
            3 => I16,
            4 => U16,
            5 => I32,
            6 => U32,
            7 => I64,
            8 => U64,
            9 => I128,
            10 => U128,
            11 => Isize,
            12 => Usize,
            o => return Err(IllegalIntKind(o as usize)),
        };
        Ok(Type::Int(int_kind))
    };
    let get_float_type = |index: usize| {
        let value = get_int(index)?;
        use FloatKind::*;
        let float_kind = match value {
            32 => F32,
            64 => F64,
            o => return Err(IllegalFloatKind(o as usize)),
        };
        Ok(Type::Float(float_kind))
    };
    Ok(match bootstrap {
        Nop => InstructionMetadata { operands: Vec::new().into(), generics: Vec::new().into() },
        Move => {
            let ty = get_type_generic(0)?;
            InstructionMetadata {
                operands: CowSlice::Owned(vec![
                    OperandMetadata { input: true, output: false, name: "input".into(), value_type: ty.clone() },
                    OperandMetadata { input: false, output: true, name: "output".into(), value_type: ty.clone() },
                ]),
                generics: vec![GenericsMetadata { kind: GenericsMetadataKind::Type, name: "type".into() }].into(),
            }
        }
        Return => {
            let ty = get_type_generic(0)?;
            InstructionMetadata {
                operands: CowSlice::Owned(vec![OperandMetadata { input: true, output: false, name: "return_value".into(), value_type: ty.clone() }]),
                generics: vec![GenericsMetadata { kind: GenericsMetadataKind::Type, name: "type".into() }].into(),
            }
        }
        Not | Neg => {
            let ty = get_int_type(0)?;
            InstructionMetadata {
                operands: vec![OperandMetadata { input: true, output: true, value_type: ty, name: "i".into() }].into(),
                generics: vec![GenericsMetadata {
                    name: "number_of_bit".into(),
                    kind: GenericsMetadataKind::Constant { value_type: Type::Int(IntKind::I64), writable: false },
                }]
                .into(),
            }
        }
        Add | Sub | Mul | Div | Rem | And | Or | Xor | Shl | Shr | Ushr => {
            let ty = get_int_type(0)?;
            InstructionMetadata {
                operands: vec![
                    OperandMetadata { input: true, output: false, value_type: ty.clone(), name: "lhs".into() },
                    OperandMetadata { input: true, output: true, value_type: ty, name: "rhs".into() },
                ]
                .into(),
                generics: vec![GenericsMetadata {
                    name: "number_of_bit".into(),
                    kind: GenericsMetadataKind::Constant { value_type: Type::Int(IntKind::I64), writable: false },
                }]
                .into(),
            }
        }
        CmpLt | CmpLe | CmpGt | CmpGe | CmpEq | CmpNe | UcmpLe | UcmpLt | UcmpGe | UcmpGt | UcmpEq | UcmpNe => {
            let ty = get_int_type(0)?;
            InstructionMetadata {
                operands: vec![
                    OperandMetadata { input: true, output: false, value_type: ty.clone(), name: "lhs".into() },
                    OperandMetadata { input: true, output: false, value_type: ty, name: "rhs".into() },
                    OperandMetadata { input: false, output: true, value_type: Type::Int(IntKind::Bool), name: "result".into() },
                ]
                .into(),
                generics: vec![GenericsMetadata {
                    name: "number_of_bit".into(),
                    kind: GenericsMetadataKind::Constant { value_type: Type::Int(IntKind::I64), writable: false },
                }]
                .into(),
            }
        }
        FNeg => {
            let ty = get_float_type(0)?;
            InstructionMetadata {
                operands: vec![OperandMetadata { input: true, output: true, value_type: ty, name: "float".into() }].into(),
                generics: vec![GenericsMetadata {
                    name: "number_of_bit".into(),
                    kind: GenericsMetadataKind::Constant { value_type: Type::Int(IntKind::I64), writable: false },
                }]
                .into(),
            }
        }
        FAdd | FSub | FMul | FDiv | FRem => {
            let ty = get_float_type(0)?;
            InstructionMetadata {
                operands: vec![
                    OperandMetadata { input: true, output: false, value_type: ty.clone(), name: "lhs".into() },
                    OperandMetadata { input: true, output: true, value_type: ty, name: "rhs".into() },
                ]
                .into(),
                generics: vec![GenericsMetadata {
                    name: "number_of_bit".into(),
                    kind: GenericsMetadataKind::Constant { value_type: Type::Int(IntKind::I64), writable: false },
                }]
                .into(),
            }
        }
        FcmpLt | FcmpLe | FcmpGe | FcmpGt | FcmpEq | FcmpNe => {
            let ty = get_float_type(0)?;
            InstructionMetadata {
                operands: vec![
                    OperandMetadata { input: true, output: false, value_type: ty.clone(), name: "lhs".into() },
                    OperandMetadata { input: true, output: false, value_type: ty, name: "rhs".into() },
                    OperandMetadata { input: false, output: true, value_type: Type::Int(IntKind::Bool), name: "result".into() },
                ]
                .into(),
                generics: vec![GenericsMetadata {
                    name: "number_of_bit".into(),
                    kind: GenericsMetadataKind::Constant { value_type: Type::Int(IntKind::I64), writable: false },
                }]
                .into(),
            }
        }
        IntToFloat | FloatToFloat | FloatToInt | IntExtend | UIntExtend | IntTruncate => {
            let ty = match bootstrap {
                IntExtend | UIntExtend | IntTruncate | FloatToInt => get_int_type(0)?,
                IntToFloat | FloatToFloat => get_float_type(0)?,
                _ => unreachable!(),
            };
            let ty2 = match bootstrap {
                IntToFloat | IntExtend | UIntExtend | IntTruncate => get_int_type(1)?,
                FloatToInt | FloatToFloat => get_float_type(1)?,
                _ => unreachable!(),
            };
            InstructionMetadata {
                operands: vec![
                    OperandMetadata { input: true, output: false, value_type: ty2, name: "operand".into() },
                    OperandMetadata { input: false, output: true, value_type: ty, name: "result".into() },
                ]
                .into(),
                generics: vec![
                    GenericsMetadata {
                        name: "number_of_bit_of_operand".into(),
                        kind: GenericsMetadataKind::Constant { value_type: Type::Int(IntKind::I64), writable: false },
                    },
                    GenericsMetadata {
                        name: "number_of_bit".into(),
                        kind: GenericsMetadataKind::Constant { value_type: Type::Int(IntKind::I64), writable: false },
                    },
                ]
                .into(),
            }
        }
        CastUnchecked => {
            let ty = get_type_generic(0)?.clone();
            let ty2 = get_type_generic(1)?.clone();
            InstructionMetadata {
                operands: vec![
                    OperandMetadata { input: true, output: false, value_type: ty2, name: "operand".into() },
                    OperandMetadata { input: false, output: true, value_type: ty, name: "result".into() },
                ]
                .into(),
                generics: vec![
                    GenericsMetadata { name: "to".into(), kind: GenericsMetadataKind::Type },
                    GenericsMetadata { name: "from".into(), kind: GenericsMetadataKind::Type },
                ]
                .into(),
            }
        }
        Branch => InstructionMetadata {
            operands: vec![].into(),
            generics: vec![GenericsMetadata { name: "then".into(), kind: GenericsMetadataKind::BasicBlock }].into(),
        },
        BranchIf => InstructionMetadata {
            operands: vec![OperandMetadata { input: true, output: false, value_type: Type::Int(IntKind::Bool), name: "predicate".into() }].into(),
            generics: vec![
                GenericsMetadata { name: "then".into(), kind: GenericsMetadataKind::BasicBlock },
                GenericsMetadata { name: "else".into(), kind: GenericsMetadataKind::BasicBlock },
            ]
            .into(),
        },
        NativeCall => {
            let ty = get_type_generic(0)?;
            let function_type = match ty {
                Type::Function(f) => f,
                o => return Err(ExceptFunctionType(o.clone())),
            };
            let mut operand = Vec::new();
            for (index, arg_type) in function_type.args().iter().enumerate() {
                operand.push(OperandMetadata { input: true, output: false, value_type: arg_type.clone(), name: format!("arg{}", index).into() });
            }
            if let Some(va_arg_type) = &function_type.va_arg {
                operand.push(OperandMetadata {
                    input: true,
                    output: false,
                    name: "va_arg".into(),
                    value_type: Type::Pointer(CowArc::new(Type::Tuple(Tuple::Normal(CowArc::Owned(
                        vec![va_arg_type.clone(), Type::Int(IntKind::Usize)].into(),
                    ))))),
                });
            }
            if let Some(return_type) = function_type.return_type() {
                operand.push(OperandMetadata { input: false, output: true, name: "va_arg".into(), value_type: return_type.clone() });
            }

            InstructionMetadata {
                operands: operand.into(),
                generics: vec![
                    GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type },
                    GenericsMetadata { name: "ptr".into(), kind: GenericsMetadataKind::Constant { value_type: ty.clone(), writable: false } },
                    GenericsMetadata {
                        name: "name".into(),
                        kind: GenericsMetadataKind::Constant {
                            value_type: Type::Tuple(Tuple::Normal(CowArc::Owned(
                                vec![Type::Pointer(CowArc::new(Type::Int(IntKind::U8))), Type::Int(IntKind::Usize)].into(),
                            ))),
                            writable: false,
                        },
                    },
                ]
                .into(),
            }
        }
        Call => {
            let ty = get_type_generic(0)?;
            let function_type = match ty {
                Type::Function(f) => f,
                o => return Err(ExceptFunctionType(o.clone())),
            };
            let mut operand = Vec::new();
            operand.push(OperandMetadata { input: true, output: false, value_type: ty.clone(), name: "funtion".into() });
            for (index, arg_type) in function_type.args().iter().enumerate() {
                operand.push(OperandMetadata { input: true, output: false, value_type: arg_type.clone(), name: format!("arg{}", index).into() });
            }
            if let Some(va_arg_type) = &function_type.va_arg {
                operand.push(OperandMetadata {
                    input: true,
                    output: false,
                    name: "va_arg".into(),
                    value_type: (Type::Tuple(Tuple::Normal(CowArc::Owned(
                        vec![Type::Pointer(CowArc::new(va_arg_type.clone())), Type::Int(IntKind::Usize)].into(),
                    )))),
                });
            }
            if let Some(return_type) = function_type.return_type() {
                operand.push(OperandMetadata { input: false, output: true, name: "va_arg".into(), value_type: return_type.clone() });
            }

            InstructionMetadata { operands: operand.into(), generics: vec![GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type }].into() }
        }
        Invoke => {
            let ty = get_type_generic(0)?;
            let function_type = match ty {
                Type::Function(f) => f,
                o => return Err(ExceptFunctionType(o.clone())),
            };
            let mut operand = Vec::new();
            operand.push(OperandMetadata { input: true, output: false, value_type: ty.clone(), name: "funtion".into() });
            for (index, arg_type) in function_type.args().iter().enumerate() {
                operand.push(OperandMetadata { input: true, output: false, value_type: arg_type.clone(), name: format!("arg{}", index).into() });
            }
            if let Some(va_arg_type) = &function_type.va_arg {
                operand.push(OperandMetadata {
                    input: true,
                    output: false,
                    name: "va_arg".into(),
                    value_type: (Type::Tuple(Tuple::Normal(CowArc::Owned(
                        vec![Type::Pointer(CowArc::new(va_arg_type.clone())), Type::Int(IntKind::Usize)].into(),
                    )))),
                });
            }
            if let Some(return_type) = function_type.return_type() {
                operand.push(OperandMetadata { input: false, output: true, name: "va_arg".into(), value_type: return_type.clone() });
            }

            InstructionMetadata {
                operands: operand.into(),
                generics: vec![
                    GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type },
                    GenericsMetadata { name: "then".into(), kind: GenericsMetadataKind::BasicBlock },
                    GenericsMetadata { name: "catch".into(), kind: GenericsMetadataKind::BasicBlock },
                ]
                .into(),
            }
        }
        MakeSlice => {
            if is_root {
                InstructionMetadata {
                    operands: vec![].into(),
                    generics: vec![
                        GenericsMetadata {
                            name: "len".into(),
                            kind: GenericsMetadataKind::Constant { value_type: Type::Int(IntKind::Usize), writable: false },
                        },
                        GenericsMetadata {
                            name: "size".into(),
                            kind: GenericsMetadataKind::Constant { value_type: Type::Int(IntKind::Usize), writable: true },
                        },
                    ]
                    .into(),
                }
            } else {
                let ty = get_type_generic(0)?;
                let len = get_int(1)?.try_into().unwrap();
                let mut operand = Vec::new();
                for i in 0..len {
                    operand.push(OperandMetadata { input: true, output: false, value_type: ty.clone(), name: format!("arg{}", i).into() });
                }
                operand.push(OperandMetadata {
                    input: false,
                    output: true,
                    name: "variant".into(),
                    value_type: Type::Tuple(Tuple::Normal(CowArc::Owned(vec![Type::Pointer(CowArc::new(ty.clone())), Type::Int(IntKind::Usize)].into()))),
                });
                InstructionMetadata {
                    operands: operand.into(),
                    generics: vec![
                        GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type },
                        GenericsMetadata {
                            name: "len".into(),
                            kind: GenericsMetadataKind::Constant { value_type: Type::Int(IntKind::Usize), writable: false },
                        },
                    ]
                    .into(),
                }
            }
        }
        StackAlloc => {
            let ty = get_type_generic(0)?;
            InstructionMetadata {
                operands: vec![OperandMetadata { input: false, output: true, value_type: Type::Pointer(CowArc::new(ty.clone())), name: "ptr".into() }].into(),
                generics: vec![GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type }].into(),
            }
        }
        StackAllocUnsized => {
            let ty = get_type_generic(0)?;
            InstructionMetadata {
                operands: vec![
                    OperandMetadata { input: true, output: false, value_type: Type::Int(IntKind::Usize), name: "len".into() },
                    OperandMetadata { input: false, output: true, value_type: Type::Pointer(CowArc::new(ty.clone())), name: "ptr".into() },
                ]
                .into(),
                generics: vec![GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type }].into(),
            }
        }
        Read => {
            let ty = get_type_generic(0)?;
            InstructionMetadata {
                operands: vec![
                    OperandMetadata { input: true, output: false, value_type: Type::Pointer(CowArc::new(ty.clone())), name: "ptr".into() },
                    OperandMetadata { input: false, output: true, value_type: ty.clone(), name: "value".into() },
                ]
                .into(),
                generics: vec![GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type }].into(),
            }
        }
        Write => {
            let ty = get_type_generic(0)?;
            InstructionMetadata {
                operands: vec![
                    OperandMetadata { input: true, output: false, value_type: Type::Pointer(CowArc::new(ty.clone())), name: "ptr".into() },
                    OperandMetadata { input: true, output: false, value_type: ty.clone(), name: "value".into() },
                ]
                .into(),
                generics: vec![GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type }].into(),
            }
        }
        Deref => {
            let ty = get_type_generic(0)?;
            match ty {
                Type::Reference(i) => i.try_map(|i| {
                    Ok(InstructionMetadata {
                        operands: vec![
                            OperandMetadata { input: true, output: false, value_type: ty.clone(), name: "ref".into() },
                            OperandMetadata { input: false, output: true, value_type: Type::Pointer(CowArc::new(i.get_type()?.clone())), name: "ptr".into() },
                        ]
                        .into(),
                        generics: vec![GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type }].into(),
                    })
                })?,
                o => return Err(ExceptReferenceType(o.clone())),
            }
        }
        Clone => {
            let ty = get_type_generic(0)?;
            match ty {
                Type::Reference(i) => i.try_map(|_i| {
                    Ok(InstructionMetadata {
                        operands: vec![
                            OperandMetadata { input: true, output: false, value_type: ty.clone(), name: "ref".into() },
                            OperandMetadata { input: false, output: true, value_type: ty.clone(), name: "new_ref".into() },
                        ]
                        .into(),
                        generics: vec![GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type }].into(),
                    })
                })?,
                o => return Err(ExceptReferenceType(o.clone())),
            }
        }
        Drop => {
            let ty = get_type_generic(0)?;
            match ty {
                Type::Reference(i) => i.try_map(|_i| {
                    Ok(InstructionMetadata {
                        operands: vec![OperandMetadata { input: true, output: false, value_type: ty.clone(), name: "ref".into() }].into(),
                        generics: vec![GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type }].into(),
                    })
                })?,
                o => return Err(ExceptReferenceType(o.clone())),
            }
        }
        GetTag => {
            let ty = get_type_generic(0)?;
            InstructionMetadata {
                operands: vec![
                    OperandMetadata { input: true, output: false, value_type: ty.clone(), name: "value".into() },
                    OperandMetadata { input: false, output: true, name: "tag".into(), value_type: Type::Int(IntKind::Usize) },
                ]
                .into(),
                generics: vec![GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type }].into(),
            }
        }
        ReadTag => {
            let ty = get_type_generic(0)?;
            InstructionMetadata {
                operands: vec![
                    OperandMetadata { input: true, output: false, value_type: Type::Pointer(CowArc::new(ty.clone())), name: "ptr".into() },
                    OperandMetadata { input: false, output: true, name: "tag".into(), value_type: Type::Int(IntKind::Usize) },
                ]
                .into(),
                generics: vec![GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type }].into(),
            }
        }
        WriteTag => {
            let ty = get_type_generic(0)?;
            InstructionMetadata {
                operands: vec![
                    OperandMetadata { input: true, output: false, value_type: Type::Pointer(CowArc::new(ty.clone())), name: "ptr".into() },
                    OperandMetadata { input: true, output: false, name: "tag".into(), value_type: Type::Int(IntKind::Usize) },
                ]
                .into(),
                generics: vec![GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type }].into(),
            }
        }
        DecodeVariantUnchecked => {
            let ty = get_type_generic(0)?;
            let index = get_int(1)?.try_into().unwrap();
            let variant_type = match ty {
                Type::Enum(e) => e.variants.get(index).ok_or_else(|| VariantIndexOutOfRange(ty.clone(), index))?.clone(),
                o => return Err(ExceptEnumType(o.clone())),
            };
            InstructionMetadata {
                operands: vec![
                    OperandMetadata { input: true, output: false, value_type: ty.clone(), name: "enum".into() },
                    OperandMetadata { input: false, output: true, name: "variant".into(), value_type: variant_type },
                ]
                .into(),
                generics: vec![
                    GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type },
                    GenericsMetadata { name: "index".into(), kind: GenericsMetadataKind::Constant { value_type: Type::Int(IntKind::Usize), writable: false } },
                ]
                .into(),
            }
        }
        EncodeVariant => {
            let ty = get_type_generic(0)?;
            let index = get_int(1)?.try_into().unwrap();
            let variant_type = match ty {
                Type::Enum(e) => e.variants.get(index).ok_or_else(|| VariantIndexOutOfRange(ty.clone(), index))?.clone(),
                o => return Err(ExceptEnumType(o.clone())),
            };
            InstructionMetadata {
                operands: vec![
                    OperandMetadata { input: true, output: false, name: "variant".into(), value_type: variant_type },
                    OperandMetadata { input: false, output: true, value_type: ty.clone(), name: "enum".into() },
                ]
                .into(),
                generics: vec![
                    GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type },
                    GenericsMetadata { name: "index".into(), kind: GenericsMetadataKind::Constant { value_type: Type::Int(IntKind::Usize), writable: false } },
                ]
                .into(),
            }
        }
        LocateField => {
            let ty = get_type_generic(0)?;
            let index = get_int(1)?.try_into().unwrap();
            let field_type = match ty {
                Type::Tuple(Tuple::Normal(n)) => n.get(index).ok_or_else(|| VariantIndexOutOfRange(ty.clone(), index))?.clone(),
                Type::Tuple(Tuple::Compose(n)) => n.get(index).ok_or_else(|| VariantIndexOutOfRange(ty.clone(), index))?.0.clone(),
                o => return Err(ExceptTupleType(o.clone())),
            };
            InstructionMetadata {
                operands: vec![
                    OperandMetadata { input: true, output: false, value_type: Type::Pointer(CowArc::new(ty.clone())), name: "ptr".into() },
                    OperandMetadata { input: false, output: true, name: "field".into(), value_type: Type::Pointer(CowArc::new(field_type)) },
                ]
                .into(),
                generics: vec![
                    GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type },
                    GenericsMetadata { name: "index".into(), kind: GenericsMetadataKind::Constant { value_type: Type::Int(IntKind::Usize), writable: false } },
                ]
                .into(),
            }
        }
        GetField => {
            let ty = get_type_generic(0)?;
            let index = get_int(1)?.try_into().unwrap();
            let field_type = match ty {
                Type::Tuple(Tuple::Normal(n)) => n.get(index).ok_or_else(|| VariantIndexOutOfRange(ty.clone(), index))?.clone(),
                Type::Tuple(Tuple::Compose(n)) => n.get(index).ok_or_else(|| VariantIndexOutOfRange(ty.clone(), index))?.0.clone(),
                o => return Err(ExceptTupleType(o.clone())),
            };
            InstructionMetadata {
                operands: vec![
                    OperandMetadata { input: true, output: false, value_type: ty.clone(), name: "ptr".into() },
                    OperandMetadata { input: false, output: true, name: "field".into(), value_type: field_type },
                ]
                .into(),
                generics: vec![
                    GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type },
                    GenericsMetadata { name: "index".into(), kind: GenericsMetadataKind::Constant { value_type: Type::Int(IntKind::Usize), writable: false } },
                ]
                .into(),
            }
        }
        SetField => {
            let ty = get_type_generic(0)?;
            let index = get_int(1)?.try_into().unwrap();
            let field_type = match ty {
                Type::Tuple(Tuple::Normal(n)) => n.get(index).ok_or_else(|| VariantIndexOutOfRange(ty.clone(), index))?.clone(),
                Type::Tuple(Tuple::Compose(n)) => n.get(index).ok_or_else(|| VariantIndexOutOfRange(ty.clone(), index))?.0.clone(),
                o => return Err(ExceptTupleType(o.clone())),
            };
            InstructionMetadata {
                operands: vec![
                    OperandMetadata { input: true, output: true, value_type: ty.clone(), name: "ptr".into() },
                    OperandMetadata { input: true, output: false, name: "field".into(), value_type: field_type },
                ]
                .into(),
                generics: vec![
                    GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type },
                    GenericsMetadata { name: "index".into(), kind: GenericsMetadataKind::Constant { value_type: Type::Int(IntKind::Usize), writable: false } },
                ]
                .into(),
            }
        }
        UninitedStruct => {
            let ty = get_type_generic(0)?;
            InstructionMetadata {
                operands: vec![OperandMetadata { input: false, output: true, value_type: ty.clone(), name: "value".into() }].into(),
                generics: vec![GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type }].into(),
            }
        }
        LocateUnion => {
            let ty = get_type_generic(0)?;
            let index = get_int(1)?.try_into().unwrap();
            let field_type = match ty {
                Type::Tuple(Tuple::Normal(n)) => n.get(index).ok_or_else(|| VariantIndexOutOfRange(ty.clone(), index))?.clone(),
                Type::Tuple(Tuple::Compose(n)) => n.get(index).ok_or_else(|| VariantIndexOutOfRange(ty.clone(), index))?.0.clone(),
                o => return Err(ExceptTupleType(o.clone())),
            };
            InstructionMetadata {
                operands: vec![
                    OperandMetadata { input: true, output: false, value_type: Type::Pointer(CowArc::new(ty.clone())), name: "ptr".into() },
                    OperandMetadata { input: false, output: true, name: "field".into(), value_type: Type::Pointer(CowArc::new(field_type)) },
                ]
                .into(),
                generics: vec![
                    GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type },
                    GenericsMetadata { name: "index".into(), kind: GenericsMetadataKind::Constant { value_type: Type::Int(IntKind::Usize), writable: false } },
                ]
                .into(),
            }
        }
        LocateElement => {
            let ty = get_type_generic(0)?;
            match ty {
                Type::Array(element, _size) => InstructionMetadata {
                    operands: vec![
                        OperandMetadata { input: true, output: false, value_type: Type::Pointer(CowArc::new(ty.clone())), name: "ptr".into() },
                        OperandMetadata { input: true, output: false, name: "index".into(), value_type: Type::Int(IntKind::Usize) },
                        OperandMetadata { input: false, output: true, name: "element".into(), value_type: Type::Pointer(element.clone()) },
                    ]
                    .into(),
                    generics: vec![GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type }].into(),
                },
                Type::Pointer(_ptr) => InstructionMetadata {
                    operands: vec![
                        OperandMetadata { input: true, output: false, value_type: ty.clone(), name: "ptr".into() },
                        OperandMetadata { input: true, output: false, name: "index".into(), value_type: Type::Int(IntKind::Usize) },
                        OperandMetadata { input: false, output: true, name: "element".into(), value_type: ty.clone() },
                    ]
                    .into(),
                    generics: vec![GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type }].into(),
                },
                o => return Err(ExceptArrayType(o.clone())),
            }
        }
        GetLength => {
            let ty = get_type_generic(0)?;
            InstructionMetadata {
                operands: vec![
                    OperandMetadata { input: true, output: false, value_type: Type::Pointer(CowArc::new(ty.clone())), name: "ptr".into() },
                    OperandMetadata { input: false, output: true, name: "len".into(), value_type: Type::Int(IntKind::Usize) },
                ]
                .into(),
                generics: vec![GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type }].into(),
            }
        }
        // fn<type ty>(array:Pointer<Array>,len:Usize)
        SetLength => {
            let ty = get_type_generic(0)?;
            InstructionMetadata {
                operands: vec![
                    OperandMetadata { input: true, output: false, value_type: Type::Pointer(CowArc::new(ty.clone())), name: "ptr".into() },
                    OperandMetadata { input: true, output: false, name: "len".into(), value_type: Type::Int(IntKind::Usize) },
                ]
                .into(),
                generics: vec![GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type }].into(),
            }
        }
        LocateMetadata => {
            let ty = get_type_generic(0)?;
            let index = get_int(1)?.try_into().unwrap();
            let (_, meta) = match ty {
                Type::MetaData(m) => m.get(index).ok_or_else(|| MetadataIndexOutOfRange(ty.clone(), index))?,
                o => return Err(ExceptMetadataType(o.clone())),
            };
            let meta_type = meta.get_type()?.clone();
            InstructionMetadata {
                operands: vec![
                    OperandMetadata { input: true, output: false, value_type: Type::Pointer(CowArc::new(ty.clone())), name: "ptr".into() },
                    OperandMetadata { input: false, output: true, name: "meta".into(), value_type: meta_type },
                ]
                .into(),
                generics: vec![
                    GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type },
                    GenericsMetadata { name: "index".into(), kind: GenericsMetadataKind::Constant { value_type: Type::Int(IntKind::Usize), writable: false } },
                ]
                .into(),
            }
        }
        CompareAndSwap => {
            let ty = get_type_generic(0)?;
            InstructionMetadata {
                operands: vec![
                    OperandMetadata { input: true, output: false, value_type: Type::Pointer(CowArc::new(ty.clone())), name: "ptr".into() },
                    OperandMetadata { input: true, output: false, name: "except".into(), value_type: ty.clone() },
                    OperandMetadata { input: true, output: false, name: "value".into(), value_type: ty.clone() },
                    OperandMetadata { input: false, output: true, name: "sucess".into(), value_type: Type::Int(IntKind::Bool) },
                ]
                .into(),
                generics: vec![GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type }].into(),
            }
        }
        FenceReleased | FenceAcquire | FenceAcqrel | FenceSeqcst => InstructionMetadata { operands: vec![].into(), generics: vec![].into() },
        Free => {
            let ty = get_type_generic(0)?;
            InstructionMetadata {
                operands: vec![OperandMetadata { input: true, output: false, value_type: Type::Pointer(CowArc::new(ty.clone())), name: "ptr".into() }].into(),
                generics: vec![GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type }].into(),
            }
        }
        AllocSized => {
            let ty = get_type_generic(0)?;
            InstructionMetadata {
                operands: vec![OperandMetadata { input: false, output: true, value_type: ty.clone(), name: "ptr".into() }].into(),
                generics: vec![GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type }].into(),
            }
        }
        AllocUnsized => {
            let ty = get_type_generic(0)?;
            InstructionMetadata {
                operands: vec![
                    OperandMetadata { input: true, output: false, value_type: Type::Int(IntKind::Usize), name: "len".into() },
                    OperandMetadata { input: false, output: true, value_type: ty.clone(), name: "ptr".into() },
                ]
                .into(),
                generics: vec![GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type }].into(),
            }
        }
        NonGCAllocSized => {
            let ty = get_type_generic(0)?;
            match ty {
                Type::Reference(i) => i.try_map(|i| {
                    let inner_type = i.get_type()?;
                    Ok(InstructionMetadata {
                        operands: vec![OperandMetadata {
                            input: false,
                            output: true,
                            value_type: Type::Pointer(CowArc::new(inner_type.clone())),
                            name: "ptr".into(),
                        }]
                        .into(),
                        generics: vec![GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type }].into(),
                    })
                })?,
                o => return Err(ExceptReferenceType(o.clone())),
            }
        }
        NonGCAllocUnsized => {
            let ty = get_type_generic(0)?;
            match ty {
                Type::Reference(i) => i.try_map(|i| {
                    let inner_type = i.get_type()?;
                    Ok(InstructionMetadata {
                        operands: vec![
                            OperandMetadata { input: true, output: false, name: "len".into(), value_type: Type::Int(IntKind::Usize) },
                            OperandMetadata { input: false, output: true, value_type: Type::Pointer(CowArc::new(inner_type.clone())), name: "ptr".into() },
                        ]
                        .into(),
                        generics: vec![GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type }].into(),
                    })
                })?,
                o => return Err(ExceptReferenceType(o.clone())),
            }
        }
        NonGCFree => {
            let ty = get_type_generic(0)?;
            match ty {
                Type::Reference(i) => i.try_map(|i| {
                    let inner_type = i.get_type()?;
                    Ok(InstructionMetadata {
                        operands: vec![OperandMetadata {
                            input: true,
                            output: false,
                            value_type: Type::Pointer(CowArc::new(inner_type.clone())),
                            name: "ptr".into(),
                        }]
                        .into(),
                        generics: vec![GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type }].into(),
                    })
                })?,
                o => return Err(ExceptReferenceType(o.clone())),
            }
        }
        MemoryCopy => {
            let ty = get_type_generic(0)?;
            InstructionMetadata {
                operands: vec![
                    OperandMetadata { input: true, output: false, value_type: Type::Pointer(CowArc::new(ty.clone())), name: "dst".into() },
                    OperandMetadata { input: true, output: false, value_type: Type::Pointer(CowArc::new(ty.clone())), name: "src".into() },
                    OperandMetadata { input: true, output: false, value_type: Type::Int(IntKind::Usize), name: "len".into() },
                ]
                .into(),
                generics: vec![GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type }].into(),
            }
        }
        SetState => {
            InstructionMetadata { operands: vec![].into(), generics: vec![GenericsMetadata { name: "state".into(), kind: GenericsMetadataKind::State }].into() }
        }
        CallState => {
            let mut meta = last_stateul.ok_or_else(NotInStatefulInstruction)?.metadata.clone();
            meta.generics.get_mut().push(GenericsMetadata { name: "state".into(), kind: GenericsMetadataKind::State });
            meta
        }
        GetPointer => {
            let ty = get_type_generic(0)?;
            InstructionMetadata {
                operands: vec![
                    OperandMetadata { input: true, output: false, value_type: ty.clone(), name: "reg".into() },
                    OperandMetadata { input: false, output: true, value_type: Type::Pointer(CowArc::new(ty.clone())), name: "ptr".into() },
                ]
                .into(),
                generics: vec![GenericsMetadata { name: "ty".into(), kind: GenericsMetadataKind::Type }].into(),
            }
        }
    })
}
//  fn(/* register */ *mut usize, /* ip */ *const u8) -> u16;
pub(crate) fn get_instruction_function_type<'ctx>(context: &'ctx Context) -> FunctionType<'ctx> {
    context.i64_type().fn_type(
        &[
            // registers
            context.i64_type().ptr_type(AddressSpace::Local).into(),
            // instruction
            context.i8_type().ptr_type(AddressSpace::Global).into(),
        ],
        false,
    )
}
