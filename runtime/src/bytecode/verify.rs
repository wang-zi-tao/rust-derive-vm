use super::{
    frame::{self, push, store, FrameDataType, FrameDataType::*, FrameState},
    ByteCode, Environment,
};
use crate::bytecode::{code, constant_pool::*};
use classfile::{
    attributes::{Attribute, StackMapTable},
    constants::{Constant, ConstantMethodRefImpl},
    symbol::{MethodSymbol, MethodTypeSymbol, TypeSymbol},
};
use failure::format_err;
use frame::{load, load_match, pop, pop_match, pop_match_slice, replice_cell};
use jvm_core::{
    BootstrapClassSetTrait, ClassLoaderRef, ClassLoaderTrait, HasModifier, JavaClassRef,
    MemberTrait, Modifier, PrimitiveType, Type,
};
use memory::associate::AssociateStubPoolBuilderTrait;
use std::{
    collections::{hash_map::Entry, HashMap},
    convert::TryInto,
    intrinsics::bswap,
    mem::size_of,
};
use util::Result;
type BytecodeVerifier = fn(
    Environment,
    &mut dyn FnMut(&mut Environment, u32) -> Result<()>,
    &mut dyn FnMut(&mut Environment, Option<&JavaClassRef>) -> Result<()>,
    &mut dyn FnMut(&mut Environment) -> Result<()>,
) -> Result<()>;
macro_rules! declare(
    ($name:ident,$byte_code_verifier:expr)=>{
        const $name:BytecodeVerifier=$byte_code_verifier;
    }
);
declare!(NOP, |_, _, _, _| Ok(()));

macro_rules! declare_push(
    ($name:ident,$push_type:expr)=>{
        declare!($name,|e, _, _, _| push(e.frame,e.code.max_stack, $push_type));
    }
);

declare_push!(ACONST_NULL, Null);
declare_push!(ICONST_M1, Int);
declare_push!(ICONST_0, Int);
declare_push!(ICONST_1, Int);
declare_push!(ICONST_2, Int);
declare_push!(ICONST_3, Int);
declare_push!(ICONST_4, Int);
declare_push!(ICONST_5, Int);
declare_push!(LCONST_0, Long);
declare_push!(LCONST_1, Long);
declare_push!(FCONST_0, Float);
declare_push!(FCONST_1, Float);
declare_push!(FCONST_2, Float);
declare_push!(DCONST_0, Double);
declare_push!(DCONST_1, Double);

fn next<T: Copy>(byte_code: &ByteCode, offset: &mut u32) -> Result<T> {
    byte_code
        .try_get(*offset as usize)
        .map(|v| bswap(v))
        .ok_or_else(|| format_err!("Invalid byte code format"))
        .map(|v| {
            *offset += size_of::<T>() as u32;
            v
        })
}
fn get_constant<'l>(constants: &'l Vec<Constant>, index: u16) -> Result<&'l Constant> {
    constants
        .get(index as usize)
        .ok_or_else(|| format_err!("Invalid constant index:{}", index))
}
fn next_constant<'l, T: Copy>(
    byte_code: &'l ByteCode,
    offset: &'l mut u32,
    constants: &'l Vec<Constant>,
) -> Result<&'l Constant> {
    next::<u16>(byte_code, offset).and_then(|i: u16| get_constant(constants, i))
}
declare!(BIPUSH, |e, _, _, _| next::<u8>(e.byte_code, e.offset)
    .and(push(e.frame, e.code.max_stack, Int)));
declare!(SIPUSH, |e, _, _, _| next::<u16>(e.byte_code, e.offset)
    .and(push(e.frame, e.code.max_stack, Int)));
fn bootstrap_class_set<'l>(class_loader: &'l ClassLoaderRef) -> &'l dyn BootstrapClassSetTrait {
    class_loader.get_bootstrap_class_set()
}
fn frame_class_type(e: &Environment) -> FrameDataType {
    Reference(
        e.class_loader
            .get_bootstrap_class_set()
            .java_lang_class()
            .clone(),
    )
}
fn frame_string_type(e: &Environment) -> FrameDataType {
    Reference(
        e.class_loader
            .get_bootstrap_class_set()
            .java_lang_string()
            .clone(),
    )
}
fn frame_method_type_type(e: &Environment) -> FrameDataType {
    Reference(
        e.class_loader
            .get_bootstrap_class_set()
            .java_lang_invoke_method_type()
            .clone(),
    )
}
fn frame_method_handle_type(e: &Environment) -> FrameDataType {
    Reference(
        e.class_loader
            .get_bootstrap_class_set()
            .java_lang_invoke_method_handle(),
    )
}

const LDC: BytecodeVerifier = |e, _, _, _| {
    let constant = next_constant::<u8>(e.byte_code, e.offset, e.constants)?;
    match constant {
        Constant::ConstantInteger(_) => push(e.frame, e.code.max_stack, Int),
        Constant::ConstantFloat(_) => push(e.frame, e.code.max_stack, Float),
        Constant::ConstantClass(_) => push(e.frame, e.code.max_stack, frame_class_type(&e)),
        Constant::ConstantString(_) => push(e.frame, e.code.max_stack, frame_string_type(&e)),
        Constant::ConstantMethodType(_) => {
            push(e.frame, e.code.max_stack, frame_method_type_type(&e))
        }
        Constant::ConstantMethodHandle(_) => {
            push(e.frame, e.code.max_stack, frame_method_handle_type(&e))
        }
        _ => Err(format_err!(
            "{:?} can not use in instruction LDC.",
            constant
        )),
    }
};

const LDC_W: BytecodeVerifier = |e, _, _, _| {
    let constant = next_constant::<u16>(e.byte_code, e.offset, e.constants)?;
    match constant {
        Constant::ConstantInteger(_) => push(e.frame, e.code.max_stack, Int),
        Constant::ConstantFloat(_) => push(e.frame, e.code.max_stack, Float),
        Constant::ConstantClass(_) => push(e.frame, e.code.max_stack, frame_class_type(&e)),
        Constant::ConstantString(_) => push(e.frame, e.code.max_stack, frame_string_type(&e)),
        Constant::ConstantMethodType(_) => {
            push(e.frame, e.code.max_stack, frame_method_type_type(&e))
        }
        Constant::ConstantMethodHandle(_) => {
            push(e.frame, e.code.max_stack, frame_method_handle_type(&e))
        }
        _ => Err(format_err!(
            "{:?} can not use in instruction LDC.",
            constant
        )),
    }
};

const LDC2_W: BytecodeVerifier = |e, _, _, _| {
    let constant = next_constant::<u16>(e.byte_code, e.offset, e.constants)?;
    match constant {
        Constant::ConstantLong(_) => push(e.frame, e.code.max_stack, Long),
        Constant::ConstantDouble(_) => push(e.frame, e.code.max_stack, Double),
        _ => Err(format_err!(
            "{:?} can not use in instruction LDC.",
            constant
        )),
    }
};

macro_rules! declare_load_primary(
    ($name:ident,$data_type:expr,$index:expr)=>{
        declare!($name,|e,_,_,_|{
            let index:Result<_>=($index)(e.byte_code,e.offset);
            load_match(e.frame,index? as u16, &$data_type)?;
            push(e.frame,e.code.max_stack, $data_type.clone())
        });
    }
);
declare_load_primary!(ILOAD, Int, |b, o| next::<u8>(b, o));
declare_load_primary!(LLOAD, Long, |b, o| next::<u8>(b, o));
declare_load_primary!(FLOAD, Float, |b, o| next::<u8>(b, o));
declare_load_primary!(DLOAD, Double, |b, o| next::<u8>(b, o));

macro_rules! declare_load_reference(
    ($name:ident,$index:expr)=>{
        declare!($name,|e,_,_,_|{
            let index:Result<_>=($index)(e.byte_code,e.offset);
                let c:JavaClassRef=load(e.frame,index? as u16).and_then(|t| t.try_as_reference().ok_or_else(||format_err!("not a reference")))?.clone();
                push(e.frame,e.code.max_stack, Reference(c))
            }
        );
    }
);
declare_load_reference!(ALOAD, |b, o| next::<u8>(b, o));

declare_load_primary!(ILOAD_0, Int, |_, _| Ok(0));
declare_load_primary!(ILOAD_1, Int, |_, _| Ok(1));
declare_load_primary!(ILOAD_2, Int, |_, _| Ok(2));
declare_load_primary!(ILOAD_3, Int, |_, _| Ok(3));
declare_load_primary!(LLOAD_0, Long, |_, _| Ok(0));
declare_load_primary!(LLOAD_1, Long, |_, _| Ok(1));
declare_load_primary!(LLOAD_2, Long, |_, _| Ok(2));
declare_load_primary!(LLOAD_3, Long, |_, _| Ok(3));
declare_load_primary!(FLOAD_0, Float, |_, _| Ok(0));
declare_load_primary!(FLOAD_1, Float, |_, _| Ok(1));
declare_load_primary!(FLOAD_2, Float, |_, _| Ok(2));
declare_load_primary!(FLOAD_3, Float, |_, _| Ok(3));
declare_load_primary!(DLOAD_0, Double, |_, _| Ok(0));
declare_load_primary!(DLOAD_1, Double, |_, _| Ok(1));
declare_load_primary!(DLOAD_2, Double, |_, _| Ok(2));
declare_load_primary!(DLOAD_3, Double, |_, _| Ok(3));

declare_load_reference!(ALOAD_0, |_, _| Ok(0));
declare_load_reference!(ALOAD_1, |_, _| Ok(1));
declare_load_reference!(ALOAD_2, |_, _| Ok(2));
declare_load_reference!(ALOAD_3, |_, _| Ok(3));

macro_rules! declare_load_primary_array(
    ($name:ident, $type_field:tt,$data_type:expr)=>{
        declare!($name, |e, _, _, _| pop_match_slice(
            e.frame,
            &[
            Reference(bootstrap_class_set(e.class_loader).$type_field().get_array_class()?.clone()),
            Int
        ])
        .and_then(|_| push(e.frame,e.code.max_stack, $data_type)));
    }
);
declare_load_primary_array!(IALOAD, int, Int);
declare_load_primary_array!(LALOAD, long, Long);
declare_load_primary_array!(FALOAD, float, Float);
declare_load_primary_array!(DALOAD, double, Double);

declare!(AALOAD, |e, _, _, _| {
    pop_match(e.frame, &Int)?;
    let array_type = pop(e.frame)?;
    let class = array_type
        .try_as_reference()
        .ok_or_else(|| format_err!("not a reference"))?;
    if let Some(element_class) = class.get_component_type()? {
        push(e.frame, e.code.max_stack, Reference(element_class.clone()))
    } else {
        Err(format_err!("{:?} is not a array class", class))
    }
});

declare!(BALOAD, |e, _, _, _| {
    pop_match(e.frame, &Int)?;
    let array_type = pop(e.frame)?;
    let class = array_type
        .try_as_reference()
        .ok_or_else(|| format_err!("not a reference"))?;
    if !bootstrap_class_set(e.class_loader)
        .byte()
        .get_array_class()?
        .equal(&**class)?
        && !bootstrap_class_set(e.class_loader)
            .boolean()
            .get_array_class()?
            .equal(&**class)?
    {
        Err(format_err!(
            "instruction BLOAD can not load element from class {:?}",
            class
        ))?;
    }
    Ok(())
});

declare_load_primary_array!(SALOAD, short, Int);
declare_load_primary_array!(CALOAD, char, Int);

macro_rules! declare_store_primary(
    ($name:ident,$data_type:expr,$index:expr)=>{
        declare!($name,|e,_,_,_|{
            pop_match(e.frame, &$data_type)?;
            let index:Result<_>=($index)(e.byte_code,e.offset);
            store(e.frame,index? as u16, $data_type.clone())
        });
    }
);
declare_store_primary!(ISTORE, Int, |b, o| next::<u8>(b, o));
declare_store_primary!(LSTORE, Long, |b, o| next::<u8>(b, o));
declare_store_primary!(FSTORE, Float, |b, o| next::<u8>(b, o));
declare_store_primary!(DSTORE, Double, |b, o| next::<u8>(b, o));

macro_rules! declare_store_reference(
    ($name:ident,$index:expr)=>{
        declare!($name,|e,_,_,_|
            {
                let top=pop(e.frame)?;
                if!top.is_reference(){
                    Err(format_err!("except reference,found {:?}",top))
                }else{
                    let index:Result<_>=
                    (($index)(e.byte_code,e.offset));
                    store(e.frame,index? as u16,top)
                }
        }
        );
    }
);
declare_store_reference!(ASTORE, |b, o| next::<u8>(b, o));

declare_store_primary!(ISTORE_0, Int, |_, _| Ok(0));
declare_store_primary!(ISTORE_1, Int, |_, _| Ok(1));
declare_store_primary!(ISTORE_2, Int, |_, _| Ok(2));
declare_store_primary!(ISTORE_3, Int, |_, _| Ok(3));
declare_store_primary!(LSTORE_0, Long, |_, _| Ok(0));
declare_store_primary!(LSTORE_1, Long, |_, _| Ok(1));
declare_store_primary!(LSTORE_2, Long, |_, _| Ok(2));
declare_store_primary!(LSTORE_3, Long, |_, _| Ok(3));
declare_store_primary!(FSTORE_0, Float, |_, _| Ok(0));
declare_store_primary!(FSTORE_1, Float, |_, _| Ok(1));
declare_store_primary!(FSTORE_2, Float, |_, _| Ok(2));
declare_store_primary!(FSTORE_3, Float, |_, _| Ok(3));
declare_store_primary!(DSTORE_0, Double, |_, _| Ok(0));
declare_store_primary!(DSTORE_1, Double, |_, _| Ok(1));
declare_store_primary!(DSTORE_2, Double, |_, _| Ok(2));
declare_store_primary!(DSTORE_3, Double, |_, _| Ok(3));

declare_store_reference!(ASTORE_0, |_, _| Ok(0));
declare_store_reference!(ASTORE_1, |_, _| Ok(1));
declare_store_reference!(ASTORE_2, |_, _| Ok(2));
declare_store_reference!(ASTORE_3, |_, _| Ok(3));

macro_rules! declare_store_primary_array(
    ($name:ident, $type_field:tt,$data_type:expr)=>{
        declare!($name, |e, _, _, _| pop_match_slice(
            e.frame,&[
            Reference(bootstrap_class_set(e.class_loader).$type_field().get_array_class()?.clone()),
            Int,$data_type
        ]));
    }
);
declare_store_primary_array!(IASTORE, int, Int);
declare_store_primary_array!(LASTORE, long, Long);
declare_store_primary_array!(FASTORE, float, Float);
declare_store_primary_array!(DASTORE, double, Double);

declare!(AASTORE, |e, _, _, _| {
    let element_type = pop(e.frame)?;
    pop_match(e.frame, &Int)?;
    let array_type = pop(e.frame)?;
    let array_class = array_type
        .try_as_reference()
        .ok_or_else(|| format_err!("not a reference"))?;
    let element_class = element_type
        .try_as_reference()
        .ok_or_else(|| format_err!("not a reference"))?;
    if array_class
        .get_component_type()?
        .ok_or_else(|| {
            format_err!(
                "class {:?} is not assignable to class {:?}",
                element_class,
                array_class.get_component_type()
            )
        })
        .and_then(|i| element_class.is_assignable(&i))?
    {
        Ok(())
    } else {
        Err(format_err!(
            "class {:?} is not assignable to class {:?}",
            element_class,
            array_class.get_component_type()
        ))
    }
});

declare!(BASTORE, |e, _, _, _| {
    pop_match(e.frame, &Int)?;
    let array_type = pop(e.frame)?;
    let class = array_type
        .try_as_reference()
        .ok_or_else(|| format_err!("not a reference"))?;
    if !bootstrap_class_set(e.class_loader)
        .byte()
        .get_array_class()?
        .equal(&**class)?
        && !bootstrap_class_set(e.class_loader)
            .boolean()
            .get_array_class()?
            .equal(&**class)?
    {
        Err(format_err!(
            "instruction BLOAD can not load element from class {:?}",
            class
        ))?;
    }
    Ok(())
});

declare_store_primary_array!(SASTORE, short, Int);
declare_store_primary_array!(CASTORE, char, Int);
#[derive(Clone)]
enum WideDataType {
    TwoWord(FrameDataType),
    OneWord(FrameDataType, FrameDataType),
}
impl WideDataType {
    fn pop_one_word(frame: &mut FrameState) -> Result<FrameDataType> {
        let top1 = pop(frame)?;
        if top1.is_two_word() {
            Err(format_err!("except one word type"))
        } else {
            Ok(top1)
        }
    }

    fn pop_two_word(frame: &mut FrameState) -> Result<Self> {
        let top1 = pop(frame)?;
        if top1.is_two_word() {
            Ok(Self::TwoWord(top1))
        } else {
            Ok(Self::OneWord(Self::pop_one_word(frame)?, top1))
        }
    }

    fn push_two_word(self, frame: &mut FrameState, max_stack: u16) -> Result<()> {
        match self {
            WideDataType::TwoWord(t) => push(frame, max_stack, t),
            WideDataType::OneWord(t1, t2) => {
                push(frame, max_stack, t1).and_then(|_| push(frame, max_stack, t2))
            }
        }
    }
}
const POP: BytecodeVerifier = |e, _, _, _| pop(e.frame).map(|_| ());
const POP2: BytecodeVerifier = |e, _, _, _| WideDataType::pop_two_word(e.frame).map(|_| ());
const DUP: BytecodeVerifier = |e, _, _, _| {
    let top = WideDataType::pop_one_word(e.frame)?;
    push(e.frame, e.code.max_stack, top.clone())?;
    push(e.frame, e.code.max_stack, top)?;
    Ok(())
};
const DUP_X1: BytecodeVerifier = |e, _, _, _| {
    let top1 = WideDataType::pop_one_word(e.frame)?;
    let top2 = WideDataType::pop_one_word(e.frame)?;
    push(e.frame, e.code.max_stack, top1.clone())?;
    push(e.frame, e.code.max_stack, top2)?;
    push(e.frame, e.code.max_stack, top1)?;
    Ok(())
};
const DUP_X2: BytecodeVerifier = |e, _, _, _| {
    let top1 = WideDataType::pop_two_word(e.frame)?;
    let top2 = WideDataType::pop_one_word(e.frame)?;
    top1.clone().push_two_word(e.frame, e.code.max_stack)?;
    push(e.frame, e.code.max_stack, top2)?;
    top1.push_two_word(e.frame, e.code.max_stack)?;
    Ok(())
};
const DUP2: BytecodeVerifier = |e, _, _, _| {
    let top1 = WideDataType::pop_two_word(e.frame)?;
    top1.clone().push_two_word(e.frame, e.code.max_stack)?;
    top1.push_two_word(e.frame, e.code.max_stack)?;
    Ok(())
};
const DUP2_X1: BytecodeVerifier = |e, _, _, _| {
    let top1 = WideDataType::pop_two_word(e.frame)?;
    let top2 = WideDataType::pop_one_word(e.frame)?;
    top1.clone().push_two_word(e.frame, e.code.max_stack)?;
    push(e.frame, e.code.max_stack, top2)?;
    top1.push_two_word(e.frame, e.code.max_stack)?;
    Ok(())
};
const DUP2_X2: BytecodeVerifier = |e, _, _, _| {
    let top1 = WideDataType::pop_two_word(e.frame)?;
    let top2 = WideDataType::pop_two_word(e.frame)?;
    top1.clone().push_two_word(e.frame, e.code.max_stack)?;
    top2.push_two_word(e.frame, e.code.max_stack)?;
    top1.push_two_word(e.frame, e.code.max_stack)?;
    Ok(())
};
const SWAP: BytecodeVerifier = |e, _, _, _| {
    let top1 = WideDataType::pop_one_word(e.frame)?;
    let top2 = WideDataType::pop_one_word(e.frame)?;
    push(e.frame, e.code.max_stack, top2)?;
    push(e.frame, e.code.max_stack, top1)?;
    Ok(())
};
macro_rules! declare_pop_slice_and_push(
    ($name:ident, $pop_slice:expr,$push:expr) =>{
        declare!($name,|e,_,_,_|pop_match_slice(e.frame,$pop_slice).and_then(|_|push(e.frame,e.code.max_stack,$push)));
    }
);
macro_rules! declare_pop_slice_and_push_div(
    ($name:ident, $pop_slice:expr,$push:expr) =>{
        declare!($name,|e,_,_,_|
            pop_match_slice(e.frame,$pop_slice)
                .and_then(|_|push(e.frame,e.code.max_stack,$push))
        );
    }
);
declare_pop_slice_and_push!(IADD, &[Int, Int], Int);
declare_pop_slice_and_push!(LADD, &[Long, Long], Long);
declare_pop_slice_and_push!(FADD, &[Float, Float], Float);
declare_pop_slice_and_push!(DADD, &[Double, Double], Double);
declare_pop_slice_and_push!(ISUB, &[Int, Int], Int);
declare_pop_slice_and_push!(LSUB, &[Long, Long], Long);
declare_pop_slice_and_push!(FSUB, &[Float, Float], Float);
declare_pop_slice_and_push!(DSUB, &[Double, Double], Double);
declare_pop_slice_and_push!(IMUL, &[Int, Int], Int);
declare_pop_slice_and_push!(LMUL, &[Long, Long], Long);
declare_pop_slice_and_push!(FMUL, &[Float, Float], Float);
declare_pop_slice_and_push!(DMUL, &[Double, Double], Double);
declare_pop_slice_and_push!(IDIV, &[Int, Int], Int);
declare_pop_slice_and_push!(LDIV, &[Long, Long], Long);
declare_pop_slice_and_push!(FDIV, &[Float, Float], Float);
declare_pop_slice_and_push!(DDIV, &[Double, Double], Double);

declare_pop_slice_and_push_div!(IREM, &[Int, Int], Int);
declare_pop_slice_and_push_div!(LREM, &[Long, Long], Long);

declare_pop_slice_and_push!(FREM, &[Float, Float], Float);
declare_pop_slice_and_push!(DREM, &[Double, Double], Double);

declare_pop_slice_and_push_div!(INEG, &[Int, Int], Int);
declare_pop_slice_and_push_div!(LNEG, &[Long, Long], Long);

declare_pop_slice_and_push!(FNEG, &[Float, Float], Float);
declare_pop_slice_and_push!(DNEG, &[Double, Double], Double);
declare_pop_slice_and_push!(ISHL, &[Int, Int], Int);
declare_pop_slice_and_push!(LSHL, &[Long, Long], Long);
declare_pop_slice_and_push!(ISHR, &[Int, Int], Int);
declare_pop_slice_and_push!(LSHR, &[Long, Long], Long);
declare_pop_slice_and_push!(IUSHR, &[Int, Int], Int);
declare_pop_slice_and_push!(LUSHR, &[Long, Long], Long);
declare_pop_slice_and_push!(IAND, &[Int, Int], Int);
declare_pop_slice_and_push!(LAND, &[Long, Long], Long);
declare_pop_slice_and_push!(IOR, &[Int, Int], Int);
declare_pop_slice_and_push!(LOR, &[Long, Long], Long);
declare_pop_slice_and_push!(IXOR, &[Int, Int], Int);
declare_pop_slice_and_push!(LXOR, &[Long, Long], Long);

declare!(IINC, |e, _, _, _| {
    let index = next::<u8>(e.byte_code, e.offset)?;
    let _value = next::<u8>(e.byte_code, e.offset)?;
    load_match(e.frame, index as u16, &Int)
});

declare_pop_slice_and_push!(I2L, &[Int], Long);
declare_pop_slice_and_push!(I2F, &[Int], Float);
declare_pop_slice_and_push!(I2D, &[Int], Double);

declare_pop_slice_and_push!(L2I, &[Long], Int);
declare_pop_slice_and_push!(L2F, &[Long], Float);
declare_pop_slice_and_push!(L2D, &[Long], Double);

declare_pop_slice_and_push!(F2I, &[Float], Int);
declare_pop_slice_and_push!(F2L, &[Float], Long);
declare_pop_slice_and_push!(F2D, &[Float], Double);

declare_pop_slice_and_push!(D2I, &[Double], Int);
declare_pop_slice_and_push!(D2L, &[Double], Long);
declare_pop_slice_and_push!(D2F, &[Double], Float);

declare_pop_slice_and_push!(I2B, &[Int], Int);
declare_pop_slice_and_push!(I2C, &[Int], Int);
declare_pop_slice_and_push!(I2S, &[Int], Int);

declare_pop_slice_and_push!(LCMP, &[Long, Long], Long);
declare_pop_slice_and_push!(FCMPL, &[Float, Float], Float);
declare_pop_slice_and_push!(FCMPG, &[Float, Float], Float);
declare_pop_slice_and_push!(DCMPL, &[Double, Double], Double);
declare_pop_slice_and_push!(DCMPG, &[Double, Double], Double);

macro_rules! declare_if(
    ($name:ident) =>{
        declare!($name,|mut e,on_jump,_,_|{
            pop_match(e.frame, &Int)?;
            let target = *e.offset + next::<u16>(e.byte_code, e.offset)? as u32;
            on_jump(&mut e, target.try_into()?)
        });
    }
);

declare_if!(IFEQ);
declare_if!(IFNE);
declare_if!(IFLT);
declare_if!(IFGE);
declare_if!(IFGT);
declare_if!(IFLE);

macro_rules! declare_if_icmp(
    ($name:ident) =>{
        declare!($name,|mut e,on_jump,_,_|{
            pop_match(e.frame, &Int)?;
            pop_match(e.frame, &Int)?;
            let target = *e.offset-1 + next::<u16>(e.byte_code, e.offset)? as u32;
            on_jump(&mut e, target.try_into()?)
        });
    }
);

declare_if_icmp!(IF_ICMPEQ);
declare_if_icmp!(IF_ICMPNE);
declare_if_icmp!(IF_ICMPLT);
declare_if_icmp!(IF_ICMPGE);
declare_if_icmp!(IF_ICMPGT);
declare_if_icmp!(IF_ICMPLE);

macro_rules! declare_if_acmp(
    ($name:ident) =>{
        declare!($name,|mut e,on_jump,_,_|{
            let arg0=pop(e.frame)?
              .try_as_reference()
              .ok_or_else(|| format_err!("not a reference"))?;
            let arg1=pop(e.frame)?
              .try_as_reference()
              .ok_or_else(|| format_err!("not a reference"))?;
            let target = *e.offset-1 + next::<u16>(e.byte_code, e.offset)? as u32;
            on_jump(&mut e, target.try_into()?)
        });
    }
);
declare_if_acmp!(IF_ACMPEQ);
declare_if_acmp!(IF_ACMPNE);

const GOTO: BytecodeVerifier = |mut e, on_jump, _, _| {
    let target = *e.offset - 1 + next::<u16>(e.byte_code, e.offset)? as u32;
    on_jump(&mut e, target.try_into()?)
};
const JSR: BytecodeVerifier = |mut e, on_jump, _, _| {
    let target = *e.offset - 1 + next::<u16>(e.byte_code, e.offset)? as u32;
    push(e.frame, e.code.max_stack, ReturnAddress)?;
    on_jump(&mut e, target.try_into()?)
};
const RET: BytecodeVerifier = |mut e, _, _, _| {
    let index = next::<u8>(&e.byte_code, e.offset)?;
    load_match(e.frame, index as u16, &ReturnAddress)?;
    Ok(())
};
fn alias_4(offset: &mut u32) {
    *offset = 0x3 & (*offset + 0x3);
}
const TABLESWITCH: BytecodeVerifier = |mut e, on_jump, _, _| {
    let instruction_start_offset = *e.offset - 1;
    alias_4(e.offset);
    let default = next::<u32>(e.byte_code, e.offset)? + instruction_start_offset;
    on_jump(&mut e, default.try_into()?)?;
    let low = next::<u32>(e.byte_code, e.offset)?;
    let high = next::<u32>(e.byte_code, e.offset)?;
    if high < low {
        Err(format_err!(
            "The value low must be less than or equal to high."
        ))?;
    }
    for _ in low..=high {
        let target = next::<u32>(e.byte_code, e.offset)? + instruction_start_offset;
        on_jump(&mut e, target.try_into()?)?;
    }
    Ok(())
};
const LOOKUPSWITCH: BytecodeVerifier = |mut e, on_jump, _, _| {
    let instruction_start_offset: u32 = *e.offset - 1;
    alias_4(e.offset);
    let default = next::<u32>(e.byte_code, e.offset)? + instruction_start_offset;
    on_jump(&mut e, default.try_into()?)?;
    let pair_count = next::<u32>(e.byte_code, e.offset)?;
    let mut last = i64::MIN;
    for _ in 0..pair_count {
        let key = next::<u32>(e.byte_code, e.offset)?;
        if key as i64 <= last {
            Err(format_err!(" The table match-offset pairs of the lookupswitch instruction must be sorted in increasing numerical order by match. "))?;
        }
        let target = next::<u32>(e.byte_code, e.offset)? + instruction_start_offset;
        on_jump(&mut e, target.try_into()?)?;
        last = key as i64;
    }
    Ok(())
};
macro_rules! declare_return {
    ($name:ident, $pop_type:tt) => {
        declare!($name, |mut e, _, _, on_return| {
            match e.method_ref.get_return_type().map(|r| r.java_type()) {
                Some(Type::Primitive(PrimitiveType::$pop_type)) => {}
                _ => Err(format_err!("wrong return type of the method"))?,
            }
            on_return(&mut e)
        });
    };
}
declare_return!(IRETURN, Int);
declare_return!(LRETURN, Long);
declare_return!(FRETURN, Float);
declare_return!(DRETURN, Double);
const ARETURN: BytecodeVerifier = |mut e, _, _, on_return| {
    let return_type = e
        .method_ref
        .get_return_type()
        .ok_or_else(|| format_err!("the return type of the method is void"))?
        .raw_class_owned()
        .clone();
    pop_match(e.frame, &Reference(return_type.clone()))?;
    on_return(&mut e)
};

const RETURN: BytecodeVerifier = |mut e, _, _, on_return| {
    match e.method_ref.get_return_type() {
        None => {}
        _ => Err(format_err!("the return type of the method is not void"))?,
    }
    on_return(&mut e)
};
const GET_STATIC: BytecodeVerifier = |e, _, _, _| {
    let constant = next_constant::<u8>(e.byte_code, e.offset, e.constants)?;
    match constant {
        Constant::ConstantFieldRef(f) => {
            push(
                e.frame,
                e.code.max_stack,
                FrameDataType::from_type_symbol(e.class_loader, &f.symbol.descriptor)?,
            )?;
            Ok(())
        }
        _ => Err(format_err!("except ConstantFieldRef, found {:?}", constant))?,
    }
};
const PUT_STATIC: BytecodeVerifier = |e, _, _, _| {
    let constant = next_constant::<u8>(e.byte_code, e.offset, e.constants)?;
    match constant {
        Constant::ConstantFieldRef(f) => {
            pop_match(
                e.frame,
                &FrameDataType::from_type_symbol(e.class_loader, &f.symbol.descriptor)?,
            )?;
            Ok(())
        }
        _ => Err(format_err!("except ConstantFieldRef, found {:?}", constant))?,
    }
};
const GET_FIELD: BytecodeVerifier = |e, _, _, _| {
    let constant = next_constant::<u8>(e.byte_code, e.offset, e.constants)?;
    match constant {
        Constant::ConstantFieldRef(f) => {
            pop_match(
                e.frame,
                &FrameDataType::from_type_symbol(e.class_loader, &f.class.symbol)?,
            )?;
            push(
                e.frame,
                e.code.max_stack,
                FrameDataType::from_type_symbol(e.class_loader, &f.symbol.descriptor)?,
            )?;
            Ok(())
        }
        _ => Err(format_err!("except ConstantFieldRef, found {:?}", constant))?,
    }
};
const PUT_FIELD: BytecodeVerifier = |e, _, _, _| {
    let constant = next_constant::<u8>(e.byte_code, e.offset, e.constants)?;
    match constant {
        Constant::ConstantFieldRef(f) => {
            pop_match(
                e.frame,
                &FrameDataType::from_type_symbol(e.class_loader, &f.symbol.descriptor)?,
            )?;
            pop_match(
                e.frame,
                &FrameDataType::from_type_symbol(e.class_loader, &f.class.symbol)?,
            )?;
            Ok(())
        }
        _ => Err(format_err!("except ConstantFieldRef, found {:?}", constant))?,
    }
};
fn verify_method_type(
    frame: &mut FrameState,
    class_loader: &ClassLoaderRef,
    max_stack: u16,
    method_class_symbol: &TypeSymbol,
    method_symbol: &MethodSymbol,
    this_class_symbol: Option<&TypeSymbol>,
    is_invoke_special: bool,
) -> Result<()> {
    if &*method_symbol.name == "<clinit>" {
        Err(format_err!("invalid method name:'<clinit>'"))?;
    }
    if !is_invoke_special && &*method_symbol.name == "<init>" {
        Err(format_err!(
            "method name '<init>' can only use by invokespecial"
        ))?;
    }
    for t in method_symbol.descriptor.parameters.iter().rev() {
        pop_match(frame, &FrameDataType::from_type_symbol(class_loader, &t)?)?;
    }
    if let Some(this_class_symbol) = this_class_symbol {
        let this_class = class_loader.get_class(&method_class_symbol.name)?;
        let cell = pop(frame)?;
        match cell {
            Reference(class) => {
                if &*method_symbol.name == "<init>" {
                    Err(format_err!("invalid method name:'<init>'"))?;
                }
                if !class.is_assignable(&this_class)? {
                    Err(format_err!(
                        "invalid parameter type:{:?},except:{:?}",
                        class,
                        this_class
                    ))?;
                }
            }
            UninitializedVariable(Some((class, p))) => {
                if &*method_symbol.name != "<init>" {
                    Err(format_err!("invalid method name:{}", method_symbol.name))?;
                }
                if !class.is_assignable(&this_class)? {
                    Err(format_err!(
                        "invalid parameter type:{:?},except:{:?}",
                        class,
                        this_class
                    ))?;
                }
                replice_cell(
                    frame,
                    &UninitializedVariable(Some((class.clone(), p))),
                    Reference(class),
                );
            }
            UninitializedThis(class) => {
                if &*method_symbol.name != "<init>" {
                    Err(format_err!("invalid method name:{}", method_symbol.name))?;
                }
                if !class.is_assignable(&this_class)? {
                    Err(format_err!(
                        "invalid parameter type:{:?},except:{:?}",
                        class,
                        this_class
                    ))?;
                }
                replice_cell(frame, &UninitializedThis(class.clone()), Reference(class));
            }
            _ => Err(format_err!("except reference,found:{:?}", cell))?,
        }
    }
    push(
        frame,
        max_stack,
        FrameDataType::from_type_symbol(class_loader, &method_symbol.descriptor.return_type)?,
    )?;
    Ok(())
}
const INVOKE_VIRTUAL: BytecodeVerifier = |e, _, _, _| {
    let constant = next_constant::<u8>(e.byte_code, e.offset, e.constants)?;
    match constant {
        Constant::ConstantMethodRef(c) => verify_method_type(
            e.frame,
            e.class_loader,
            e.code.max_stack,
            &c.class.symbol,
            &c.symbol,
            Some(&c.class.symbol),
            false,
        ),
        _ => Err(format_err!(
            "except ConstantMethodRef, found {:?}",
            constant
        ))?,
    }
};
const INVOKE_SPECIAL: BytecodeVerifier = |e, _, _, _| {
    let constant = next_constant::<u8>(e.byte_code, e.offset, e.constants)?;
    match constant {
        Constant::ConstantMethodRef(c) => verify_method_type(
            e.frame,
            e.class_loader,
            e.code.max_stack,
            &c.class.symbol,
            &c.symbol,
            Some(&c.class.symbol),
            true,
        ),
        _ => Err(format_err!(
            "except ConstantMethodRef, found {:?}",
            constant
        ))?,
    }
};
const INVOKE_STATIC: BytecodeVerifier = |e, _, _, _| {
    let constant = next_constant::<u8>(e.byte_code, e.offset, e.constants)?;
    match constant {
        Constant::ConstantMethodRef(c) => verify_method_type(
            e.frame,
            e.class_loader,
            e.code.max_stack,
            &c.class.symbol,
            &c.symbol,
            None,
            false,
        ),
        _ => Err(format_err!(
            "except ConstantMethodRef, found {:?}",
            constant
        ))?,
    }
};
const INVOKE_INTERFACE: BytecodeVerifier = |e, _, _, _| {
    let constant_index = next::<u16>(e.byte_code, e.offset)?;
    let constant = get_constant(e.constants, constant_index)?;
    match constant {
        Constant::ConstantInterfaceMethodRef(c) => {
            let byte3 = next::<u8>(e.byte_code, e.offset)?;
            let byte4 = next::<u8>(e.byte_code, e.offset)?;
            if byte3 == 0 || byte4 != 0 {
                Err(format_err!("illegal format"))?
            } else {
                verify_method_type(
                    e.frame,
                    e.class_loader,
                    e.code.max_stack,
                    &c.class.symbol,
                    &c.symbol,
                    Some(&c.class.symbol),
                    false,
                )
            }
        }
        _ => Err(format_err!(
            "except ConstantMethodRef, found {:?}",
            constant
        ))?,
    }
};
const INVOKE_DYNAMIC: BytecodeVerifier = |e, _, _, _| todo!();
const NEW: BytecodeVerifier = |e, _, _, _| {
    let constant_index = next::<u16>(e.byte_code, e.offset)?;
    let java_class = resolve_class_or_interface(&e, constant_index)?;
    for cell in e.frame.stack().iter().chain(e.frame.locals().iter()) {
        match cell {
            UninitializedVariable(Some((_, call_point))) => {
                if *call_point as u32 == *e.offset {
                    Err(format_err!("their is uninitialized variable in the frame"))?;
                }
            }
            _ => {}
        }
    }
    push(
        e.frame,
        e.code.max_stack,
        UninitializedVariable(Some((java_class, constant_index))),
    )?;
    Ok(())
};
const NEW_ARRAY: BytecodeVerifier = |e, _, _, _| {
    let bootstrap_class_set = bootstrap_class_set(e.class_loader);
    let component_class = match next::<u8>(e.byte_code, e.offset)? {
        4 => bootstrap_class_set.boolean(),
        5 => bootstrap_class_set.char(),
        6 => bootstrap_class_set.float(),
        7 => bootstrap_class_set.double(),
        8 => bootstrap_class_set.byte(),
        9 => bootstrap_class_set.short(),
        10 => bootstrap_class_set.int(),
        11 => bootstrap_class_set.long(),
        _ => {
            return Err(format_err!("invalid class_file format")).into();
        }
    };
    pop_match(e.frame, &Int)?;
    push(
        e.frame,
        e.code.max_stack,
        Reference(component_class.get_array_class()?),
    )?;
    Ok(())
};
const ANE_ARRAY: BytecodeVerifier = |mut e, _, _, _| {
    let constant_index = next::<u16>(e.byte_code, e.offset)?;
    let component_class = resolve_class_or_interface(&mut e, constant_index)?;
    pop_match(e.frame, &Int)?;
    push(
        e.frame,
        e.code.max_stack,
        Reference(component_class.get_array_class()?),
    )?;
    Ok(())
};
const ARRAY_LENGTH: BytecodeVerifier = |e, _, _, _| {
    let cell = pop(e.frame)?;
    match cell {
        Reference(array_class) if array_class.is_array()? => {}
        _ => Err(format_err!("except array type,found {:?}", cell,))?,
    }
    push(e.frame, e.code.max_stack, Int)?;
    Ok(())
};
const ATHROW: BytecodeVerifier = |mut e, _, on_athrow, _| {
    let constant_index = next::<u16>(e.byte_code, e.offset)?;
    let exception_class = resolve_class_or_interface(&mut e, constant_index)?;
    if !exception_class.is_assignable(&bootstrap_class_set(e.class_loader).java_lang_exception())? {
        Err(format_err!(
            "except a exception class,found {:?}",
            exception_class
        ))?;
    }
    on_athrow(&mut e, Some(&exception_class))?;
    Ok(())
};
const CHECK_CAST: BytecodeVerifier = |mut e, _, _, _| {
    let cell = pop(e.frame)?;
    match cell {
        Reference(java_class) => {}
        _ => Err(format_err!("except reference,found {:?}", cell,))?,
    }
    let constant_index = next::<u16>(e.byte_code, e.offset)?;
    let _target_class = resolve_class_or_interface(&mut e, constant_index)?;
    push(e.frame, e.code.max_stack, Int)?;
    Ok(())
};
const INSTANCE_OF: BytecodeVerifier = |mut e, _, _, _| {
    let cell = pop(e.frame)?;
    match cell {
        Reference(java_class) => {}
        _ => Err(format_err!("except reference,found {:?}", cell,))?,
    }
    let constant_index = next::<u16>(e.byte_code, e.offset)?;
    let target_class = resolve_class_or_interface(&mut e, constant_index)?;
    push(e.frame, e.code.max_stack, Reference(target_class))?;
    Ok(())
};
const MONITOR_ENTER: BytecodeVerifier = |e, _, _, _| {
    let cell = pop(e.frame)?;
    match cell {
        Reference(java_class) => {}
        _ => Err(format_err!("except reference,found {:?}", cell,))?,
    }
    Ok(())
};
const MONITOR_EXIT: BytecodeVerifier = |e, _, _, _| {
    let cell = pop(e.frame)?;
    match cell {
        Reference(java_class) => {}
        _ => Err(format_err!("except reference,found {:?}", cell,))?,
    }
    Ok(())
};

const WIDE: BytecodeVerifier = |e, _, _, _| {
    let instruction = next::<u8>(e.byte_code, e.offset)?;
    match instruction {
        code::byte_code::ILOAD..=code::byte_code::ALOAD => {
            let index = next::<u16>(e.byte_code, e.offset)?;
            let cell = load(e.frame, index)?.clone();
            match (&cell, instruction) {
                (Int, code::byte_code::ILOAD)
                | (Long, code::byte_code::LLOAD)
                | (Float, code::byte_code::FLOAD)
                | (Double, code::byte_code::DLOAD)
                | (Reference(_), code::byte_code::ALOAD) => {}
                _ => Err(format_err!("wrong value type"))?,
            }
            push(e.frame, e.code.max_stack, cell)?;
        }
        code::byte_code::ISTORE..=code::byte_code::ASTORE => {
            let index = next::<u16>(e.byte_code, e.offset)?;
            let cell = pop(e.frame)?;
            match (&cell, instruction) {
                (Int, code::byte_code::ISTORE)
                | (Long, code::byte_code::LSTORE)
                | (Float, code::byte_code::FSTORE)
                | (Double, code::byte_code::DSTORE)
                | (Reference(_), code::byte_code::ASTORE)
                | (ReturnAddress, code::byte_code::ASTORE) => {}
                _ => Err(format_err!("wrong value type"))?,
            }
            store(e.frame, index, cell);
        }
        code::byte_code::RET => {
            let index = next::<u16>(e.byte_code, e.offset)?;
            load_match(e.frame, index, &ReturnAddress)?;
        }
        code::byte_code::IINC => {
            let index = next::<u16>(e.byte_code, e.offset)?;
            let _const_value = next::<u16>(e.byte_code, e.offset)?;
            load_match(e.frame, index, &Int)?;
        }
        _ => {
            Err(format_err!("invalid opcode: {:X}", instruction))?;
        }
    }
    Ok(())
};
const MULTIA_NEW_ARRAY: BytecodeVerifier = |e, _, _, _| {
    let constant_index = next::<u16>(e.byte_code, e.offset)?;
    let array_class = resolve_class_or_interface(&e, constant_index)?;
    if !array_class.is_array()? {
        Err(format_err!("except a array type,found {:?}", array_class))?;
    }
    let dimensions = next::<u8>(e.byte_code, e.offset)?;
    let name = &*array_class.binary_name();
    let mut name_iter = name.bytes();
    for _ in 0..dimensions {
        if Some(b'[') != name_iter.next() {
            Err(format_err!(
                "array dimensions error, array type:{:?}",
                array_class
            ))?;
        }
    }
    for _ in 0..dimensions {
        pop_match(e.frame, &Int)?;
    }
    push(e.frame, e.code.max_stack, Reference(array_class))?;
    Ok(())
};
const IF_NULL: BytecodeVerifier = |mut e, on_jump, _, _| {
    let cell = pop(e.frame)?;
    match cell {
        Reference(java_class) => {}
        _ => Err(format_err!("except reference,found {:?}", cell,))?,
    }
    let target = *e.offset - 1 + next::<u16>(e.byte_code, e.offset)? as u32;
    on_jump(&mut e, target.try_into()?)
};
const IF_NON_NULL: BytecodeVerifier = |mut e, on_jump, _, _| {
    let cell = pop(e.frame)?;
    match cell {
        Reference(java_class) => {}
        _ => Err(format_err!("except reference,found {:?}", cell,))?,
    }
    let target = *e.offset - 1 + next::<u16>(e.byte_code, e.offset)? as u32;
    on_jump(&mut e, target.try_into()?)
};
const GOTO_W: BytecodeVerifier = |mut e, on_jump, _, _| {
    let target = *e.offset - 1 + next::<u16>(e.byte_code, e.offset)? as u32;
    on_jump(&mut e, target.try_into()?)
};
const JSR_W: BytecodeVerifier = |mut e, on_jump, _, _| {
    let target_offset = next::<u32>(e.byte_code, e.offset)?;
    let target = e
        .offset
        .checked_sub(1)
        .and_then(|s| s.checked_add(target_offset))
        .ok_or_else(|| format_err!("u32 overflow"))?;
    push(e.frame, e.code.max_stack, ReturnAddress)?;
    on_jump(&mut e, target.try_into()?)
};

const BYTECODE_VERIFIERS: [BytecodeVerifier; 202] = [
    NOP,
    ACONST_NULL,
    ICONST_M1,
    ICONST_0,
    ICONST_1,
    ICONST_2,
    ICONST_3,
    ICONST_4,
    ICONST_5,
    LCONST_0,
    LCONST_1,
    FCONST_0,
    FCONST_1,
    FCONST_2,
    DCONST_0,
    DCONST_1,
    BIPUSH,
    SIPUSH,
    LDC,
    LDC_W,
    LDC2_W,
    ILOAD,
    LLOAD,
    FLOAD,
    DLOAD,
    ALOAD,
    ILOAD_0,
    ILOAD_1,
    ILOAD_2,
    ILOAD_3,
    LLOAD_0,
    LLOAD_1,
    LLOAD_2,
    LLOAD_3,
    FLOAD_0,
    FLOAD_1,
    FLOAD_2,
    FLOAD_3,
    DLOAD_0,
    DLOAD_1,
    DLOAD_2,
    DLOAD_3,
    ALOAD_0,
    ALOAD_1,
    ALOAD_2,
    ALOAD_3,
    IALOAD,
    LALOAD,
    FALOAD,
    DALOAD,
    AALOAD,
    BALOAD,
    CALOAD,
    SALOAD,
    ISTORE,
    LSTORE,
    FSTORE,
    DSTORE,
    ASTORE,
    ISTORE_0,
    ISTORE_1,
    ISTORE_2,
    ISTORE_3,
    LSTORE_0,
    LSTORE_1,
    LSTORE_2,
    LSTORE_3,
    FSTORE_0,
    FSTORE_1,
    FSTORE_2,
    FSTORE_3,
    DSTORE_0,
    DSTORE_1,
    DSTORE_2,
    DSTORE_3,
    ASTORE_0,
    ASTORE_1,
    ASTORE_2,
    ASTORE_3,
    IASTORE,
    LASTORE,
    FASTORE,
    DASTORE,
    AASTORE,
    BASTORE,
    CASTORE,
    SASTORE,
    POP,
    POP2,
    DUP,
    DUP_X1,
    DUP_X2,
    DUP2,
    DUP2_X1,
    DUP2_X2,
    SWAP,
    IADD,
    LADD,
    FADD,
    DADD,
    ISUB,
    LSUB,
    FSUB,
    DSUB,
    IMUL,
    LMUL,
    FMUL,
    DMUL,
    IDIV,
    LDIV,
    FDIV,
    DDIV,
    IREM,
    LREM,
    FREM,
    DREM,
    INEG,
    LNEG,
    FNEG,
    DNEG,
    ISHL,
    LSHL,
    ISHR,
    LSHR,
    IUSHR,
    LUSHR,
    IAND,
    LAND,
    IOR,
    LOR,
    IXOR,
    LXOR,
    IINC,
    I2L,
    I2F,
    I2D,
    L2I,
    L2F,
    L2D,
    F2I,
    F2L,
    F2D,
    D2I,
    D2L,
    D2F,
    I2B,
    I2C,
    I2S,
    LCMP,
    FCMPL,
    FCMPG,
    DCMPL,
    DCMPG,
    IFEQ,
    IFNE,
    IFLT,
    IFGE,
    IFGT,
    IFLE,
    IF_ICMPEQ,
    IF_ICMPNE,
    IF_ICMPLT,
    IF_ICMPGE,
    IF_ICMPGT,
    IF_ICMPLE,
    IF_ACMPEQ,
    IF_ACMPNE,
    GOTO,
    JSR,
    RET,
    TABLESWITCH,
    LOOKUPSWITCH,
    IRETURN,
    LRETURN,
    FRETURN,
    DRETURN,
    ARETURN,
    RETURN,
    GET_STATIC,
    PUT_STATIC,
    GET_FIELD,
    PUT_FIELD,
    INVOKE_VIRTUAL,
    INVOKE_SPECIAL,
    INVOKE_STATIC,
    INVOKE_INTERFACE,
    INVOKE_DYNAMIC,
    NEW,
    NEW_ARRAY,
    ANE_ARRAY,
    ARRAY_LENGTH,
    ATHROW,
    CHECK_CAST,
    INSTANCE_OF,
    MONITOR_ENTER,
    MONITOR_EXIT,
    WIDE,
    MULTIA_NEW_ARRAY,
    IF_NULL,
    IF_NON_NULL,
    GOTO_W,
    JSR_W,
];
pub fn verify_code(environment: &mut Environment) -> Result<()> {
    let Environment {
        offset,
        byte_code,
        code,
        method,
        method_ref,
        constants,
        class,
        class_loader,
        frame,
    } = environment;
    let modifiers = method_ref.modifiers();
    let is_static = modifiers.is_static();
    let is_constructor = &*method.name == "<init>";
    let len = code.code.len();
    let mut frame = FrameState::new(
        (!is_static).then_some(class),
        is_constructor,
        code.max_stack,
        code.max_locals,
        &method_ref,
    );

    if let Some(Attribute::StackMapTable(frame_map)) = code.attributes.get("StackMapTable") {
        let mut offset = 0u32;
        let frame_map = FrameState::from_stack_map_table(
            frame_map,
            &mut Environment {
                offset: &mut offset,
                byte_code,
                code,
                method,
                method_ref,
                constants,
                class,
                class_loader,
                frame: &mut frame,
            },
        )?;
        while (offset as usize) < len {
            let mut environment = Environment {
                offset: &mut offset,
                byte_code,
                code,
                method,
                method_ref,
                constants,
                class,
                class_loader,
                frame: &mut frame,
            };
            let instruction = next::<u8>(environment.byte_code, environment.offset)?;
            let byte_code_verifier = BYTECODE_VERIFIERS[instruction as usize];
            let mut on_jump = |e: &mut Environment, target: u32| {
                if let Some(frame_state) = frame_map.get(&target) {
                    if !frame_state.is_assignable(&e.frame)? {
                        Err(format_err!("frame state error"))?;
                    }
                }
                Ok(())
            };
            let mut on_throw = |e: &mut Environment, _: Option<&JavaClassRef>| Ok(());
            let mut on_return = |e: &mut Environment| Ok(());
            let is_wide_ret = instruction == code::byte_code::WIDE
                && byte_code.get::<u8>((*environment.offset + 1) as usize) == code::byte_code::RET;

            byte_code_verifier(
                environment,
                &mut on_jump as &mut dyn FnMut(&mut Environment, u32) -> Result<()>,
                &mut on_throw
                    as &mut dyn FnMut(&mut Environment, Option<&JavaClassRef>) -> Result<()>,
                &mut on_return as &mut dyn FnMut(&mut Environment) -> Result<()>,
            )
            .map_err(|e| {
                format_err!(
                    "error when verifier the code,offset:{},instruction:{:X},error:{:#?}",
                    offset,
                    instruction,
                    e
                )
            })?;
            match instruction {
                code::byte_code::GOTO
                | code::byte_code::GOTO_W
                | code::byte_code::JSR
                | code::byte_code::JSR_W
                | code::byte_code::RET
                | code::byte_code::IRETURN
                | code::byte_code::LRETURN
                | code::byte_code::DRETURN
                | code::byte_code::FRETURN
                | code::byte_code::ARETURN
                | code::byte_code::RETURN => {
                    frame = frame_map
                        .get(&offset)
                        .ok_or_else(|| format_err!("stack_map_frame not found"))?
                        .clone()
                }
                code::byte_code::WIDE if is_wide_ret => {
                    frame = frame_map
                        .get(&offset)
                        .ok_or_else(|| format_err!("stack_map_frame not found"))?
                        .clone()
                }
                _ => {
                    if offset as usize >= len {
                        Err(format_err!("invalid code format"))?;
                    }
                    if let Some(frame_state) = frame_map.get(&offset) {
                        if !frame_state.is_assignable(&frame)? {
                            Err(format_err!("frame state error"))?;
                        }
                    }
                }
            }
        }
    } else {
        let mut frame_state = HashMap::new();
        let mut buffer = Vec::new();
        let mut offset = 0u32;
        buffer.push(0);
        while let Some(next_offset) = buffer.pop() {
            let mut last_frame = frame.clone();
            let current_offset = offset;
            let mut environment = Environment {
                offset: &mut offset,
                byte_code,
                code,
                method,
                method_ref,
                constants,
                class,
                class_loader,
                frame: &mut frame,
            };
            *environment.offset = next_offset;
            let instruction = next::<u8>(environment.byte_code, environment.offset)?;
            let byte_code_verifier = BYTECODE_VERIFIERS[instruction as usize];
            let mut on_jump = |e: &mut Environment, target: u32| match frame_state.entry(target) {
                Entry::Occupied(mut o) => {
                    let target_state: &mut FrameState = o.get_mut();
                    if target_state.merge_assign(e.frame)? {
                        buffer.push(current_offset);
                    }
                    Ok(())
                }
                Entry::Vacant(o) => {
                    o.insert(e.frame.clone());
                    buffer.push(target);
                    Ok(())
                }
            };
            let mut on_throw = |e: &mut Environment, _: Option<&JavaClassRef>| Ok(());
            let mut on_return = |e: &mut Environment| Ok(());
            let is_wide_ret = instruction == code::byte_code::WIDE
                && byte_code.get::<u8>((*environment.offset + 1) as usize) == code::byte_code::RET;

            byte_code_verifier(
                environment,
                &mut on_jump as &mut dyn FnMut(&mut Environment, u32) -> Result<()>,
                &mut on_throw
                    as &mut dyn FnMut(&mut Environment, Option<&JavaClassRef>) -> Result<()>,
                &mut on_return as &mut dyn FnMut(&mut Environment) -> Result<()>,
            )
            .map_err(|e| {
                format_err!(
                    "error when verifier the code,offset:{},instruction:{:X},error:{:#?}",
                    offset,
                    instruction,
                    e
                )
            })?;
            match frame_state.entry(offset) {
                Entry::Occupied(mut o) => {
                    if o.get_mut().merge_assign(&frame)? {
                        buffer.push(offset);
                    }
                }
                Entry::Vacant(o) => {
                    o.insert(frame.clone());
                    buffer.push(offset);
                }
            }
        }
        todo!(); // TODO
    }
    todo!() // TODO
}
