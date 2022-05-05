use std::borrow::{Borrow, Cow};

use vm_core::Type;

use util::{CowArc, CowSlice};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BootstrapInstruction {
    Nop,

    Move,

    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Neg,

    And,
    Or,
    Xor,
    Not,
    Shl,
    Shr,
    Ushr,

    CmpLt,
    CmpLe,
    CmpGt,
    CmpGe,
    CmpEq,
    CmpNe,

    UcmpLt,
    UcmpLe,
    UcmpGe,
    UcmpGt,
    UcmpEq,
    UcmpNe,

    IntToFloat,
    FloatToFloat,
    FloatToInt,

    IntExtend,
    UIntExtend,
    IntTruncate,

    FAdd,
    FSub,
    FMul,
    FDiv,
    FRem,
    FNeg,

    FcmpLt,
    FcmpLe,
    FcmpGe,
    FcmpGt,
    FcmpEq,
    FcmpNe,

    CastUnchecked,

    Branch,
    BranchIf,
    // Switch,
    /// fn<type fn_type>(fn:Pointer<U8>,args...)->(o)
    Call,
    /// fn<type fn_type,block then,block catch>(fn:Pointer<U8>,args...,vaargs:Slice<U8>)->(o)
    Invoke,
    /// fn<type fn_type>(fn:Pointer<U8>,args...,vaargs:Slice<U8>)->(o)
    MakeSlice,
    Return,

    /// fn<type ty>()->(out:Pointer<Type>)
    StackAlloc,
    /// fn<type ty>(in:Usize)->(out:Pointer<Type>)
    StackAllocUnsized,
    /// fn<type ty>(in:Pointer<Type>)->(out:Type)
    Read,
    /// fn<type ty>(ptr:Pointer<Type>,value:Type)
    Write,

    /// fn<type ty,const ptr:Pointer<U8>,const symbol:&str>(...)->(r);
    NativeCall,

    /// fn<type ty:Type::Reference>(in:Reference<TYPE>)->(out:Pointer<TYPE>)
    Deref,
    /// fn<type ty:Type::Reference>(in:Reference<TYPE>)->(out:Reference<TYPE>)
    Clone,
    /// fn<type ty:Type::Reference>(in:Reference<TYPE>)->()
    Drop,
    /// fn<type ty>(in:Enum)->(out:U32)
    GetTag,
    /// fn<type ty>(in:Pointer<Enum>)->(out:U32)
    ReadTag,
    /// fn<type ty>(in:Pointer<Enum>,tag:U32)
    WriteTag,
    /// fn<type ty,const TAG:U32>(in:Enum)->(out:Variant)
    DecodeVariantUnchecked,
    /// fn<type ty,const TAG:U32>(in:Variant)->(out:Variant)
    EncodeVariant,

    /// fn<type ty,const INDEX:U32>(in:Pointer<Struct>)->(out:Pointer<Field>)
    LocateField,
    /// fn<type ty,const INDEX:U32>(in:Struct)->(out:Field)
    GetField,
    /// fn<type ty,const INDEX:U32>(s:Struct,f:Field)->(s:Struct)
    SetField,
    /// fn<type ty>()->(out:Struct)
    UninitedStruct,

    /// fn<type ty,const TAG:U32>(in:Reference<Union>)->(out:Reference<Variant>)
    LocateUnion,

    /// fn<type ty>(array:Pointer<Array>,index:Usize)->(out:Pointer<Element>)
    LocateElement,

    /// fn<type ty>(array:Pointer<Array>)->(len:Usize)
    GetLength,
    /// fn<type ty>(array:Pointer<Array>,len:Usize)
    SetLength,

    /// fn<type ty,const TIRE:U32>(in:Pointer<Object>)->(out:Pointer<Metadata>)
    LocateMetadata,

    /// fn<type ty>(ptr:Pointer<Type>,exception:Type,value:Type)->(Bool)
    CompareAndSwap,

    GetPointer,

    FenceReleased,
    FenceAcquire,
    FenceAcqrel,
    FenceSeqcst,

    /// fn<const ty:TYPE>(in:Pointer<Type>)
    Free,
    /// fn<const ty:TYPE>()->(out:Reference)
    AllocSized,
    /// fn<const ty:TYPE>(in:Usize)->(out:Reference)
    AllocUnsized,
    /// fn<const ty:TYPE>()->(out:Pointer<Type>)
    NonGCAllocSized,
    /// fn<const ty:TYPE>(in:Usize)->(out:Pointer<Type>)
    NonGCAllocUnsized,
    /// fn<const ty:TYPE>(ptr:Pointer<Type>)->()
    NonGCFree,

    /// fn<const ty:TYPE>(dst:Pointer<Type>,src:Pointer<Type>,size:Usize)
    MemoryCopy,

    SetState,
    CallState,
}
#[derive(Debug, Clone)]
pub struct MemoryInstructionSet {
    pub clone: InstructionType,
    pub drop: InstructionType,
    pub deref: InstructionType,
    pub alloc: InstructionType,
    pub alloc_unsized: InstructionType,
    pub free: InstructionType,
    pub non_gc_alloc: InstructionType,
    pub non_gc_alloc_unsized: InstructionType,
    pub non_gc_free: InstructionType,
}

impl MemoryInstructionSet {
    pub fn get(&self, instruction: BootstrapInstruction) -> Option<&InstructionType> {
        use BootstrapInstruction::*;
        match instruction {
            Clone => Some(&self.clone),
            Drop => Some(&self.drop),
            Deref => Some(&self.deref),
            AllocSized => Some(&self.alloc),
            AllocUnsized => Some(&self.alloc_unsized),
            Free => Some(&self.free),
            NonGCAllocSized => Some(&self.non_gc_alloc),
            NonGCAllocUnsized => Some(&self.non_gc_alloc_unsized),
            NonGCFree => Some(&self.non_gc_free),
            _ => None,
        }
    }
}
pub struct RawRegister(u16);
pub trait InstructionSet: Sync + Send {
    const INSTRUCTIONS: CowSlice<'static, (usize, InstructionType)>;
    const INSTRUCTION_COUNT: usize;
}
pub trait Instruction {
    const INSTRUCTION_TYPE: InstructionType;
    const STATE_COUNT: usize;
}
pub trait InstructionOf<S: InstructionSet> {
    const OPCODE: usize;
}
#[derive(Clone)]
pub enum InstructionType {
    Bootstrap(BootstrapInstruction),
    // Proxy(CowArc<'static, ProxyInstruction>),
    Compression(CowArc<'static, CompressionInstruction>),
    Complex(CowArc<'static, ComplexInstruction>),
    Stateful(CowArc<'static, StatefulInstruction>),
}

impl InstructionType {
    pub fn get_name(&self) -> String {
        match self {
            InstructionType::Bootstrap(b) => format!("{:?}", b),
            InstructionType::Compression(_) => "Compression".to_string(),
            InstructionType::Complex(c) => c.name.to_string(),
            InstructionType::Stateful(_) => "Stateful".to_string(),
        }
    }
}

impl std::fmt::Debug for InstructionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bootstrap(arg0) => f.write_fmt(format_args!("bootstrap::{:?}", arg0)),
            Self::Compression(_arg0) => f.debug_tuple("Compression").finish(),
            Self::Complex(c) => f.debug_tuple(&*c.name).finish(),
            Self::Stateful(_arg0) => f.debug_tuple("Stateful").finish(),
        }
    }
}
impl InstructionType {
    pub fn state_count(&self) -> usize {
        match self {
            InstructionType::Bootstrap(_) => 1,
            // InstructionType::Proxy(_) => 1,
            InstructionType::Compression(_) => 1,
            InstructionType::Complex(_) => 1,
            InstructionType::Stateful(i) => i.statuses.len(),
        }
    }
}
#[derive(Debug, Clone)]
pub enum GenericsMetadataKind {
    Constant { value_type: Type, writable: bool },
    BasicBlock,
    Type,
    State,
}
#[derive(Debug, Clone)]
pub struct GenericsMetadata {
    pub kind: GenericsMetadataKind,
    pub name: Cow<'static, str>,
}
#[derive(Debug, Clone)]
pub struct OperandMetadata {
    pub input: bool,
    pub output: bool,
    pub name: Cow<'static, str>,
    pub value_type: Type,
}
#[derive(Clone)]
pub struct InstructionMetadata {
    pub operands: CowSlice<'static, OperandMetadata>,
    pub generics: CowSlice<'static, GenericsMetadata>,
}

impl std::fmt::Debug for InstructionMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("<{:?}>({:?})", &*self.generics, &*self.operands))
    }
}
#[derive(Debug, Clone)]
pub struct Phi {
    pub variable: Cow<'static, str>,
    pub ty: Type,
    pub map: CowSlice<'static, (Cow<'static, str>, Cow<'static, str>)>,
}
#[derive(Debug, Clone)]
pub enum GenericArgument {
    Var(Cow<'static, str>),
    Value(Value),
}
#[derive(Debug, Clone)]
pub enum Value {
    Str(Cow<'static, str>),
    ByteStr(CowSlice<'static, u8>),
    F32(f32),
    F64(f64),
    I64(i64),
    U8(u8),
    Bool(bool),
    RustFn(*const u8),
    Type(Type),
    Instruction(InstructionType),
}
impl From<&'static str> for Value {
    fn from(i: &'static str) -> Self {
        Self::Str(Cow::Borrowed(i))
    }
}
impl From<&'static [u8]> for Value {
    fn from(i: &'static [u8]) -> Self {
        Self::ByteStr(CowSlice::Ref(i))
    }
}
impl From<f32> for Value {
    fn from(i: f32) -> Self {
        Self::F32(i)
    }
}
impl From<f64> for Value {
    fn from(i: f64) -> Self {
        Self::F64(i)
    }
}
impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Self::I64(i)
    }
}
impl From<u8> for Value {
    fn from(i: u8) -> Self {
        Self::U8(i)
    }
}
impl From<bool> for Value {
    fn from(i: bool) -> Self {
        Self::Bool(i)
    }
}
impl From<Type> for Value {
    fn from(i: Type) -> Self {
        Self::Type(i)
    }
}
impl From<InstructionType> for Value {
    fn from(i: InstructionType) -> Self {
        Self::Instruction(i)
    }
}
#[derive(Clone)]
pub struct InstructionCall {
    pub args: CowSlice<'static, Cow<'static, str>>,
    pub rets: CowSlice<'static, Cow<'static, str>>,
    pub generics: CowSlice<'static, GenericArgument>,
    pub instruction: InstructionType,
}

impl std::fmt::Debug for InstructionCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}={:?}<{:?}>({:?})", &*self.rets, &self.instruction, &*self.generics, &*self.args))
    }
}
#[derive(Clone)]
pub enum Stat {
    InstructionCall(InstructionCall),
    Lit(Cow<'static, str>, Value),
    Move(Cow<'static, str>, Cow<'static, str>),
}

impl std::fmt::Debug for Stat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InstructionCall(arg0) => arg0.fmt(f),
            Self::Lit(arg0, arg1) => f.write_fmt(format_args!("{:?}={:?}", &**arg0, arg1)),
            Self::Move(arg0, arg1) => f.write_fmt(format_args!("{:?}={:?}", &**arg0, &**arg1)),
        }
    }
}
#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: Cow<'static, str>,
    pub phi: CowSlice<'static, Phi>,
    pub stat: CowSlice<'static, Stat>,
}
impl Borrow<str> for BasicBlock {
    fn borrow(&self) -> &str {
        &self.id
    }
}
#[derive(Debug, Clone)]
pub struct ComplexInstruction {
    pub name: Cow<'static, str>,
    pub metadata: InstructionMetadata,
    pub blocks: CowSlice<'static, BasicBlock>,
}
// #[derive(Debug,Clone)]
// pub struct ProxyInstruction {
//     pub metadata: InstructionMetadata,
//     pub implementations: CowSlice<'static, InstructionType>,
//     pub selector: Cow<'static, Arc<dyn Sync + Send + Fn(&[u8]) -> usize>>,
// }
#[derive(Debug, Clone)]
pub struct CompressionInstruction {
    pub instructions: CowSlice<'static, (usize, InstructionType)>,
    pub instruction_count: usize,
}
#[derive(Debug, Clone)]
pub struct StatefulInstruction {
    pub metadata: InstructionMetadata,
    pub boost: Cow<'static, str>,
    pub statuses: CowSlice<'static, State>,
}
#[derive(Debug, Clone)]
pub struct State {
    pub name: Cow<'static, str>,
    pub instruction: ComplexInstruction,
}
pub mod bootstrap {

    use failure::Fallible;
    use ghost_cell::GhostToken;
    use vm_core::{Slice, TypeDeclaration};

    use crate::{
        code::{BlockBuilder, Register, RegisterPool},
        instructions::InstructionOf,
    };

    use super::{BootstrapInstruction, Instruction, InstructionSet, InstructionType};
    macro_rules! declare_boostrap_instruction {
        ($name:ident) => {
            pub enum $name {}
            impl Instruction for $name {
                const INSTRUCTION_TYPE: InstructionType = InstructionType::Bootstrap(BootstrapInstruction::$name);
                const STATE_COUNT: usize = 1;
            }
        };
    }
    declare_boostrap_instruction!(Nop);
    declare_boostrap_instruction!(Move);
    declare_boostrap_instruction!(Add);
    declare_boostrap_instruction!(Sub);
    declare_boostrap_instruction!(Mul);
    declare_boostrap_instruction!(Div);
    declare_boostrap_instruction!(Rem);
    declare_boostrap_instruction!(Neg);
    declare_boostrap_instruction!(And);
    declare_boostrap_instruction!(Or);
    declare_boostrap_instruction!(Xor);
    declare_boostrap_instruction!(Not);
    declare_boostrap_instruction!(Shl);
    declare_boostrap_instruction!(Shr);
    declare_boostrap_instruction!(Ushr);
    declare_boostrap_instruction!(CmpLt);
    declare_boostrap_instruction!(CmpLe);
    declare_boostrap_instruction!(CmpGt);
    declare_boostrap_instruction!(CmpGe);
    declare_boostrap_instruction!(CmpEq);
    declare_boostrap_instruction!(CmpNe);
    declare_boostrap_instruction!(UcmpLt);
    declare_boostrap_instruction!(UcmpLe);
    declare_boostrap_instruction!(UcmpGe);
    declare_boostrap_instruction!(UcmpGt);
    declare_boostrap_instruction!(UcmpEq);
    declare_boostrap_instruction!(UcmpNe);
    declare_boostrap_instruction!(IntToFloat);
    declare_boostrap_instruction!(FloatToFloat);
    declare_boostrap_instruction!(FloatToInt);
    declare_boostrap_instruction!(IntExtend);
    declare_boostrap_instruction!(UIntExtend);
    declare_boostrap_instruction!(IntTruncate);
    declare_boostrap_instruction!(FAdd);
    declare_boostrap_instruction!(FSub);
    declare_boostrap_instruction!(FMul);
    declare_boostrap_instruction!(FDiv);
    declare_boostrap_instruction!(FRem);
    declare_boostrap_instruction!(FNeg);
    declare_boostrap_instruction!(FcmpLt);
    declare_boostrap_instruction!(FcmpLe);
    declare_boostrap_instruction!(FcmpGe);
    declare_boostrap_instruction!(FcmpGt);
    declare_boostrap_instruction!(FcmpEq);
    declare_boostrap_instruction!(FcmpNe);
    declare_boostrap_instruction!(CastUnchecked);
    declare_boostrap_instruction!(Branch);
    declare_boostrap_instruction!(BranchIf);
    declare_boostrap_instruction!(Call);
    declare_boostrap_instruction!(Invoke);
    declare_boostrap_instruction!(MakeSlice);
    impl MakeSlice {
        pub fn emit<'l, S: InstructionSet, T: TypeDeclaration, A: RegisterPool>(
            builder: &BlockBuilder<'l, S>,
            token: &mut GhostToken<'l>,
            arg_elements: &[Register<T, A>],
            ret_slice: &Register<Slice<T>, A>,
            array_reg: u16,
        ) -> Fallible<()>
        where
            Self: InstructionOf<S>,
        {
            unsafe {
                builder.emit_opcode(token, <Self as InstructionOf<S>>::OPCODE);
                let align = 8;
                builder.codes().borrow_mut(token).align(align);
                builder.emit(token, arg_elements.len());
                builder.emit(token, T::LAYOUT.into_flexible_array().flexible_size());
                for elem in arg_elements {
                    builder.emit_register(token, elem);
                }
                builder.emit(token, array_reg);
                builder.emit_register(token, ret_slice);
            }
            Ok(())
        }
    }
    declare_boostrap_instruction!(Return);
    declare_boostrap_instruction!(StackAlloc);
    declare_boostrap_instruction!(StackAllocUnsized);
    declare_boostrap_instruction!(Read);
    declare_boostrap_instruction!(Write);
    declare_boostrap_instruction!(NativeCall);
    declare_boostrap_instruction!(Deref);
    declare_boostrap_instruction!(Clone);
    declare_boostrap_instruction!(Drop);
    declare_boostrap_instruction!(GetTag);
    declare_boostrap_instruction!(ReadTag);
    declare_boostrap_instruction!(WriteTag);
    declare_boostrap_instruction!(DecodeVariantUnchecked);
    declare_boostrap_instruction!(EncodeVariant);
    declare_boostrap_instruction!(LocateField);
    declare_boostrap_instruction!(GetField);
    declare_boostrap_instruction!(SetField);
    declare_boostrap_instruction!(UninitedStruct);
    declare_boostrap_instruction!(LocateUnion);
    declare_boostrap_instruction!(LocateElement);
    declare_boostrap_instruction!(GetLength);
    declare_boostrap_instruction!(SetLength);
    declare_boostrap_instruction!(LocateMetadata);
    declare_boostrap_instruction!(CompareAndSwap);
    declare_boostrap_instruction!(GetPointer);
    declare_boostrap_instruction!(FenceReleased);
    declare_boostrap_instruction!(FenceAcquire);
    declare_boostrap_instruction!(FenceAcqrel);
    declare_boostrap_instruction!(FenceSeqcst);
    declare_boostrap_instruction!(Free);
    declare_boostrap_instruction!(AllocSized);
    declare_boostrap_instruction!(AllocUnsized);
    declare_boostrap_instruction!(NonGCAllocSized);
    declare_boostrap_instruction!(NonGCAllocUnsized);
    declare_boostrap_instruction!(NonGCFree);
    declare_boostrap_instruction!(MemoryCopy);
    declare_boostrap_instruction!(SetState);
    declare_boostrap_instruction!(CallState);
}
