use super::ir::I64ToF64;
use super::{ir, ir::*, lua_lexical::*};
use crate::error::LuaVMError;
use crate::instruction::{BreakPoint, CallFunction0VaSliceRet1, CallFunctionVaSliceRet1, ForInLoopJump, GetField, GetRet, GetVaArgs, NewUpValue, Return0VaSlice, ReturnVaSlice, SetElement, SetUpRef, SetUpValue};
use crate::instruction::{GetArg, InlineCacheLineImpl};
use crate::{instruction::{BranchIf, ConstM1, ConstNil, ConstZero, F64ToValue, I64ToValue}, mem::*};
use e::{Goto, F64, U8};
use failure::Fallible;
use getset::Getters;
use ghost_cell::{GhostCell, GhostToken};
use log::{debug, trace};
use runtime::code::{BlockBuilder, BuddyRegisterPool, FunctionBuilder, FunctionPack, RegisterPool};
use runtime::instructions::bootstrap::MakeSlice;
use vm_core::{FunctionTypeBuilder, ObjectBuilder, Slice, SymbolBuilder, SymbolRef, UnsizedArray};
use vm_core::{Pointer, TypeDeclaration};

use runtime_extra as e;
use runtime_extra::{NullableOptionImpl, Usize, I64};

use std::cell::RefCell;

use std::collections::HashMap;
use std::mem::{size_of, MaybeUninit};
use std::rc::Rc;

type Register<T> = runtime::code::Register<T, BuddyRegisterPool>;
#[derive(Clone)]
pub enum LuaRegister<'l> {
    Integer(Register<e::I64>),
    Float(Register<e::F64>),
    Function(Register<LuaValue>, LuaFunctionBuilderRef<'l>),
    Value(Register<LuaValue>, Option<()>),
}

impl<'l> std::fmt::Debug for LuaRegister<'l> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Integer(arg0) => f.debug_tuple("Integer").field(arg0).finish(),
            Self::Float(arg0) => f.debug_tuple("Float").field(arg0).finish(),
            Self::Function(arg0, _arg1) => f.debug_tuple("Function").field(arg0).finish(),
            Self::Value(arg0, _arg1) => f.debug_tuple("Value").field(arg0).finish(),
        }
    }
}
impl<'l> LuaRegister<'l> {
    pub(crate) fn kind(&self) -> LuaRegisterKind {
        match self {
            LuaRegister::Integer(_) => LuaRegisterKind::Integer,
            LuaRegister::Float(_) => LuaRegisterKind::Float,
            LuaRegister::Function(_, _) | LuaRegister::Value(_, _) => LuaRegisterKind::Value,
        }
    }
    pub fn reg_index(&self) -> u16 {
        match self {
            LuaRegister::Integer(r) => r.reg(),
            LuaRegister::Float(r) => r.reg(),
            LuaRegister::Function(r, _) | LuaRegister::Value(r, _) => r.reg(),
        }
    }
}
#[derive(Clone, Copy)]
pub enum LuaRegisterKind {
    Integer,
    Float,
    Value,
}
#[derive(Builder, Clone, Debug)]
pub struct LuaExpr<'l> {
    register: LuaRegister<'l>,
    #[builder(default)]
    lifetime: ExprLifeTimeKind,
}

impl<'l> LuaExpr<'l> {
    pub fn new_value(reg: Register<LuaValue>) -> LuaExprRef<'l> {
        Rc::new(Self {
            lifetime: Default::default(),
            register: LuaRegister::Value(reg, None),
        })
    }
    pub fn value_reg(&self) -> &Register<LuaValue> {
        match &self.register {
            LuaRegister::Value(r, _) => r,
            _ => panic!(),
        }
    }
}
pub type LuaExprRef<'l> = Rc<LuaExpr<'l>>;
#[derive(Debug)]
pub enum VaArgs<'l> {
    VaArgs(),
    FunctionCall(LuaExprRef<'l>, Box<LuaExprList<'l>>),
}
#[derive(Debug, Builder)]
#[builder(pattern = "owned")]
pub struct LuaExprList<'l> {
    #[builder(default)]
    exprs: Vec<LuaExprRef<'l>>,
    #[builder(default)]
    va_arg: Option<VaArgs<'l>>,
}

impl<'l> LuaExprList<'l> {
    pub fn new() -> Self {
        Self {
            exprs: vec![],
            va_arg: None,
        }
    }
}
impl<'l> From<LuaExprRef<'l>> for LuaExprList<'l> {
    fn from(i: LuaExprRef<'l>) -> Self {
        Self {
            exprs: vec![i],
            va_arg: None,
        }
    }
}
pub enum LuaTableKey<'l> {
    None,
    String(String),
    Expr(LuaExprRef<'l>),
}
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct LuaTableMetadata {}
#[derive(Clone)]
pub enum ScoptKind<'l> {
    Loop { break_block: LuaBlockRef<'l> },
    Other,
}
#[derive(Clone, Copy, Debug)]
pub enum ExprLifeTimeKind {
    Own,
    COW,
}
impl Default for ExprLifeTimeKind {
    fn default() -> Self { Self::Own }
}
#[derive(Default, Clone)]
pub struct VarAttribute {
    is_const: bool,
    is_close: bool,
}
impl VarAttribute {
    pub fn new(name: String) -> Fallible<Self> {
        Ok(match &*name {
            "const" => VarAttribute {
                is_const: true,
                is_close: false,
            },
            "close" => VarAttribute {
                is_const: false,
                is_close: true,
            },
            o => {
                return Err(LuaVMError::SyntaxError(format_err!(
                    "illegal attribute , except `const` or `close`, got {}",
                    o
                ))
                .into())
            }
        })
    }
}
pub type LuaScoptRef<'l> = Rc<GhostCell<'l, LuaScopt<'l>>>;
pub struct LuaScopt<'l> {
    variables: HashMap<String, LuaVariable<'l>>,
    kind: ScoptKind<'l>,
}
impl<'l> LuaScopt<'l> {
    pub fn new(kind: ScoptKind<'l>) -> Self {
        Self {
            variables: HashMap::new(),
            kind,
        }
    }
}
#[derive(Clone)]
pub struct LuaVariable<'l> {
    expr: LuaExprRef<'l>,
    attributes: VarAttribute,
    upvalue: Option<usize>,
}
pub type LuaBlockRef<'l> = Rc<GhostCell<'l, LuaBlock<'l>>>;
#[derive(Getters)]
pub struct LuaBlock<'l> {
    #[getset(get = "pub")]
    builder: BlockBuilder<'l, LuaInstructionSet>,
}
impl<'l> LuaBlock<'l> {
    pub fn new() -> Self {
        Self {
            builder: BlockBuilder::default(),
        }
    }
}
pub type LuaFunctionBuilderRef<'l> = Rc<GhostCell<'l, LuaFunctionBuilder<'l>>>;
pub enum LuaVar<'l> {
    Variable(String),
    Field(LuaExprRef<'l>, String),
    Element(LuaExprRef<'l>, LuaExprRef<'l>),
}
pub struct LuaFunctionBuilder<'l> {
    parameters: Vec<String>,
    va_param: bool,
    register_pool: Rc<RefCell<BuddyRegisterPool>>,
    scopts: Vec<LuaScoptRef<'l>>,
    current_scopt: LuaScoptRef<'l>,
    blocks: Vec<LuaBlockRef<'l>>,
    current_block: LuaBlockRef<'l>,
    goto_blocks: HashMap<String, Vec<LuaBlockRef<'l>>>,
    label_blocks: HashMap<String, LuaBlockRef<'l>>,
    child_closure_slot_map: HashMap<(String, usize), usize>,
    new_child_closure_slot_map: HashMap<String, (usize, usize)>,
    parent_closure_slot_map: HashMap<String, usize>,
    constants: ObjectBuilder<'l>,
}
impl<'l> LuaFunctionBuilder<'l> {
    pub fn new() -> Self {
        let new_scopt = LuaScopt::new(ScoptKind::Other);
        let new_block = LuaBlock::new();
        let new_scopt = Rc::new(GhostCell::new(new_scopt));
        let new_block = Rc::new(GhostCell::new(new_block));
        Self {
            parameters: Default::default(),
            va_param: false,
            register_pool: BuddyRegisterPool::reserve_range(0..LUA_PIN_REG_COUNT),
            current_scopt: new_scopt.clone(),
            scopts: vec![new_scopt],
            current_block: new_block.clone(),
            blocks: vec![new_block],
            goto_blocks: Default::default(),
            label_blocks: Default::default(),
            child_closure_slot_map: Default::default(),
            parent_closure_slot_map: Default::default(),
            new_child_closure_slot_map: Default::default(),
            constants: Default::default(),
        }
    }
    pub fn new_block(&mut self) -> LuaBlockRef<'l> {
        let new_block = LuaBlock::new();
        let new_block_wraped = Rc::new(GhostCell::new(new_block));
        self.blocks.push(new_block_wraped.clone());
        self.current_block = new_block_wraped.clone();
        new_block_wraped
    }
    pub fn new_scopt(&mut self, kind: ScoptKind<'l>) -> LuaScoptRef<'l> {
        let new_scopt = LuaScopt::new(kind);
        let new_scopt_wraped = Rc::new(GhostCell::new(new_scopt));
        self.scopts.push(new_scopt_wraped.clone());
        self.current_scopt = new_scopt_wraped.clone();
        new_scopt_wraped
    }
}
#[macro_export]
macro_rules! unique_operate_type {
    ($ty:ty $(, $ty1:ty)*) => {
        fn( &BlockBuilder<'l, LuaInstructionSet>, &mut GhostToken<'l>, &Register<$ty> $(,&Register<$ty1>)*) -> Fallible<()>
    };
}
#[macro_export]
macro_rules! binary_operate_type {
    ($ty:ty $(, $ty1:ty)*) => {
        fn( &BlockBuilder<'l, LuaInstructionSet>, &mut GhostToken<'l>, &Register<$ty>, &Register<$ty> $(,&Register<$ty1>)*) -> Fallible<()>
    };
}
pub const LUA_STATE_REG: Register<LuaStateReference> = Register::new_const(0);
pub const LUA_ARGS_REG: Register<Slice<LuaValue>> = Register::new_const(1);
pub const LUA_CLOSURE_REG: Register<LuaClosureReference> = Register::new_const(2);
pub const LUA_UP_VALUES_REG: Register<LuaUpValueReference> = Register::new_const(3);
pub const LUA_PIN_REG_COUNT: u16 = 4;

pub(crate) fn new_ctx<'l>(token: ghost_cell::GhostToken<'l>, lua_state: LuaStateReference) -> LuaContext<'l> {
    LuaContext::new(token, lua_state)
}
pub struct LuaContext<'l> {
    pub token: GhostToken<'l>,
    pub packs: Vec<FunctionPack<LuaInstructionSet>>,
    pub closure_stack: Vec<LuaFunctionBuilderRef<'l>>,
    pub current_function: LuaFunctionBuilderRef<'l>,
    pub current_scopt: LuaScoptRef<'l>,
    pub current_block: LuaBlockRef<'l>,
    pub current_builder: BlockBuilder<'l, LuaInstructionSet>,
    pub shape_map: HashMap<(Vec<String>, usize), LuaShapeReference>,
    pub lua_state: LuaStateReference,
}
impl<'l> LuaContext<'l> {
    pub fn new(token: GhostToken<'l>, lua_state: LuaStateReference) -> Self {
        let new_function = LuaFunctionBuilder::new();
        let new_scopt = new_function.current_scopt.clone();
        let new_block = new_function.current_block.clone();
        let new_builder = new_block.borrow(&token).builder().clone();
        let new_function = Rc::new(GhostCell::new(new_function));
        
        Self {
            token,
            closure_stack: vec![new_function.clone()],
            current_function: new_function,
            current_scopt: new_scopt,
            current_block: new_block,
            current_builder: new_builder,
            shape_map: Default::default(),
            packs: Default::default(),
            lua_state,
        }
    }
    pub fn builder(&self) -> &BlockBuilder<'l, LuaInstructionSet> { &self.current_builder }
    pub fn token(&self) -> &GhostToken<'l> { &self.token }
    pub fn token_mut(&mut self) -> &mut GhostToken<'l> { &mut self.token }
    pub fn new_scopt(&mut self, kind: ScoptKind<'l>) -> Fallible<LuaScoptRef<'l>> {
        trace!("new_scopt");
        let new_scopt = self.current_function_mut().new_scopt(kind);
        self.current_scopt = new_scopt;
        Ok(self.current_scopt.clone())
    }
    pub fn new_block(&mut self) -> &LuaBlockRef<'l> {
        let new_block = self.current_function_mut().new_block();
        self.current_builder = new_block.borrow(self.token()).builder.clone();
        self.current_block = new_block;
        &self.current_block
    }
    pub fn split_block(&mut self) -> Fallible<(LuaBlockRef<'l>, LuaBlockRef<'l>)> {
        Ok((self.current_block.clone(), self.new_block().clone()))
    }
    pub fn current_function(&self) -> &LuaFunctionBuilder<'l> { self.current_function.borrow(self.token()) }
    pub fn current_function_mut(&mut self) -> &mut LuaFunctionBuilder<'l> {
        self.current_function.borrow_mut(&mut self.token)
    }
    pub fn current_block(&self) -> &LuaBlock<'l> { self.current_block.borrow(self.token()) }
    pub fn current_block_mut(&mut self) -> &mut LuaBlock<'l> { self.current_block.borrow_mut(&mut self.token) }
    pub fn current_scopt(&self) -> &LuaScopt<'l> { self.current_scopt.borrow(self.token()) }
    pub fn current_scopt_mut(&mut self) -> &mut LuaScopt<'l> { self.current_scopt.borrow_mut(&mut self.token) }
    pub fn get_value(&mut self, name: String) -> Fallible<LuaExprRef<'l>> {
        for (closure_index, function) in self.closure_stack.iter().rev().enumerate() {
            for (scopt_index, scopt) in function.borrow(&self.token).scopts.clone().iter().enumerate().rev() {
                if let Some(variable) = scopt.borrow(&self.token).variables.get(&name).cloned() {
                    let expr = if closure_index == 0 {
                        trace!("get local value {:?}", &variable.expr);
                        Rc::new(
                            LuaExprBuilder::default()
                                .register(variable.expr.register.clone())
                                .lifetime(ExprLifeTimeKind::COW)
                                .build()?,
                        )
                    } else {
                        let child_closure_slot_map = &mut function.borrow_mut(&mut self.token).child_closure_slot_map;
                        let slot = child_closure_slot_map.len();
                        child_closure_slot_map.insert((name.clone(), scopt_index), slot);
                        function
                            .borrow_mut(&mut self.token)
                            .new_child_closure_slot_map
                            .insert(name, (slot, scopt_index));
                        let reg = self.alloc_register()?;
                        GetUpVariable::emit(
                            &self.current_builder,
                            &mut self.token,
                            Usize(closure_index - 1),
                            Usize(slot),
                            &LUA_CLOSURE_REG,
                            &reg,
                        )?;
                        trace!("get up value ({}<-({},{}))", reg.reg(), closure_index - 1, slot);
                        LuaExpr::new_value(reg)
                    };
                    return Ok(Rc::new(
                        LuaExprBuilder::default()
                            .register(expr.register.clone())
                            .lifetime(ExprLifeTimeKind::COW)
                            .build()?,
                    ));
                }
            }
        }
        trace!("get global value {}", &name);
        let reg = self.alloc_register()?;
        let name = self.const_string_value(name)?;
        let cache = self.empty_inline_cache_line()?;
        GetGlobal::emit(
            &self.current_builder,
            &mut self.token,
            name,
            cache,
            &LUA_STATE_REG,
            &reg,
        )?;
        Ok(Rc::new(
            LuaExprBuilder::default()
                .register(LuaRegister::Value(reg, None))
                .lifetime(ExprLifeTimeKind::COW)
                .build()?,
        ))
    }
    pub fn put_value(&mut self, name: String, value: LuaExprRef<'l>) -> Fallible<()> {
        for (closure_index, function) in self.closure_stack.clone().iter().rev().enumerate() {
            for (scopt_index, scopt) in function.borrow(self.token()).scopts.clone().iter().enumerate().rev() {
                if let Some(variable) = scopt.borrow(self.token()).variables.get(&name).cloned() {
                    if closure_index == 0 {
                        let operate_kind = Self::trans_binary_type(true, true, &value, &variable.expr);
                        let from = Self::transform_expr(self, value.clone(), operate_kind)?;
                        let to = Self::transform_expr(self, variable.expr.clone(), operate_kind)?;
                        if to.register.reg_index() != variable.expr.register.reg_index() {
                            scopt
                                .borrow_mut(self.token_mut())
                                .variables
                                .get_mut(&name)
                                .unwrap()
                                .expr = to.clone();
                        }
                        trace!(
                            "put local value {:?}=>{:?}<---{:?}<={:?}",
                            &variable.expr,
                            &to,
                            &from,
                            &value
                        );
                        if from.register.reg_index() != to.register.reg_index() {
                            match (&from.register, &to.register) {
                                (LuaRegister::Integer(r1), LuaRegister::Integer(r2)) => {
                                    MoveI64::emit(&self.current_builder, &mut self.token, r1, r2)?;
                                }
                                (LuaRegister::Float(r1), LuaRegister::Float(r2)) => {
                                    MoveF64::emit(&self.current_builder, &mut self.token, r1, r2)?;
                                }
                                (LuaRegister::Value(r1, _), LuaRegister::Value(r2, _)) => {
                                    MoveValue::emit(&self.current_builder, &mut self.token, r1, r2)?;
                                }
                                _ => unreachable!(),
                            }
                        }
                    } else {
                        let value = self.to_value(value)?;
                        let child_closure_slot_map = &mut function.borrow_mut(&mut self.token).child_closure_slot_map;
                        let slot = child_closure_slot_map.len();
                        child_closure_slot_map.insert((name.clone(), scopt_index), slot);
                        function
                            .borrow_mut(&mut self.token)
                            .new_child_closure_slot_map
                            .insert(name, (slot, scopt_index));
                        trace!("put up value ({},{})<-{:?}", closure_index - 1, slot, &value);
                        SetUpVariable::emit(
                            &self.current_builder,
                            &mut self.token,
                            Usize(closure_index - 1),
                            Usize(slot),
                            &LUA_CLOSURE_REG,
                            value.value_reg(),
                        )?;
                    }
                    return Ok(());
                }
            }
        }
        let value = self.to_value(value)?;
        match &value.register {
            LuaRegister::Value(r, _) => {
                trace!("put global value {:?}<-{:?}", &name, &value);
                let name = self.const_string_value(name)?;
                let cache = self.empty_inline_cache_line()?;
                SetGlobal::emit(&self.current_builder, &mut self.token, name, cache, &LUA_STATE_REG, r)?;
            }
            _ => unreachable!(),
        }
        Ok(())
    }
    pub fn insert_break_point(&mut self, block: &LuaBlockRef<'l>) -> Fallible<()> {
        let builder = &block.borrow(self.token()).clone().builder().clone();
        BreakPoint::emit(builder, &mut self.token)?;
        Ok(())
    }

    pub fn branch(&mut self, from: &LuaBlockRef<'l>, to: &LuaBlockRef<'l>) -> Fallible<()> {
        let from = &from.borrow(self.token()).clone().builder().clone();
        let to = &to.borrow(self.token()).clone().builder().clone();
        debug!("branch {:?}->{:?}", &from, &to);
        ir::Goto::emit(from, &mut self.token, to)?;
        Ok(())
    }
    pub fn branch_if(
        &mut self,
        predicate: LuaExprRef<'l>,
        from: &LuaBlockRef<'l>,
        t: &LuaBlockRef<'l>,
        f: &LuaBlockRef<'l>,
    ) -> Fallible<()> {
        let from = &from.borrow(self.token()).clone().builder().clone();
        let t = &t.borrow(self.token()).clone().builder().clone();
        let f = &f.borrow(self.token()).clone().builder().clone();
        debug!("if branch {:?}?{:?}:{:?}", &predicate, &t, &f);
        match &predicate.register {
            LuaRegister::Integer(_r) => ir::Goto::emit(from, &mut self.token, t)?,
            LuaRegister::Float(_r) => ir::Goto::emit(from, &mut self.token, t)?,
            LuaRegister::Function(r, _) | LuaRegister::Value(r, _) => BranchIf::emit(from, &mut self.token, t, f, r)?,
        };
        Ok(())
    }
    pub fn const_number(&mut self, number: LuaNumberLit) -> Fallible<LuaExprRef<'l>> {
        match number {
            LuaNumberLit::Integer(int) => {
                let reg = self.alloc_register()?;
                match int {
                    -1 => ConstM1::emit(&self.current_builder, &mut self.token, &reg),
                    0 => ConstZero::emit(&self.current_builder, &mut self.token, &reg),
                    1 => ConstOne::emit(&self.current_builder, &mut self.token, &reg),
                    _o => ConstI64::emit(&self.current_builder, &mut self.token, I64(int), &reg),
                }?;
                Ok(Rc::new(
                    LuaExprBuilder::default().register(LuaRegister::Integer(reg)).build()?,
                ))
            }
            LuaNumberLit::Float(float) => {
                let reg = self.alloc_register()?;
                ConstF64::emit(&self.current_builder, &mut self.token, F64(float), &reg)?;
                Ok(Rc::new(
                    LuaExprBuilder::default().register(LuaRegister::Float(reg)).build()?,
                ))
            }
        }
    }
    pub fn const_string_value(&mut self, s: String) -> Fallible<LuaValueImpl> {
        crate::new_string(self.lua_state.as_pointer(), s.as_bytes())
    }
    pub fn const_string(&mut self, string: String) -> Fallible<LuaExprRef<'l>> {
        let reg = self.alloc_register()?;
        let const_string = self.const_string_value(string)?;
        ConstValue::emit(&self.current_builder, &mut self.token, const_string, &reg)?;
        Ok(LuaExpr::new_value(reg))
    }
    pub fn empty_inline_cache_line(&mut self) -> Fallible<InlineCacheLineImpl> {
        unsafe {
            let mut i: MaybeUninit<InlineCacheLineImpl> = MaybeUninit::zeroed();
            let r = i.assume_init_mut();
            r.set_shape(NullableOptionImpl::encode_none(()));
            r.set_key(LuaValueImpl::encode_nil(()));
            r.set_invalid(NullableOptionImpl::encode_none(()));
            r.set_table(NullableOptionImpl::encode_none(()));
            r.set_slot(e::U32(u32::MAX));
            Ok(i.assume_init())
        }
    }
    pub fn const_va_arg0(&mut self) -> Fallible<LuaExprRef<'l>> {
        let reg = self.alloc_register()?;
        let va_args = self.va_args()?;
        GetArg::emit(&self.current_builder, &mut self.token, Usize(0), &va_args, &reg)?;
        Ok(LuaExpr::new_value(reg))
    }
    pub fn const_table(&mut self, table_decl: Vec<(LuaTableKey<'l>, LuaExprRef<'l>)>) -> Fallible<LuaExprRef<'l>> {
        let reg = self.alloc_register()?;
        let mut string_key_values = Vec::new();
        let mut int_key_values = Vec::new();
        let mut expr_key_values = Vec::new();
        for (key, value) in table_decl {
            match key {
                LuaTableKey::None => int_key_values.push(value),
                LuaTableKey::String(s) => string_key_values.push((s, value)),
                LuaTableKey::Expr(e) => expr_key_values.push((e, value)),
            }
        }
        string_key_values.sort_by(|(k0, _), (k1, _)| k0.cmp(k1));
        if int_key_values.is_empty() && string_key_values.is_empty() {
            MakeTable0::emit(&self.current_builder, &mut self.token, &LUA_STATE_REG, &reg)?;
        } else {
            let shape = if int_key_values.is_empty() {
                let keys = (
                    string_key_values.iter().map(|(k, _v)| k.clone()).collect(),
                    int_key_values.len(),
                );
                if let Some(shape) = self.shape_map.get(&keys) {
                    shape.clone()
                } else {
                    let shape = crate::new_shape(crate::new_meta_functions()?, false)?;
                    unsafe {
                        let hash_map = shape.as_pointer().as_ref().ref_fields().get().as_mut().unwrap();
                        for (key, _expr) in string_key_values.iter() {
                            let key = self.const_string_value(key.clone())?;
                            let len = hash_map.len();
                            let mut slot_metadata = MaybeUninit::<LuaSlotMetadataImpl>::zeroed();
                            let slot_metadata_ref = slot_metadata.assume_init_mut();
                            slot_metadata_ref.set_slot(Usize(len));
                            hash_map.insert(key, slot_metadata.assume_init_read());
                        }
                    }
                    self.shape_map.insert(keys, shape.clone());
                    shape
                }
            } else {
                let shape = crate::new_shape(crate::new_meta_functions()?, true)?;
                unsafe {
                    let hash_map = shape.as_pointer().as_ref().ref_fields().get().as_mut().unwrap();
                    for (key, _expr) in string_key_values.iter() {
                        let key = self.const_string_value(key.to_owned())?;
                        let len = hash_map.len();
                        let mut slot_metadata = MaybeUninit::<LuaSlotMetadataImpl>::zeroed();
                        let slot_metadata_ref = slot_metadata.assume_init_mut();
                        slot_metadata_ref.set_slot(Usize(len));
                        hash_map.insert(key, slot_metadata.assume_init_read());
                    }
                    for (key, _expr) in int_key_values.iter().enumerate() {
                        assert!(key + 1 < 1 << (usize::BITS - 4));
                        let key = LuaValueImpl::encode_integer(I64((1 + (key as i64)) << 4));
                        let len = hash_map.len();
                        let mut slot_metadata = MaybeUninit::<LuaSlotMetadataImpl>::zeroed();
                        let slot_metadata_ref = slot_metadata.assume_init_mut();
                        slot_metadata_ref.set_slot(Usize(len));
                        hash_map.insert(key, slot_metadata.assume_init_read());
                    }
                }
                shape
            };
            let fast_len = string_key_values.len() + int_key_values.len();
            let base_len = LuaTableImpl::LAYOUT.size();
            let var_len = LuaTableImpl::LAYOUT.flexible_size();
            let table_mem_size = base_len + fast_len * var_len;
            let table_mem_size = 1 << (usize::BITS - table_mem_size.leading_zeros());
            let fast_len = (table_mem_size - base_len) / var_len;
            let mut fields = Vec::new();
            for (_key, value) in string_key_values {
                fields.push(self.to_value(value)?.value_reg().clone());
            }
            for (_key, value) in int_key_values.into_iter().enumerate() {
                fields.push(self.to_value(value)?.value_reg().clone());
            }
            let array_reg = self.alloc_array::<LuaValue>(fields.len())?;
            let fields_reg = self.alloc_register()?;
            MakeSlice::emit(&self.current_builder, &mut self.token, &fields, &fields_reg, array_reg)?;
            self.free_array::<LuaValue>(fields.len(), array_reg);
            MakeTable::emit(
                &self.current_builder,
                &mut self.token,
                shape.as_pointer(),
                Usize(fast_len),
                &fields_reg,
                &reg,
            )?;
        }
        for (key, value) in expr_key_values {
            let key = self.to_value(key)?;
            let value = self.to_value(value)?;
            let cache = self.empty_inline_cache_line()?;
            SetElement::emit(
                &self.current_builder,
                &mut self.token,
                cache,
                &reg,
                key.value_reg(),
                value.value_reg(),
            )?;
        }
        Ok(LuaExpr::new_value(reg))
    }
    pub fn emit_return(&mut self, exprs: Option<LuaExprList<'l>>) -> Fallible<()> {
        debug!("emit_return:{:?}", &exprs);
        if let Some(exprs) = exprs {
            let LuaExprList { exprs, va_arg } = exprs;
            let exprs: Vec<_> = exprs.into_iter().map(|arg| self.to_value(arg)).try_collect()?;
            match va_arg {
                Some(VaArgs::VaArgs()) => {
                    let va_args = self.va_args()?;
                    match exprs.len() {
                        0 => Return0VaSlice::emit(&self.current_builder, &mut self.token, &va_args)?,
                        1 => Return1VaSlice::emit(
                            &self.current_builder,
                            &mut self.token,
                            exprs[0].value_reg(),
                            &va_args,
                        )?,
                        2 => Return2VaSlice::emit(
                            &self.current_builder,
                            &mut self.token,
                            exprs[0].value_reg(),
                            exprs[1].value_reg(),
                            &va_args,
                        )?,
                        3 => Return3VaSlice::emit(
                            &self.current_builder,
                            &mut self.token,
                            exprs[0].value_reg(),
                            exprs[1].value_reg(),
                            exprs[2].value_reg(),
                            &va_args,
                        )?,
                        _len => {
                            let args = self.alloc_register()?;
                            let array_reg = self.alloc_array::<LuaValue>(exprs.len())?;
                            MakeSlice::emit(
                                &self.current_builder,
                                &mut self.token,
                                &exprs.iter().map(|e| e.value_reg().clone()).collect::<Vec<_>>(),
                                &args,
                                array_reg,
                            )?;
                            self.free_array::<LuaValue>(exprs.len(), array_reg);
                            ReturnVaSlice::emit(&self.current_builder, &mut self.token, &args, &va_args)?;
                        }
                    };
                }
                Some(VaArgs::FunctionCall(function, args)) => {
                    let va_args = self.emit_call(function, *args)?;
                    match exprs.len() {
                        0 => Return0VA::emit(&self.current_builder, &mut self.token, &va_args)?,
                        1 => Return1VA::emit(&self.current_builder, &mut self.token, exprs[0].value_reg(), &va_args)?,
                        2 => Return2VA::emit(
                            &self.current_builder,
                            &mut self.token,
                            exprs[0].value_reg(),
                            exprs[1].value_reg(),
                            &va_args,
                        )?,
                        3 => Return3VA::emit(
                            &self.current_builder,
                            &mut self.token,
                            exprs[0].value_reg(),
                            exprs[1].value_reg(),
                            exprs[2].value_reg(),
                            &va_args,
                        )?,
                        _len => {
                            let args = self.alloc_register()?;
                            let array_reg = self.alloc_array::<LuaValue>(exprs.len())?;
                            MakeSlice::emit(
                                &self.current_builder,
                                &mut self.token,
                                &exprs.iter().map(|e| e.value_reg().clone()).collect::<Vec<_>>(),
                                &args,
                                array_reg,
                            )?;
                            self.free_array::<LuaValue>(exprs.len(), array_reg);
                            ReturnVA::emit(&self.current_builder, &mut self.token, &args, &va_args)?;
                        }
                    };
                }
                None => match exprs.len() {
                    0 => Return0::emit(&self.current_builder, &mut self.token)?,
                    1 => Return1::emit(&self.current_builder, &mut self.token, exprs[0].value_reg())?,
                    2 => Return2::emit(
                        &self.current_builder,
                        &mut self.token,
                        exprs[0].value_reg(),
                        exprs[1].value_reg(),
                    )?,
                    3 => Return3::emit(
                        &self.current_builder,
                        &mut self.token,
                        exprs[0].value_reg(),
                        exprs[1].value_reg(),
                        exprs[2].value_reg(),
                    )?,
                    _len => {
                        let args = self.alloc_register()?;
                        let array_reg = self.alloc_array::<LuaValue>(exprs.len())?;
                        MakeSlice::emit(
                            &self.current_builder,
                            &mut self.token,
                            &exprs.iter().map(|e| e.value_reg().clone()).collect::<Vec<_>>(),
                            &args,
                            array_reg,
                        )?;
                        self.free_array::<LuaValue>(exprs.len(), array_reg);
                        Return0VaSlice::emit(&self.current_builder, &mut self.token, &args)?;
                    }
                },
            }
        } else {
            Return0::emit(&self.current_builder, &mut self.token)?;
        }
        Ok(())
    }
    pub fn return_(
        &mut self,
        exprs: Option<LuaExprList<'l>>,
        (pre_block, post_block): (LuaBlockRef<'l>, LuaBlockRef<'l>),
    ) -> Fallible<(LuaBlockRef<'l>, LuaBlockRef<'l>)> {
        self.emit_return(exprs)?;
        Ok((pre_block, post_block))
    }
    pub fn new_function(&mut self, parameters: Vec<String>, va_param: bool) -> Fallible<LuaFunctionBuilderRef<'l>> {
        let new_function = LuaFunctionBuilder::new();
        let new_scopt = new_function.current_scopt.clone();
        let new_block = new_function.current_block.clone();
        let new_function = Rc::new(GhostCell::new(new_function));
        self.closure_stack.push(new_function.clone());
        self.current_function = new_function.clone();
        self.current_scopt = new_scopt.clone();
        self.current_builder = new_block.borrow(self.token()).builder.clone();
        self.current_block = new_block.clone();
        {
            let new_function = new_function.borrow_mut(self.token_mut());
            new_function.parameters = parameters.clone();
            new_function.va_param = va_param;
            new_function.register_pool = BuddyRegisterPool::reserve_range(
                0..(LUA_PIN_REG_COUNT as usize + new_function.parameters.len()).try_into()?,
            );
        }
        let va_args_reg = Register::new_const((LUA_PIN_REG_COUNT as usize + parameters.len()).try_into()?);
        GetVaArgs::emit(
            &self.current_builder,
            &mut self.token,
            Usize(parameters.len().try_into()?),
            &LUA_ARGS_REG,
            &va_args_reg,
        )?;
        for (index, param) in parameters.into_iter().enumerate() {
            let reg_index = (index + LUA_PIN_REG_COUNT as usize).try_into()?;
            let reg = Register::new_const(reg_index);
            GetArg::emit(
                &self.current_builder,
                &mut self.token,
                Usize(reg_index as usize),
                &LUA_ARGS_REG,
                &reg,
            )?;
            new_scopt
                .borrow_mut(self.token_mut())
                .variables
                .insert(param, LuaVariable {
                    expr: Rc::new(
                        LuaExprBuilder::default()
                            .register(LuaRegister::Value(reg, None))
                            .lifetime(ExprLifeTimeKind::COW)
                            .build()?,
                    ),
                    attributes: Default::default(),
                    upvalue: None,
                });
        }
        Ok(self.current_function.clone())
    }
    pub fn finish_function(
        &mut self,
        (finish_block, _last_block): (LuaBlockRef<'l>, LuaBlockRef<'l>),
    ) -> Fallible<LuaFunctionBuilderRef<'l>> {
        debug!("finish_function");
        let function = self.closure_stack.pop().unwrap();
        function.borrow_mut(self.token_mut()).blocks.pop().unwrap();
        let builder = finish_block.borrow(self.token()).builder().clone();
        Return0::emit(&builder, &mut self.token)?;
        self.current_function = self.closure_stack.last().unwrap().clone();
        self.current_scopt = self.current_function().current_scopt.clone();
        self.current_block = self.current_function().current_block.clone();
        self.current_builder = self.current_block.borrow(self.token()).builder().clone();
        Ok(function)
    }
    pub fn finish_scopt(
        &mut self,
        block_split: (LuaBlockRef<'l>, LuaBlockRef<'l>),
    ) -> Fallible<(LuaBlockRef<'l>, LuaBlockRef<'l>)> {
        debug!("finish_scopt");
        let current_function = self.current_function_mut();
        let scopt = current_function.scopts.pop().unwrap();
        let scopt_index = current_function.scopts.len();
        let current_scopt = current_function.scopts.last().unwrap().clone();
        current_function.current_scopt = current_scopt.clone();
        for ((name, _scopt_index), slot) in current_function
            .child_closure_slot_map
            .drain_filter(|(_name, var_scopt_index), _| *var_scopt_index == scopt_index)
            .collect::<HashMap<_, _>>()
        {
            let mut expr = scopt
                .borrow_mut(self.token_mut())
                .variables
                .get_mut(&name)
                .unwrap()
                .expr
                .clone();
            if !matches!(&expr.register, LuaRegister::Value(_, _)) {
                expr = self.to_value(expr)?;
            }
            SetUpValue::emit(
                &self.current_builder,
                &mut self.token,
                Usize(slot),
                &LUA_UP_VALUES_REG,
                expr.value_reg(),
            )?;
        }
        self.current_scopt = current_scopt;
        Ok(block_split)
    }
    pub fn pack(mut self) -> Fallible<Vec<FunctionPack<LuaInstructionSet>>> {
        let mut function_builder = FunctionBuilder::new();
        match self.current_function().child_closure_slot_map.len() {
            0 => {}
            o => {
                let first_block = self.current_function().blocks[0].clone();
                let entry_block = BlockBuilder::default();
                NewUpValue::emit(&entry_block, &mut self.token, Usize(o), &LUA_UP_VALUES_REG)?;
                let first_block_builder = first_block.borrow(self.token()).builder().clone();
                Goto::emit(&entry_block, &mut self.token, &first_block_builder)?;
                function_builder.add_block(first_block_builder);
            }
        }
        for block in self.current_function().blocks.clone().iter() {
            debug!(
                "function_builder.add_block:{:?}",
                block.borrow(&self.token).builder.borrow(self.token())
            );
            assert_ne!(block.borrow(&self.token).builder.borrow(self.token()).len(), 0);
            function_builder.add_block(block.borrow(&mut self.token).builder.clone());
        }
        let reg_count = self.current_function().register_pool.borrow().max_allocated();
        let pack = function_builder.pack(
            &mut self.token,
            FunctionTypeBuilder::default()
                .args(vec![LuaStateReference::TYPE].into())
                .return_type(Some(Pointer::<UnsizedArray<LuaValue>>::TYPE))
                .va_arg(Some(LuaValue::TYPE))
                .build()
                .unwrap(),
            reg_count,
        )?;
        let mut packs = self.packs;
        packs.push(pack);
        Ok(packs)
    }
    pub fn const_function(&mut self, function: LuaFunctionBuilderRef<'l>) -> Fallible<LuaExprRef<'l>> {
        let mut function_builder = FunctionBuilder::new();
        match function.borrow(self.token()).child_closure_slot_map.len() {
            0 => {}
            o => {
                let first_block = function.borrow(self.token_mut()).blocks[0].clone();
                let entry_block = BlockBuilder::default();
                NewUpValue::emit(&entry_block, &mut self.token, Usize(o), &LUA_UP_VALUES_REG)?;
                let first_block_builder = first_block.borrow(self.token()).builder().clone();
                Goto::emit(&entry_block, &mut self.token, &first_block_builder)?;
                function_builder.add_block(first_block_builder);
            }
        }
        for block in function.borrow(self.token_mut()).blocks.clone().iter() {
            debug!(
                "function_builder.add_block:{:?}",
                block.borrow(&mut self.token).builder.codes()
            );
            function_builder.add_block(block.borrow(&mut self.token).builder.clone());
        }
        let reg_count = self.current_function().register_pool.borrow().max_allocated();
        let obj_builder = ObjectBuilder::default();
        obj_builder.borrow_mut(self.token_mut()).receive::<usize>();
        obj_builder
            .borrow_mut(self.token_mut())
            .add_symbol(SymbolBuilder::default().offset(0).build().unwrap());
        let obj = obj_builder.take(self.token_mut()).build()?;
        let pack = function_builder.pack_into(
            &mut self.token,
            FunctionTypeBuilder::default()
                .args(vec![LuaStateReference::TYPE, LuaClosureReference::TYPE].into())
                .return_type(Some(Pointer::<UnsizedArray<LuaValue>>::TYPE))
                .va_arg(Some(LuaValue::TYPE))
                .build()
                .unwrap(),
            reg_count,
            obj.clone(),
        )?;
        self.packs.push(pack);
        for (new_closure_variable, (slot, scopt_index)) in
            std::mem::take(&mut self.current_function_mut().new_child_closure_slot_map)
        {
            let scopt = self.current_function().scopts[scopt_index].clone();
            let mut expr = scopt
                .borrow_mut(self.token_mut())
                .variables
                .get_mut(&new_closure_variable)
                .unwrap()
                .expr
                .clone();
            if !matches!(&expr.register, LuaRegister::Value(_, _)) {
                expr = self.to_value(expr)?;
                scopt
                    .borrow_mut(self.token_mut())
                    .variables
                    .get_mut(&new_closure_variable)
                    .unwrap()
                    .expr = expr.clone();
            }
            SetUpRef::emit(
                &self.current_builder,
                &mut self.token,
                Usize(slot),
                &LUA_UP_VALUES_REG,
                expr.value_reg(),
            )?;
        }
        let value_reg = self.alloc_register()?;
        match self.current_function().parent_closure_slot_map.len() {
            0 => {
                ConstClosure0::emit(
                    &self.current_builder,
                    &mut self.token,
                    SymbolRef::new(obj, 0),
                    &LUA_STATE_REG,
                    &LUA_UP_VALUES_REG,
                    &value_reg,
                )?;
            }
            _ => {
                ConstClosure::emit(
                    &self.current_builder,
                    &mut self.token,
                    SymbolRef::new(obj, 0),
                    &LUA_STATE_REG,
                    &LUA_UP_VALUES_REG,
                    &LUA_CLOSURE_REG,
                    &value_reg,
                )?;
            }
        }

        Ok(Rc::new(
            LuaExprBuilder::default()
                .register(LuaRegister::Value(value_reg, None))
                .build()?,
        ))
    }
    // [var_list(v),t!(=),expr_list(exprs)]=>cxt.put_values(v,exprs);
    pub fn put_values(&mut self, v: Vec<LuaVar<'l>>, exprs: LuaExprList<'l>) -> Fallible<()> {
        let len = v.len();
        for (var, e) in v.into_iter().zip(self.expr_list_to_vec(exprs, len)?.into_iter()) {
            let value = self.to_value(e.clone())?;
            match var {
                LuaVar::Variable(var) => {
                    self.put_value(var, value.clone())?;
                }
                LuaVar::Field(t, name) => {
                    let table = self.to_value(t.clone())?;
                    let name = self.const_string_value(name)?;
                    let cache = self.empty_inline_cache_line()?;
                    SetField::emit(
                        &self.current_builder,
                        &mut self.token,
                        name,
                        cache,
                        e::U8(0),
                        table.value_reg(),
                        value.value_reg(),
                    )?;
                }
                LuaVar::Element(t, k) => {
                    let table = self.to_value(t.clone())?;
                    let key = self.to_value(k.clone())?;
                    let cache = self.empty_inline_cache_line()?;
                    SetElement::emit(
                        &self.current_builder,
                        &mut self.token,
                        cache,
                        table.value_reg(),
                        key.value_reg(),
                        value.value_reg(),
                    )?;
                }
            }
        }
        Ok(())
    }
    pub fn extend_expr_list(&mut self, exprs: LuaExprList<'l>, expr: LuaExprRef<'l>) -> Fallible<LuaExprList<'l>> {
        let mut exprs = self.va_arg_to_arg(exprs)?;
        exprs.exprs.push(expr);
        Ok(exprs)
    }
    pub fn va_arg_to_arg(&mut self, mut exprs: LuaExprList<'l>) -> Fallible<LuaExprList<'l>> {
        if let Some(va_arg) = exprs.va_arg.take() {
            match va_arg {
                VaArgs::VaArgs() => unreachable!(),
                call => {
                    let ret0 = self.get_from_slice(0, LuaExprList {
                        exprs: vec![],
                        va_arg: Some(call),
                    })?;
                    exprs.exprs.push(ret0);
                }
            }
        }
        Ok(exprs)
    }
    pub fn expr_list_to_vec(&mut self, expr_list: LuaExprList<'l>, len: usize) -> Fallible<Vec<LuaExprRef<'l>>> {
        let mut list = Vec::with_capacity(len);
        let LuaExprList { exprs, va_arg } = expr_list;
        list.extend(exprs);
        match (len.saturating_sub(list.len()), va_arg) {
            (o, None) => {
                for _i in 0..o {
                    let reg = self.alloc_register()?;
                    ConstNil::emit(&self.current_builder, &mut self.token, &reg)?;
                    list.push(Rc::new(
                        LuaExprBuilder::default()
                            .register(LuaRegister::Value(reg, None))
                            .build()?,
                    ));
                }
            }
            (o, Some(VaArgs::VaArgs())) => {
                let va_args = self.va_args()?;
                for i in 0..o {
                    let reg = self.alloc_register()?;
                    GetArg::emit(&self.current_builder, &mut self.token, Usize(i), &va_args, &reg)?;
                    list.push(Rc::new(
                        LuaExprBuilder::default()
                            .register(LuaRegister::Value(reg, None))
                            .build()?,
                    ));
                }
            }
            (1, Some(VaArgs::FunctionCall(function, args))) => {
                let reg = self.alloc_register()?;
                self.emit_call_ret1(function, *args, &reg)?;
                list.push(Rc::new(
                    LuaExprBuilder::default()
                        .register(LuaRegister::Value(reg, None))
                        .build()?,
                ));
            }
            (o, Some(VaArgs::FunctionCall(function, args))) => {
                let rets = self.emit_call(function, *args)?;
                for i in 0..o {
                    let reg = self.alloc_register()?;
                    GetRet::emit(&self.current_builder, &mut self.token, Usize(i), &rets, &reg)?;
                    list.push(Rc::new(
                        LuaExprBuilder::default()
                            .register(LuaRegister::Value(reg, None))
                            .build()?,
                    ));
                }
            }
        }
        if len > list.len() {
            list.split_off(len);
        }
        Ok(list)
    }
    pub fn va_args(&mut self) -> Fallible<Register<Slice<LuaValue>>> {
        if !self.current_function().va_param {
            return Err(format_err!("cannot use '...' outside a vararg function near '...'"));
        }
        let reg = LUA_PIN_REG_COUNT as usize + self.current_function().parameters.len();
        Ok(Register::new_const(reg.try_into()?))
    }
    pub fn emit_call(
        &mut self,
        function: LuaExprRef<'l>,
        args: LuaExprList<'l>,
    ) -> Fallible<Register<Pointer<UnsizedArray<LuaValue>>>> {
        let LuaExprList { exprs, va_arg } = args;
        let exprs: Vec<_> = exprs.into_iter().map(|arg| self.to_value(arg)).try_collect()?;
        let reg = self.alloc_register()?;
        match (exprs.len(), va_arg) {
            (0, None) => CallFunction0::emit(&self.current_builder, &mut self.token, function.value_reg(), &reg)?,
            (1, None) => CallFunction1::emit(
                &self.current_builder,
                &mut self.token,
                function.value_reg(),
                exprs[0].value_reg(),
                &reg,
            )?,
            (2, None) => CallFunction2::emit(
                &self.current_builder,
                &mut self.token,
                function.value_reg(),
                exprs[0].value_reg(),
                exprs[1].value_reg(),
                &reg,
            )?,
            (3, None) => CallFunction3::emit(
                &self.current_builder,
                &mut self.token,
                function.value_reg(),
                exprs[0].value_reg(),
                exprs[1].value_reg(),
                exprs[2].value_reg(),
                &reg,
            )?,
            (_len, None) => {
                let args = self.alloc_register()?;
                let array_reg = self.alloc_array::<LuaValue>(exprs.len())?;
                MakeSlice::emit(
                    &self.current_builder,
                    &mut self.token,
                    &exprs.iter().map(|e| e.value_reg().clone()).collect::<Vec<_>>(),
                    &args,
                    array_reg,
                )?;
                self.free_array::<LuaValue>(exprs.len(), array_reg);
                CallFunction::emit(
                    &self.current_builder,
                    &mut self.token,
                    function.value_reg(),
                    &args,
                    &reg,
                )?;
            }
            (_, Some(VaArgs::VaArgs())) => {
                let va_args = self.va_args()?;
                match exprs.len() {
                    0 => CallFunction0VaSlice::emit(
                        &self.current_builder,
                        &mut self.token,
                        function.value_reg(),
                        &va_args,
                        &reg,
                    )?,
                    1 => CallFunction1VaSlice::emit(
                        &self.current_builder,
                        &mut self.token,
                        function.value_reg(),
                        exprs[0].value_reg(),
                        &va_args,
                        &reg,
                    )?,

                    2 => CallFunction2VaSlice::emit(
                        &self.current_builder,
                        &mut self.token,
                        function.value_reg(),
                        exprs[0].value_reg(),
                        exprs[1].value_reg(),
                        &va_args,
                        &reg,
                    )?,

                    3 => CallFunction3VaSlice::emit(
                        &self.current_builder,
                        &mut self.token,
                        function.value_reg(),
                        exprs[0].value_reg(),
                        exprs[1].value_reg(),
                        exprs[2].value_reg(),
                        &va_args,
                        &reg,
                    )?,
                    _len => {
                        let args = self.alloc_register()?;
                        let array_reg = self.alloc_array::<LuaValue>(exprs.len())?;
                        MakeSlice::emit(
                            &self.current_builder,
                            &mut self.token,
                            &exprs.iter().map(|e| e.value_reg().clone()).collect::<Vec<_>>(),
                            &args,
                            array_reg,
                        )?;
                        self.free_array::<LuaValue>(exprs.len(), array_reg);
                        let va_args = self.va_args()?;
                        CallFunctionVaSlice::emit(
                            &self.current_builder,
                            &mut self.token,
                            function.value_reg(),
                            &args,
                            &va_args,
                            &reg,
                        )?;
                    }
                }
            }
            (_, Some(VaArgs::FunctionCall(last_function, args))) => {
                let va_args = self.emit_call(last_function, *args)?;
                match exprs.len() {
                    0 => CallFunction0VA::emit(
                        &self.current_builder,
                        &mut self.token,
                        function.value_reg(),
                        &va_args,
                        &reg,
                    )?,
                    1 => CallFunction1VA::emit(
                        &self.current_builder,
                        &mut self.token,
                        function.value_reg(),
                        exprs[0].value_reg(),
                        &va_args,
                        &reg,
                    )?,
                    2 => CallFunction2VA::emit(
                        &self.current_builder,
                        &mut self.token,
                        function.value_reg(),
                        exprs[0].value_reg(),
                        exprs[1].value_reg(),
                        &va_args,
                        &reg,
                    )?,
                    3 => CallFunction3VA::emit(
                        &self.current_builder,
                        &mut self.token,
                        function.value_reg(),
                        exprs[0].value_reg(),
                        exprs[1].value_reg(),
                        exprs[2].value_reg(),
                        &va_args,
                        &reg,
                    )?,
                    _len => {
                        let args = self.alloc_register()?;
                        let array_reg = self.alloc_array::<LuaValue>(exprs.len())?;
                        MakeSlice::emit(
                            &self.current_builder,
                            &mut self.token,
                            &exprs.iter().map(|e| e.value_reg().clone()).collect::<Vec<_>>(),
                            &args,
                            array_reg,
                        )?;
                        self.free_array::<LuaValue>(exprs.len(), array_reg);
                        CallFunctionVA::emit(
                            &self.current_builder,
                            &mut self.token,
                            function.value_reg(),
                            &args,
                            &va_args,
                            &reg,
                        )?;
                    }
                }
            }
        }
        Ok(reg)
    }
    pub fn emit_call_ret1(
        &mut self,
        function: LuaExprRef<'l>,
        args: LuaExprList<'l>,
        reg: &Register<LuaValue>,
    ) -> Fallible<()> {
        let LuaExprList { exprs, va_arg } = args;
        let exprs: Vec<LuaExprRef<'l>> = exprs.into_iter().map(|arg| self.to_value(arg)).try_collect()?;
        match (exprs.len(), va_arg) {
            (0, None) => CallFunction0Ret1::emit(&self.current_builder, &mut self.token, function.value_reg(), reg)?,
            (1, None) => CallFunction1Ret1::emit(
                &self.current_builder,
                &mut self.token,
                function.value_reg(),
                exprs[0].value_reg(),
                reg,
            )?,
            (2, None) => CallFunction2Ret1::emit(
                &self.current_builder,
                &mut self.token,
                function.value_reg(),
                exprs[0].value_reg(),
                exprs[1].value_reg(),
                reg,
            )?,
            (3, None) => CallFunction3Ret1::emit(
                &self.current_builder,
                &mut self.token,
                function.value_reg(),
                exprs[0].value_reg(),
                exprs[1].value_reg(),
                exprs[2].value_reg(),
                reg,
            )?,
            (_len, None) => {
                let args = self.alloc_register()?;
                let array_reg = self.alloc_array::<LuaValue>(exprs.len())?;
                MakeSlice::emit(
                    &self.current_builder,
                    &mut self.token,
                    &exprs.iter().map(|e| e.value_reg().clone()).collect::<Vec<_>>(),
                    &args,
                    array_reg,
                )?;
                self.free_array::<LuaValue>(exprs.len(), array_reg);
                CallFunctionRet1::emit(&self.current_builder, &mut self.token, function.value_reg(), &args, reg)?;
            }
            (_, Some(VaArgs::VaArgs())) => {
                let va_args = self.va_args()?;
                match exprs.len() {
                    0 => CallFunction0VaSliceRet1::emit(
                        &self.current_builder,
                        &mut self.token,
                        function.value_reg(),
                        &va_args,
                        reg,
                    )?,
                    1 => CallFunction1VaSliceRet1::emit(
                        &self.current_builder,
                        &mut self.token,
                        function.value_reg(),
                        exprs[0].value_reg(),
                        &va_args,
                        reg,
                    )?,

                    2 => CallFunction2VaSliceRet1::emit(
                        &self.current_builder,
                        &mut self.token,
                        function.value_reg(),
                        exprs[0].value_reg(),
                        exprs[1].value_reg(),
                        &va_args,
                        reg,
                    )?,

                    3 => CallFunction3VaSliceRet1::emit(
                        &self.current_builder,
                        &mut self.token,
                        function.value_reg(),
                        exprs[0].value_reg(),
                        exprs[1].value_reg(),
                        exprs[2].value_reg(),
                        &va_args,
                        reg,
                    )?,
                    _len => {
                        let args = self.alloc_register()?;
                        let array_reg = self.alloc_array::<LuaValue>(exprs.len())?;
                        MakeSlice::emit(
                            &self.current_builder,
                            &mut self.token,
                            &exprs.iter().map(|e| e.value_reg().clone()).collect::<Vec<_>>(),
                            &args,
                            array_reg,
                        )?;
                        self.free_array::<LuaValue>(exprs.len(), array_reg);
                        let va_args = self.va_args()?;
                        CallFunctionVaSliceRet1::emit(
                            &self.current_builder,
                            &mut self.token,
                            function.value_reg(),
                            &args,
                            &va_args,
                            reg,
                        )?;
                    }
                }
            }
            (_, Some(VaArgs::FunctionCall(last_function, args))) => {
                let va_args = self.emit_call(last_function, *args)?;
                match exprs.len() {
                    0 => CallFunction0VaRet1::emit(
                        &self.current_builder,
                        &mut self.token,
                        function.value_reg(),
                        &va_args,
                        reg,
                    )?,
                    1 => CallFunction1VaRet1::emit(
                        &self.current_builder,
                        &mut self.token,
                        function.value_reg(),
                        exprs[0].value_reg(),
                        &va_args,
                        reg,
                    )?,
                    2 => CallFunction2VaRet1::emit(
                        &self.current_builder,
                        &mut self.token,
                        function.value_reg(),
                        exprs[0].value_reg(),
                        exprs[1].value_reg(),
                        &va_args,
                        reg,
                    )?,
                    3 => CallFunction3VaRet1::emit(
                        &self.current_builder,
                        &mut self.token,
                        function.value_reg(),
                        exprs[0].value_reg(),
                        exprs[1].value_reg(),
                        exprs[2].value_reg(),
                        &va_args,
                        reg,
                    )?,
                    _len => {
                        let args = self.alloc_register()?;
                        let array_reg = self.alloc_array::<LuaValue>(exprs.len())?;
                        MakeSlice::emit(
                            &self.current_builder,
                            &mut self.token,
                            &exprs.iter().map(|e| e.value_reg().clone()).collect::<Vec<_>>(),
                            &args,
                            array_reg,
                        )?;
                        self.free_array::<LuaValue>(exprs.len(), array_reg);
                        CallFunctionVaRet1::emit(
                            &self.current_builder,
                            &mut self.token,
                            function.value_reg(),
                            &args,
                            &va_args,
                            reg,
                        )?;
                    }
                }
            }
        }
        Ok(())
    }
    pub fn get_from_slice(&mut self, index: usize, c: LuaExprList<'l>) -> Fallible<LuaExprRef<'l>> {
        if let Some(expr) = c.exprs.get(index) {
            Ok(expr.clone())
        } else {
            let reg = self.alloc_register()?;
            match (index - c.exprs.len(), c.va_arg) {
                (_, None) => ConstNil::emit(&self.current_builder, &mut self.token, &reg)?,
                (index, Some(VaArgs::VaArgs())) => {
                    let va_args = self.va_args()?;
                    GetArg::emit(&self.current_builder, &mut self.token, Usize(index), &va_args, &reg)?
                }
                (index @ 1.., Some(VaArgs::FunctionCall(f, args))) => {
                    let rets = self.emit_call(f, *args)?;
                    GetRet::emit(&self.current_builder, &mut self.token, Usize(index), &rets, &reg)?;
                }
                (0, Some(VaArgs::FunctionCall(function, args))) => self.emit_call_ret1(function, *args, &reg)?,
                _ => unreachable!(),
            }
            Ok(Rc::new(
                LuaExprBuilder::default()
                    .register(LuaRegister::Value(reg, None))
                    .build()?,
            ))
        }
    }
    pub fn stat_call(&mut self, p: LuaExprRef<'l>, c: LuaExprList<'l>) -> Fallible<LuaExprRef<'l>> {
        let call = self.call(p, c)?;
        self.get_from_slice(0, call)
    }
    pub fn stat_call_self(
        &mut self,
        this: LuaExprRef<'l>,
        name: String,
        c: LuaExprList<'l>,
    ) -> Fallible<LuaExprRef<'l>> {
        let call = self.call_self(this, name, c)?;
        self.get_from_slice(0, call)
    }
    pub fn call(&mut self, p: LuaExprRef<'l>, c: LuaExprList<'l>) -> Fallible<LuaExprList<'l>> {
        Ok(LuaExprList {
            exprs: vec![],
            va_arg: Some(VaArgs::FunctionCall(p, Box::new(c))),
        })
    }
    // [prefix_expr(e),t!(:),Name(n),args(a)]=>ctx.call_self(e,n,a);
    pub fn call_self(
        &mut self,
        this: LuaExprRef<'l>,
        name: String,
        mut a: LuaExprList<'l>,
    ) -> Fallible<LuaExprList<'l>> {
        trace!("call_self");
        let reg = self.alloc_register()?;
        let name = self.const_string_value(name)?;
        let cache = self.empty_inline_cache_line()?;
        GetField::emit(
            &self.current_builder,
            &mut self.token,
            name,
            cache,
            U8(0),
            this.value_reg(),
            &reg,
        )?;
        a.exprs.insert(0, this);
        Ok(LuaExprList {
            exprs: vec![],
            va_arg: Some(VaArgs::FunctionCall(
                Rc::new(
                    LuaExprBuilder::default()
                        .register(LuaRegister::Value(reg, None))
                        .build()?,
                ),
                Box::new(a),
            )),
        })
    }
    // [t!(break)]=>cxt.break_();
    pub fn break_(&mut self) -> Fallible<()> {
        trace!("break_");
        let old_block = self.current_block.clone();
        let _new_block = self.new_block();
        let function = self.current_function();
        let _scopt = &function.current_scopt;
        for scopt in &function.scopts.clone() {
            match &scopt.borrow(&self.token).kind.clone() {
                ScoptKind::Other => {}
                ScoptKind::Loop { break_block } => {
                    self.branch(&old_block, break_block)?;
                    return Ok(());
                }
            }
        }
        Err(format_err!("invalid `break` statement"))
    }
    // [label(l)]=>cxt.define_label(l);
    pub fn define_label(&mut self, name: String) -> Fallible<()> {
        trace!("define_label");
        let function = self.current_function_mut();
        let _old_block = function.current_block.clone();
        let new_block = function.new_block();
        function
            .goto_blocks
            .entry(name.clone())
            .or_insert_with(Default::default)
            .push(new_block.clone());
        if let Some(goto_blocks_of_this_label) = function.goto_blocks.get(&name) {
            let goto_blocks_of_this_label = goto_blocks_of_this_label.clone();
            for goto_block in goto_blocks_of_this_label.iter() {
                self.branch(goto_block, &new_block)?;
            }
        }
        Ok(())
    }
    // [t!(goto),Name(n)]=>cxt.goto(n);
    pub fn goto(&mut self, name: String) -> Fallible<()> {
        trace!("goto");
        let function = self.current_function_mut();
        let old_block = function.current_block.clone();
        let _new_block = function.new_block();
        if let Some(label_block) = function.label_blocks.get(&name).cloned() {
            self.branch(&old_block, &label_block)?;
        } else {
            function
                .goto_blocks
                .entry(name)
                .or_insert_with(Default::default)
                .push(old_block);
        }
        Ok(())
    }
    // [t!(do),block(b),t!(end)]=>cxt.finish(b);
    pub fn finish_block(&mut self, (old_block, new_block): (LuaBlockRef<'l>, LuaBlockRef<'l>)) -> Fallible<()> {
        self.branch(&old_block, &new_block)?;
        Ok(())
    }
    // [t!(while),expr_wraped(e),block_split(s),t!(do),block(b),t!(end)]=>ctx.while_(e,s,b);
    pub fn while_(
        &mut self,
        ((pre_block_end, predicate_block_begin), predicate_expr): ((LuaBlockRef<'l>, LuaBlockRef<'l>), LuaExprRef<'l>),
        (predicate_block_end, loop_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
        (loop_block_end, post_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
    ) -> Fallible<()> {
        trace!("while_");
        self.branch_if(
            predicate_expr,
            &predicate_block_end,
            &loop_block_begin,
            &post_block_begin,
        )?;
        debug!(
            "while ({:?}:{:?}) do {:?},{:?} end",
            predicate_block_begin.borrow(self.token()).builder(),
            predicate_block_end.borrow(self.token()).builder(),
            loop_block_begin.borrow(self.token()).builder(),
            loop_block_end.borrow(self.token()).builder(),
        );
        self.branch(&loop_block_end, &predicate_block_begin)?;
        self.branch(&pre_block_end, &predicate_block_begin)?;
        Ok(())
    }
    // [t!(repeat),block_split(s),block(b),t!(until),expr_wraped(e)]=>ctx.repeat(s,b,e);
    pub fn repeat(
        &mut self,
        (pre_block_end, loop_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
        (loop_block_end, predicate_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
        ((predicate_block_end, post_block_begin), predicate_expr): ((LuaBlockRef<'l>, LuaBlockRef<'l>), LuaExprRef<'l>),
    ) -> Fallible<()> {
        trace!("repeat");
        self.branch_if(
            predicate_expr,
            &predicate_block_end,
            &loop_block_begin,
            &post_block_begin,
        )?;
        self.branch(&loop_block_end, &predicate_block_begin)?;
        self.branch(&pre_block_end, &loop_block_begin)?;
        Ok(())
    }
    // [expr_high_1(v),t!(or),block_split(b1),expr_high_2(v1),block_split(b2)]=>ctx.or(v,b1,v1,b2);
    pub fn or(
        &mut self,
        lhs: LuaExprRef<'l>,
        (lhs_block_end, rhs_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
        rhs: LuaExprRef<'l>,
        (rhs_block_end, post_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
    ) -> Fallible<LuaExprRef<'l>> {
        trace!("or");
        let rhs = self.to_value(rhs)?;
        let lhs = self.to_value(lhs)?;
        self.branch_if(lhs.clone(), &lhs_block_end, &post_block_begin, &rhs_block_begin)?;
        MoveValue::emit(
            &mut self.current_builder,
            &mut self.token,
            rhs.value_reg(),
            lhs.value_reg(),
        )?;
        self.branch(&rhs_block_end, &post_block_begin)?;
        Ok(lhs)
    }
    // [expr_high_1(v),t!(and),block_split(b1),expr_high_2(v1),block_split(b2)]=>ctx.and(v,b1,v1,b2);
    pub fn and(
        &mut self,
        lhs: LuaExprRef<'l>,
        (lhs_block_end, rhs_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
        rhs: LuaExprRef<'l>,
        (rhs_block_end, post_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
    ) -> Fallible<LuaExprRef<'l>> {
        trace!("and");
        let rhs = self.to_value(rhs)?;
        let lhs = self.to_value(lhs)?;
        self.branch_if(lhs.clone(), &lhs_block_end, &rhs_block_begin, &post_block_begin)?;
        MoveValue::emit(
            &mut self.current_builder,
            &mut self.token,
            rhs.value_reg(),
            lhs.value_reg(),
        )?;
        self.branch(&rhs_block_end, &post_block_begin)?;
        Ok(lhs)
    }
    // [t!(if),expr(e),block_split(b),block(c),t!(then)]=>ctx.if_(e,b,c);
    pub fn if_(
        &mut self,
        predict: LuaExprRef<'l>,
        (predict_block_end, then_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
        (then_block_end, else_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
    ) -> Fallible<(LuaBlockRef<'l>, LuaBlockRef<'l>)> {
        trace!("if_");
        self.branch_if(predict, &predict_block_end, &then_block_begin, &else_block_begin)?;
        Ok((then_block_end, else_block_begin))
    }
    // [if_prefix(p),t!(else),block(b),t!(end)]=>ctx.else_(p,a);
    pub fn else_(
        &mut self,
        (true_block_end, _false_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
        (false_block_end, else_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
        (else_block_end, post_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
    ) -> Fallible<()> {
        trace!("else_");
        self.branch(&true_block_end, &post_block_begin)?;
        self.branch(&false_block_end, &else_block_begin)?;
        self.branch(&else_block_end, &post_block_begin)?;
        Ok(())
    }
    // [if_prefix(p),t!(elseif),expr(e),block_split(c),t!(then),block(b),block_split(n)]=>ctx.elseif(p,e,c,b,n);
    pub fn elseif(
        &mut self,
        (true_block_end, _false_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
        predict: LuaExprRef<'l>,
        (predict_block_end, then_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
        (then_block_end, new_true_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
        (new_true_block_end, else_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
    ) -> Fallible<(LuaBlockRef<'l>, LuaBlockRef<'l>)> {
        trace!("elseif");
        self.branch_if(predict, &predict_block_end, &then_block_begin, &else_block_begin)?;
        self.branch(&true_block_end, &new_true_block_begin)?;
        self.branch(&then_block_end, &new_true_block_begin)?;
        Ok((new_true_block_end, else_block_begin))
    }
    // [if_prefix(p),t!(end),block_split(b)]=>ctx.end_if(p,b);
    pub fn end_if(
        &mut self,
        (true_block_end, _false_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
        (false_block_end, post_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
    ) -> Fallible<()> {
        trace!("end_if");
        self.branch(&true_block_end, &post_block_begin)?;
        self.branch(&false_block_end, &post_block_begin)?;
        Ok(())
    }
    pub fn loop_head(&mut self) -> Fallible<()> {
        for scopt in self.current_function().scopts.clone() {
            let mut variables = scopt.borrow(self.token()).variables.clone();
            for var in variables.values_mut() {
                var.expr = self.to_value(var.expr.clone())?;
            }
            scopt.borrow_mut(self.token_mut()).variables = variables;
        }
        Ok(())
    }

    pub fn for_head(
        &mut self,
        var: String,
        start: LuaExprRef<'l>,
        end: LuaExprRef<'l>,
        (init_block_end, predicate_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
        (predict_block_end, loop_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
    ) -> Fallible<(
        LuaExprRef<'l>,
        LuaExprRef<'l>,
        (LuaBlockRef<'l>, LuaBlockRef<'l>),
        (LuaBlockRef<'l>, LuaBlockRef<'l>),
        LuaExprRef<'l>,
    )> {
        let reg = self.alloc_register()?;
        let expr = LuaExpr::new_value(reg);
        self.add_local(var, Default::default(), expr.clone())?;
        let pre_block_end = &init_block_end.borrow(self.token()).builder().clone();
        self.current_builder = pre_block_end.clone();
        let start = self.to_value(start)?;
        let end = self.to_value(end)?;
        self.current_builder = loop_block_begin.borrow(self.token()).builder().clone();
        Ok((
            start,
            end,
            (init_block_end, predicate_block_begin),
            (predict_block_end, loop_block_begin),
            expr,
        ))
    }
    // [t!(for),Name(n),t!(=),expr(e),t!(,),expr(e1),block_split(p),block_split(p1),t!(do),block(b),t!(end)]=>ctx.for_(n,e,e1,p1,b);
    pub fn for_(
        &mut self,
        (start, end, (init_block_end, predicate_block_begin), (predicate_block_end, loop_block_begin), state): (
            LuaExprRef<'l>,
            LuaExprRef<'l>,
            (LuaBlockRef<'l>, LuaBlockRef<'l>),
            (LuaBlockRef<'l>, LuaBlockRef<'l>),
            LuaExprRef<'l>,
        ),
        (loop_block_end, post_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
    ) -> Fallible<()> {
        trace!("for_");
        let state_reg = state.value_reg();
        let predicate_block_begin = &predicate_block_begin.borrow(self.token()).builder().clone();
        let predicate_block_end = &predicate_block_end.borrow(self.token()).builder().clone();
        let loop_block_begin = &loop_block_begin.borrow(self.token()).builder().clone();
        let loop_block_end = &loop_block_end.borrow(self.token()).builder().clone();
        let post_block_begin = &post_block_begin.borrow(self.token()).builder().clone();
        let pre_block_end = &init_block_end.borrow(self.token()).builder().clone();
        ForLoopInit::emit(
            pre_block_end,
            &mut self.token,
            predicate_block_begin,
            start.value_reg(),
            end.value_reg(),
            state_reg,
        )?;
        ForLoopJump::emit(
            predicate_block_end,
            &mut self.token,
            loop_block_begin,
            post_block_begin,
            end.value_reg(),
            state_reg,
        )?;
        ForLoopIncrease::emit(loop_block_end, &mut self.token, predicate_block_begin, state_reg)?;
        Ok(())
    }
    pub fn for_step_head(
        &mut self,
        var: String,
        start: LuaExprRef<'l>,
        end: LuaExprRef<'l>,
        step: LuaExprRef<'l>,
        (init_block_end, predicate_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
        (predict_block_end, loop_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
    ) -> Fallible<(
        LuaExprRef<'l>,
        LuaExprRef<'l>,
        LuaExprRef<'l>,
        (LuaBlockRef<'l>, LuaBlockRef<'l>),
        (LuaBlockRef<'l>, LuaBlockRef<'l>),
        LuaExprRef<'l>,
    )> {
        let reg = self.alloc_register()?;
        let expr = LuaExpr::new_value(reg);
        self.add_local(var, Default::default(), expr.clone())?;
        let init_block_end_builder = &init_block_end.borrow(self.token()).builder().clone();
        self.current_builder = init_block_end_builder.clone();
        let start = self.to_value(start)?;
        let end = self.to_value(end)?;
        let step = self.to_value(step)?;
        self.current_builder = loop_block_begin.borrow(self.token()).builder().clone();
        Ok((
            start,
            end,
            step,
            (init_block_end, predicate_block_begin),
            (predict_block_end, loop_block_begin),
            expr,
        ))
    }
    // [t!(for),Name(n),t!(=),expr(e),t!(,),expr(e1),t!(,),expr(e2),block_split(p),block_split(p1),t!(do),block(b),t!(end)]=>ctx.for_step(n,e,e1,e2,p1,b);
    pub fn for_step(
        &mut self,
        (start, end, step, (init_block_end, predicate_block_begin), (predicate_block_end, loop_block_begin), state): (
            LuaExprRef<'l>,
            LuaExprRef<'l>,
            LuaExprRef<'l>,
            (LuaBlockRef<'l>, LuaBlockRef<'l>),
            (LuaBlockRef<'l>, LuaBlockRef<'l>),
            LuaExprRef<'l>,
        ),
        (loop_block_end, post_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
    ) -> Fallible<()> {
        trace!("for_step");
        let state_reg = state.value_reg();
        debug!("for_ {:?} = {:?},{:?},{:?}", &state_reg, &start, &end, &step);
        let predicate_block_begin = &predicate_block_begin.borrow(self.token()).builder().clone();
        let predicate_block_end = &predicate_block_end.borrow(self.token()).builder().clone();
        let loop_block_begin = &loop_block_begin.borrow(self.token()).builder().clone();
        let loop_block_end = &loop_block_end.borrow(self.token()).builder().clone();
        let post_block_begin = &post_block_begin.borrow(self.token()).builder().clone();
        let init_block_end = &init_block_end.borrow(self.token()).builder().clone();
        debug!(
            "init_block:{:?} ({:?},{:?}) do {:?},{:?} end {:?}",
            init_block_end.codes(),
            predicate_block_begin.codes(),
            predicate_block_end.codes(),
            loop_block_begin.codes(),
            loop_block_end.codes(),
            post_block_begin.codes()
        );
        ForStepLoopInit::emit(
            init_block_end,
            &mut self.token,
            predicate_block_begin,
            start.value_reg(),
            end.value_reg(),
            step.value_reg(),
            state_reg,
        )?;
        ForStepLoopJump::emit(
            predicate_block_end,
            &mut self.token,
            loop_block_begin,
            post_block_begin,
            end.value_reg(),
            step.value_reg(),
            state_reg,
        )?;
        ForStepLoopIncrease::emit(
            loop_block_end,
            &mut self.token,
            predicate_block_begin,
            state_reg,
            step.value_reg(),
        )?;
        Ok(())
    }
    pub fn for_in_head(
        &mut self,
        mut vars: Vec<String>,
        exprs: LuaExprList<'l>,
        (init_block_end, predicate_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
        (predict_block_end, loop_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
    ) -> Fallible<(
        LuaExprList<'l>,
        (LuaBlockRef<'l>, LuaBlockRef<'l>),
        (LuaBlockRef<'l>, LuaBlockRef<'l>),
        Vec<LuaExprRef<'l>>,
    )> {
        let mut new_vars = Vec::new();
        let rets = vars.split_off(1);
        for var in rets {
            let reg = self.alloc_register()?;
            let expr = LuaExpr::new_value(reg.clone());
            self.add_local(var, Default::default(), expr.clone())?;
            new_vars.push(expr);
        }
        let pre_block_end = &init_block_end.borrow(self.token()).builder().clone();
        self.current_builder = pre_block_end.clone();
        let state_less_iter = self.expr_list_to_vec(exprs, 3)?;
        let iter = self.to_value(state_less_iter[0].clone())?;
        let iterable = self.to_value(state_less_iter[1].clone())?;
        let state = self.to_writable_value(state_less_iter[2].clone())?;
        let exprs = LuaExprList {
            exprs: vec![iter, iterable, state],
            va_arg: None,
        };
        self.current_builder = loop_block_begin.borrow(self.token()).builder().clone();
        Ok((
            exprs,
            (init_block_end, predicate_block_begin),
            (predict_block_end, loop_block_begin),
            new_vars,
        ))
    }
    // [t!(for),name_list(n),t!(in),expr_list(e),block_split(p),block_split(p1),t!(do),block(b),t!(end)]=>ctx.for_in(n,e,p,b);
    pub fn for_in(
        &mut self,
        (exprs, (init_block_end, predicate_block_begin), (predicate_block_end, loop_block_begin), new_vars): (
            LuaExprList<'l>,
            (LuaBlockRef<'l>, LuaBlockRef<'l>),
            (LuaBlockRef<'l>, LuaBlockRef<'l>),
            Vec<LuaExprRef<'l>>,
        ),
        (loop_block_end, post_block_begin): (LuaBlockRef<'l>, LuaBlockRef<'l>),
    ) -> Fallible<()> {
        self.branch(&loop_block_end, &predicate_block_begin)?;
        self.branch(&init_block_end, &predicate_block_begin)?;
        let predicate_block_begin = &predicate_block_begin.borrow(self.token()).builder().clone();
        let predicate_block_end = &predicate_block_end.borrow(self.token()).builder().clone();
        let loop_block_begin = &loop_block_begin.borrow(self.token()).builder().clone();
        let post_block_begin = &post_block_begin.borrow(self.token()).builder().clone();
        let state_less_iter = self.expr_list_to_vec(exprs, 3)?;
        let _init_block_end = &init_block_end.borrow(self.token()).builder().clone();
        let iter = self.to_value(state_less_iter[0].clone())?;
        let iterable = self.to_value(state_less_iter[1].clone())?;
        let state = self.to_writable_value(state_less_iter[2].clone())?;
        match new_vars.len() {
            0 => {
                ForInLoopJump1::emit(
                    predicate_block_end,
                    &mut self.token,
                    loop_block_begin,
                    post_block_begin,
                    iter.value_reg(),
                    iterable.value_reg(),
                    state.value_reg(),
                )?;
            }
            1 => {
                let ret1 = new_vars[0].value_reg();
                ForInLoopJump2::emit(
                    predicate_block_end,
                    &mut self.token,
                    loop_block_begin,
                    post_block_begin,
                    iter.value_reg(),
                    iterable.value_reg(),
                    state.value_reg(),
                    ret1,
                )?;
            }
            _o => {
                let rets = self.alloc_register()?;
                ForInLoopJump::emit(
                    predicate_block_end,
                    &mut self.token,
                    loop_block_begin,
                    post_block_begin,
                    iter.value_reg(),
                    iterable.value_reg(),
                    state.value_reg(),
                    &rets,
                )?;
                for (var_index, var) in new_vars.into_iter().enumerate() {
                    GetRet::emit(
                        predicate_block_begin,
                        &mut self.token,
                        Usize(var_index + 1),
                        &rets,
                        var.value_reg(),
                    )?;
                }
            }
        };
        Ok(())
    }
    // [t!(function),function_boby(f)]=>ctx.function(f);
    pub fn set_function(&mut self, name: String, body: LuaFunctionBuilderRef<'l>) -> Fallible<()> {
        trace!("set_function");
        let function = self.const_function(body)?;
        self.add_local(name, Default::default(), function)
    }
    // [t!(local),t!(function),Name(n),function_boby(f)]=>ctx.local_function(n,f);
    pub fn local_function(&mut self, name: String, body: LuaFunctionBuilderRef<'l>) -> Fallible<()> {
        trace!("local_function");
        let function = self.const_function(body)?;
        self.put_value(name, function)
    }
    pub fn load_var(&mut self, var: LuaVar<'l>) -> Fallible<LuaExprRef<'l>> {
        trace!("load_var");
        match var {
            LuaVar::Variable(v) => self.get_value(v),
            LuaVar::Field(t, f) => {
                let reg = self.alloc_register()?;
                let name = self.const_string_value(f)?;
                let cache = self.empty_inline_cache_line()?;
                GetField::emit(
                    &self.current_builder,
                    &mut self.token,
                    name,
                    cache,
                    U8(0),
                    t.value_reg(),
                    &reg,
                )?;
                Ok(Rc::new(
                    LuaExprBuilder::default()
                        .register(LuaRegister::Value(reg, None))
                        .build()?,
                ))
            }
            LuaVar::Element(t, k) => {
                let reg = self.alloc_register()?;
                let cache = self.empty_inline_cache_line()?;
                GetElement::emit(
                    &self.current_builder,
                    &mut self.token,
                    cache,
                    t.value_reg(),
                    k.value_reg(),
                    &reg,
                )?;
                Ok(Rc::new(
                    LuaExprBuilder::default()
                        .register(LuaRegister::Value(reg, None))
                        .build()?,
                ))
            }
        }
    }
    pub fn add_local(&mut self, name: String, attr: VarAttribute, expr: LuaExprRef<'l>) -> Fallible<()> {
        self.current_scopt_mut().variables.insert(name, LuaVariable {
            expr,
            attributes: attr,
            upvalue: None,
        });
        Ok(())
    }
    // [t!(local),att_name_list(a)]=>ctx.local_variable(a);
    pub fn local_variable(&mut self, names: Vec<(std::string::String, VarAttribute)>) -> Fallible<()> {
        trace!("local_variable");
        for (name, attr) in names {
            let reg = self.alloc_register()?;
            ConstNil::emit(&self.current_builder, &mut self.token, &reg)?;
            self.add_local(name, attr, LuaExpr::new_value(reg))?;
        }
        Ok(())
    }
    // [t!(local),att_name_list(a),t!(=),expr_list(e)]=>ctx.local_variable_with_values(a,e);
    pub fn local_variable_with_values(
        &mut self,
        names: Vec<(std::string::String, VarAttribute)>,
        exprs: LuaExprList<'l>,
    ) -> Fallible<()> {
        trace!("local_variable_with_values");
        let len = names.len();
        for ((name, attr), expr) in names.into_iter().zip(self.expr_list_to_vec(exprs, len)?) {
            self.add_local(name, attr, expr)?;
        }
        Ok(())
    }
    //  [expr_high_8(v),t!(..),expr_high_8(v1)]=>ctx.concat(v,v1);
    pub fn concat(&mut self, expr1: LuaExprRef<'l>, expr2: LuaExprRef<'l>) -> Fallible<LuaExprRef<'l>> {
        trace!("concat");
        let expr1 = self.to_value(expr1)?;
        let expr2 = self.to_value(expr2)?;
        let expr2 = self.to_writable(expr2)?;
        let result = match (&expr1.register, &expr2.register) {
            (LuaRegister::Value(r1, _), LuaRegister::Value(r2, _)) => {
                Concat::emit(&self.current_builder, &mut self.token, &LUA_STATE_REG, r1, r2)?;
                LuaRegister::Value(r2.clone(), None)
            }
            _ => unreachable!(),
        };
        Ok(Rc::new(LuaExprBuilder::default().register(result).build()?))
    }
    pub fn trans_binary_type(
        use_integer: bool,
        use_float: bool,
        expr1: &LuaExprRef<'l>,
        expr2: &LuaExprRef<'l>,
    ) -> LuaRegisterKind {
        use LuaRegisterKind::*;
        if !use_integer && !use_float {
            Value
        } else {
            match (expr1.register.kind(), expr2.register.kind()) {
                (Value, _) | (_, Value) => Value,
                (Integer, Integer) => {
                    if use_integer {
                        Integer
                    } else {
                        Float
                    }
                }
                (Float, Float) | (Integer, Float) | (Float, Integer) => match (use_integer, use_float) {
                    (true, true) => Float,
                    (false, true) => Float,
                    (true, false) => Integer,
                    (false, false) => Value,
                },
            }
        }
    }
    pub fn transport_register<F: TypeDeclaration, T: TypeDeclaration>(
        &mut self,
        register: &Register<F>,
        emit: impl FnOnce(
            &BlockBuilder<'l, LuaInstructionSet>,
            &mut GhostToken<'l>,
            &Register<F>,
            &Register<T>,
        ) -> Fallible<()>,
    ) -> Fallible<Register<T>> {
        let new_reg = self.alloc_register()?;
        emit(&self.current_builder, &mut self.token, register, &new_reg)?;
        Ok(new_reg)
    }
    pub fn transform_expr(&mut self, expr: LuaExprRef<'l>, kind: LuaRegisterKind) -> Fallible<LuaExprRef<'l>> {
        let result = match (&expr.register, kind) {
            (LuaRegister::Integer(_r), LuaRegisterKind::Integer) => return Ok(expr),
            (LuaRegister::Integer(r), LuaRegisterKind::Float) => {
                LuaRegister::Float(self.transport_register(r, I64ToF64::emit)?)
            }
            (LuaRegister::Integer(r), LuaRegisterKind::Value) => {
                LuaRegister::Value(self.transport_register(r, I64ToValue::emit)?, None)
            }
            (LuaRegister::Float(r), LuaRegisterKind::Integer) => {
                LuaRegister::Integer(self.transport_register(r, ir::F64ToI64::emit)?)
            }
            (LuaRegister::Float(_r), LuaRegisterKind::Float) => return Ok(expr),
            (LuaRegister::Float(r), LuaRegisterKind::Value) => {
                LuaRegister::Value(self.transport_register(r, F64ToValue::emit)?, None)
            }
            (LuaRegister::Function(_r, _) | LuaRegister::Value(_r, _), LuaRegisterKind::Value) => return Ok(expr),
            _ => unreachable!(),
        };
        Ok(Rc::new(LuaExprBuilder::default().register(result).build()?))
    }
    pub fn binary_operate(
        &mut self,
        expr1: LuaExprRef<'l>,
        expr2: LuaExprRef<'l>,
        emit_int: Option<binary_operate_type!(e::I64)>,
        emit_float: Option<binary_operate_type!(e::F64)>,
        emit_value: binary_operate_type!(LuaValue),
    ) -> Fallible<LuaExprRef<'l>> {
        let operate_kind = Self::trans_binary_type(emit_int.is_some(), emit_float.is_some(), &expr1, &expr2);
        let expr1 = Self::transform_expr(self, expr1, operate_kind)?;
        let expr2 = Self::transform_expr(self, expr2, operate_kind)?;
        let expr2 = self.to_writable(expr2)?;
        let result = match (&expr1.register, &expr2.register) {
            (LuaRegister::Integer(r1), LuaRegister::Integer(r2)) => {
                (emit_int.unwrap())(&self.current_builder, &mut self.token, r1, r2)?;
                LuaRegister::Integer(r2.clone())
            }
            (LuaRegister::Float(r1), LuaRegister::Float(r2)) => {
                (emit_float.unwrap())(&self.current_builder, &mut self.token, r1, r2)?;
                LuaRegister::Float(r2.clone())
            }
            (LuaRegister::Value(r1, _), LuaRegister::Value(r2, _)) => {
                emit_value(&self.current_builder, &mut self.token, r1, r2)?;
                LuaRegister::Value(r2.clone(), None)
            }
            _ => unreachable!(),
        };
        Ok(Rc::new(LuaExprBuilder::default().register(result).build()?))
    }
    pub fn binary_to_bool_operate(
        &mut self,
        expr1: LuaExprRef<'l>,
        expr2: LuaExprRef<'l>,
        emit_int: Option<binary_operate_type!(e::I64, LuaValue)>,
        emit_float: Option<binary_operate_type!(e::F64, LuaValue)>,
        emit_value: binary_operate_type!(LuaValue),
    ) -> Fallible<LuaExprRef<'l>> {
        let operate_kind = Self::trans_binary_type(emit_int.is_some(), emit_float.is_some(), &expr1, &expr2);
        let expr1 = Self::transform_expr(self, expr1, operate_kind)?;
        let expr2 = Self::transform_expr(self, expr2, operate_kind)?;
        let result = match (&expr1.register, &expr2.register) {
            (LuaRegister::Integer(r1), LuaRegister::Integer(r2)) => {
                let r3 = self.alloc_register()?;
                (emit_int.unwrap())(&self.current_builder, &mut self.token, r1, r2, &r3)?;
                LuaExpr::new_value(r3)
            }
            (LuaRegister::Float(r1), LuaRegister::Float(r2)) => {
                let r3 = self.alloc_register()?;
                (emit_float.unwrap())(&self.current_builder, &mut self.token, r1, r2, &r3)?;
                LuaExpr::new_value(r3)
            }
            (LuaRegister::Value(r1, _), LuaRegister::Value(_r2, _)) => {
                let expr2 = self.to_writable(expr2)?;
                emit_value(&self.current_builder, &mut self.token, r1, expr2.value_reg())?;
                expr2
            }
            _ => unreachable!(),
        };
        Ok(result)
    }
    pub fn unique_operate(
        &mut self,
        expr1: LuaExprRef<'l>,
        emit_int: Option<unique_operate_type!(e::I64)>,
        emit_float: Option<unique_operate_type!(e::F64)>,
        emit_object: unique_operate_type!(LuaValue),
    ) -> Fallible<LuaExprRef<'l>> {
        let expr1 = self.to_writable(expr1)?;
        let builder = &self.current_builder;
        let result = match &expr1.register {
            LuaRegister::Integer(r1) => {
                (emit_int.unwrap())(builder, &mut self.token, r1)?;
                LuaRegister::Integer(r1.clone())
            }
            LuaRegister::Float(r1) => {
                (emit_float.unwrap())(builder, &mut self.token, r1)?;
                LuaRegister::Float(r1.clone())
            }
            LuaRegister::Value(r1, s) => {
                emit_object(builder, &mut self.token, r1)?;
                LuaRegister::Value(r1.clone(), *s)
            }
            _ => unreachable!(),
        };
        Ok(Rc::new(LuaExprBuilder::default().register(result).build()?))
    }
    pub fn emit_const_value(&mut self, emit_object: unique_operate_type!(LuaValue)) -> Fallible<LuaExprRef<'l>> {
        let _expr1 = LuaExpr::new_value(self.alloc_register()?);
        let r1 = self.alloc_register()?;
        emit_object(&self.current_builder, &mut self.token, &r1)?;
        Ok(LuaExpr::new_value(r1))
    }
    fn to_writable_value(&mut self, expr: LuaExprRef<'l>) -> Fallible<LuaExprRef<'l>> {
        let writeable = self.to_writable(expr)?;
        self.to_value(writeable)
    }
    pub fn to_value(&mut self, expr: LuaExprRef<'l>) -> Fallible<LuaExprRef<'l>> {
        Ok(match &expr.register {
            LuaRegister::Integer(r) => {
                let new_reg = self.alloc_register()?;
                debug!("to value {:?}->{:?}", &expr, &new_reg);
                I64ToValue::emit(&self.current_builder, &mut self.token, r, &new_reg)?;
                LuaExpr::new_value(new_reg)
            }
            LuaRegister::Float(r) => {
                let new_reg = self.alloc_register()?;
                debug!("to value {:?}->{:?}", &expr, &new_reg);
                F64ToValue::emit(&self.current_builder, &mut self.token, r, &new_reg)?;
                LuaExpr::new_value(new_reg)
            }
            _ => expr,
        })
    }
    pub fn to_writable(&mut self, expr: LuaExprRef<'l>) -> Fallible<LuaExprRef<'l>> {
        match &expr.lifetime {
            ExprLifeTimeKind::Own => Ok(expr),
            ExprLifeTimeKind::COW => {
                let reg = match &expr.register {
                    LuaRegister::Integer(r) => {
                        let new_reg = self.alloc_register()?;
                        MoveI64::emit(&self.current_builder, &mut self.token, r, &new_reg)?;
                        LuaRegister::Integer(new_reg)
                    }
                    LuaRegister::Float(r) => {
                        let new_reg = self.alloc_register()?;
                        MoveF64::emit(&self.current_builder, &mut self.token, r, &new_reg)?;
                        LuaRegister::Float(new_reg)
                    }
                    LuaRegister::Function(r, s) => {
                        let new_reg = self.alloc_register()?;
                        MoveValue::emit(&self.current_builder, &mut self.token, r, &new_reg)?;
                        LuaRegister::Function(new_reg, s.clone())
                    }
                    LuaRegister::Value(r, s) => {
                        let new_reg = self.alloc_register()?;
                        MoveValue::emit(&self.current_builder, &mut self.token, r, &new_reg)?;
                        LuaRegister::Value(new_reg, *s)
                    }
                };
                debug!("to writeable {:?}<-{:?}", &reg, &expr);
                Ok(Rc::new(LuaExprBuilder::default().register(reg).build()?))
            }
        }
    }
    pub fn alloc_register<T: TypeDeclaration>(&mut self) -> Fallible<Register<T>> {
        BuddyRegisterPool::alloc(self.current_function().register_pool.clone())
            .ok_or_else(|| format_err!("not left register"))
    }
    pub fn alloc_array<T: TypeDeclaration>(&mut self, len: usize) -> Fallible<u16> {
        self.current_function_mut()
            .register_pool
            .borrow_mut()
            .raw_alloc(len * T::LAYOUT.into_flexible_array().flexible_size() + size_of::<usize>())
            .ok_or_else(|| format_err!("not left register"))
    }
    pub fn free_array<T: TypeDeclaration>(&mut self, len: usize, reg: u16) {
        self.current_function_mut().register_pool.borrow_mut().raw_free(
            len * T::LAYOUT.into_flexible_array().flexible_size() + size_of::<usize>(),
            reg,
        )
    }
}
