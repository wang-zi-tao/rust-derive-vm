use std::{
    alloc::Layout, any::TypeId, borrow::Borrow, cell::RefCell, collections::HashMap, fmt::Debug, hash::Hash, marker::PhantomData, mem::size_of, ops::Range,
    rc::Rc,
};

use derive_builder::Builder;
use failure::Fallible;
use ghost_cell::{GhostCell, GhostToken};
use smallvec::SmallVec;
use util::CowSlice;
use vm_core::{self, FunctionType, ObjectBuilder, ObjectBuilderImport, ObjectBuilderInner, ObjectRef, SymbolBuilder, TypeDeclaration};

use crate::instructions::InstructionSet;

#[derive(Clone, Copy)]
pub enum SegmentKind {
    BasicBlock,
    StackMap,
    Constants,
    Reference,
}
#[derive(Builder, Getters, CopyGetters)]
pub struct FunctionPack<S: 'static> {
    #[builder(default)]
    pub _ph: PhantomData<S>,
    #[getset(get = "pub")]
    pub byte_code: ObjectRef,
    #[getset(get = "pub")]
    pub function_type: FunctionType,
    #[getset(get_copy = "pub")]
    pub register_count: u16,
    #[builder(default)]
    pub output: Option<ObjectRef>,
}

impl<S> Debug for FunctionPack<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunctionPack")
            .field("byte_code", &self.byte_code)
            .field("function_type", &self.function_type)
            .field("register_count", &self.register_count)
            .finish()
    }
}
unsafe impl<S> Sync for FunctionPack<S> {}
unsafe impl<S> Send for FunctionPack<S> {}
pub struct Function {
    pub blocks_metadata: CowSlice<'static, (Range<u32>, SegmentKind)>,
    pub blocks: ObjectRef,
    pub constant_offset: u32,
    // pub interpreter: fn(
    //     // ip
    //     *const u8,
    //     // args
    //     *const usize,
    //     // arg_count
    //     usize,
    //     // last_frame_registers
    //     *const usize,
    // ) -> isize,
}
pub struct FunctionBuilder<'l, S> {
    blocks: Vec<BlockBuilder<'l, S>>,
    remote_constants: ObjectBuilder<'l>,
}
impl<'l, S> Default for FunctionBuilder<'l, S> {
    fn default() -> Self {
        Self { blocks: Default::default(), remote_constants: Default::default() }
    }
}
impl<'l, S> FunctionBuilder<'l, S> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn blocks(&self) -> &Vec<BlockBuilder<'l, S>> {
        &self.blocks
    }

    pub fn add_block(&mut self, block: BlockBuilder<'l, S>) {
        self.blocks.push(block);
    }

    pub fn new_block(&mut self) -> BlockBuilder<'l, S> {
        let block = BlockBuilder::default();
        self.add_block(block.clone());
        block
    }

    pub fn remote_constants(&self) -> &ObjectBuilder<'l> {
        &self.remote_constants
    }

    pub fn remote_constants_mut(&mut self) -> &mut ObjectBuilder<'l> {
        &mut self.remote_constants
    }

    pub fn pack(self, token: &mut GhostToken<'l>, function_type: FunctionType, register_count: u16) -> Fallible<FunctionPack<S>> {
        self.pack_into(token, function_type, register_count, Default::default())
    }

    pub fn pack_into(self, token: &mut GhostToken<'l>, function_type: FunctionType, register_count: u16, output: ObjectRef) -> Fallible<FunctionPack<S>> {
        let blocks = self.blocks;
        let remote_constants = self.remote_constants;
        let mut buffer = ObjectBuilder::default();
        for block in blocks {
            buffer = ObjectBuilder::merge(token, buffer, block.codes);
        }
        buffer = ObjectBuilder::merge(token, buffer, remote_constants);
        buffer.borrow_mut(token).add_symbol(SymbolBuilder::default().offset(0).build()?);
        let object = buffer.take(token).build()?;
        Ok(FunctionPack { _ph: PhantomData, byte_code: object, function_type, register_count, output: Some(output) })
    }
}
#[derive(Getters)]
pub struct BlockBuilder<'l, S> {
    #[getset(get = "pub")]
    codes: ObjectBuilder<'l>,
    phantom_data: PhantomData<fn(S) -> S>,
}

impl<'l, S> std::ops::Deref for BlockBuilder<'l, S> {
    type Target = ObjectBuilder<'l>;

    fn deref(&self) -> &Self::Target {
        &self.codes
    }
}

impl<'l, S> Debug for BlockBuilder<'l, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&format!("BlockBuilder@{:?}", &**self.codes as *const GhostCell<'_, ObjectBuilderInner>)).field("codes", &self.codes).finish()
    }
}

impl<'l, S> Clone for BlockBuilder<'l, S> {
    fn clone(&self) -> Self {
        Self { codes: self.codes.clone(), phantom_data: self.phantom_data }
    }
}
impl<'l, S> Default for BlockBuilder<'l, S> {
    fn default() -> Self {
        let mut object_builder = ObjectBuilderInner::default();
        object_builder.add_symbol(SymbolBuilder::default().offset(0).build().unwrap());
        let codes = object_builder.into();
        Self { codes, phantom_data: PhantomData }
    }
}
impl<'l, S: InstructionSet> BlockBuilder<'l, S> {
    pub unsafe fn emit<F: Copy>(&self, token: &mut GhostToken<'l>, value: F) {
        self.codes.borrow_mut(token).push(value)
    }

    pub unsafe fn emit_opcode(&self, token: &mut GhostToken<'l>, opcode: usize) {
        let b = self.codes.borrow_mut(token);
        match S::INSTRUCTIONS.len() {
            0..=0xff => {
                b.push(opcode as u8);
            }
            0x1_00..=0xffff => {
                b.align(2);
                b.push(opcode as u16);
            }
            _ => panic!("too many instructions"),
        }
    }

    pub unsafe fn emit_register<T: TypeDeclaration, A: RegisterPool>(&self, token: &mut GhostToken<'l>, register: &Register<T, A>) {
        let b = self.codes.borrow_mut(token);
        b.align(2);
        b.push(register.reg());
    }

    pub unsafe fn push_block_offset(&self, token: &mut GhostToken<'l>, block: &BlockBuilder<'l, S>) {
        let import = if self.codes() == block.codes() {
            ObjectBuilderImport::Reflexive
        } else {
            ObjectBuilderImport::Builder(block.codes().clone())
        };
        ObjectBuilderInner::push_import(self.codes(), token, import, vm_core::RelocationKind::I32Relative, 0);
    }
}
#[derive(Debug)]
pub struct RegisterInner<Alloc: RegisterPool = BuddyRegisterPool>(u16, usize, Rc<RefCell<Alloc>>);

impl<Alloc: RegisterPool> Drop for RegisterInner<Alloc> {
    fn drop(&mut self) {
        self.2.borrow_mut().free(self.1, self);
    }
}

pub struct Register<T, Alloc: RegisterPool> {
    reg: u16,
    inner: Option<Rc<RegisterInner<Alloc>>>,
    _ph: PhantomData<T>,
}

impl<T, Alloc: RegisterPool> Debug for Register<T, Alloc> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Register").field(&self.reg).finish()
    }
}

impl<T, Alloc: RegisterPool> Clone for Register<T, Alloc> {
    fn clone(&self) -> Self {
        Self { reg: self.reg, inner: self.inner.clone(), _ph: self._ph }
    }
}

impl<T: TypeDeclaration, A: RegisterPool> Register<T, A> {
    pub const fn new_const(reg: u16) -> Self {
        Self { reg, inner: None, _ph: PhantomData }
    }

    pub fn new(reg: u16, allocator: Rc<RefCell<A>>) -> Self {
        let layout = T::LAYOUT;
        let size = usize::max(layout.size(), layout.align());
        Self { reg, inner: Some(Rc::new(RegisterInner(reg, size, allocator))), _ph: PhantomData }
    }

    pub fn layout(&self) -> Layout {
        Layout::new::<T>()
    }

    pub fn size(&self) -> usize {
        let layout = self.layout();
        usize::max(layout.size(), layout.align())
    }

    pub fn forget(self) {
        std::mem::forget(self)
    }

    pub fn reg(&self) -> u16 {
        self.reg
    }
}

#[derive(Debug, Getters, CopyGetters)]
pub struct Variable<A: RegisterPool> {
    #[getset(get_copy = "pub")]
    register: u16,
    #[getset(get = "pub")]
    inner: Option<Rc<RegisterInner<A>>>,
    #[getset(get_copy = "pub")]
    type_id: TypeId,
}
impl<A: RegisterPool> Drop for Variable<A> {
    fn drop(&mut self) {
        panic!("Use `RegisterPool::free_variable` or `Variable::forget` to drop a variable!")
    }
}
impl<A: RegisterPool> Variable<A> {
    pub fn as_ref<T: 'static>(&self) -> Register<T, A> {
        assert_eq!(TypeId::of::<T>(), self.type_id);
        Register { reg: self.register, inner: self.inner.clone(), _ph: PhantomData }
    }

    pub fn try_as_ref<T: 'static>(&self) -> Option<Register<T, A>> {
        if TypeId::of::<T>() == self.type_id {
            None
        } else {
            Some(Register { reg: self.register, inner: self.inner.clone(), _ph: PhantomData })
        }
    }

    pub fn forget(self) {
        std::mem::forget(self)
    }
}
impl<A: RegisterPool, T: 'static> From<Register<T, A>> for Variable<A> {
    fn from(i: Register<T, A>) -> Self {
        Variable { register: i.reg, inner: i.inner, type_id: TypeId::of::<T>() }
    }
}
#[derive(Debug, Default, Getters, CopyGetters)]
pub struct VariableSet<K, A: RegisterPool> {
    #[getset(get = "pub")]
    map: HashMap<K, Variable<A>>,
}
impl<K, A: RegisterPool> VariableSet<K, A>
where
    K: Hash + Eq,
{
    pub fn get<Q>(&self, key: &Q) -> Option<&Variable<A>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.map.get(key)
    }

    pub fn set(&mut self, key: K, var: Variable<A>) -> Option<Variable<A>> {
        self.map.insert(key, var)
    }

    pub fn take<Q>(&mut self, key: &Q) -> Option<(K, Variable<A>)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.map.remove_entry(key)
    }

    pub fn remove<Q>(&mut self, key: &Q) -> Option<K>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.map.remove_entry(key).map(|(k, v)| {
            v.forget();
            k
        })
    }
}
pub trait RegisterPool: Sized {
    fn new() -> Rc<RefCell<Self>>;
    fn reserve_range(regs: Range<u16>) -> Rc<RefCell<Self>>;
    fn alloc<T: TypeDeclaration>(this: Rc<RefCell<Self>>) -> Option<Register<T, Self>> {
        let start_offset = {
            let mut this_mut = this.borrow_mut();
            let layout = T::LAYOUT;
            let size = usize::max(layout.size(), layout.align());
            this_mut.raw_alloc(size)?
        };
        Some(Register::new(start_offset, this))
    }
    fn raw_alloc(&mut self, size: usize) -> Option<u16>;
    fn free(&mut self, size: usize, reg: &RegisterInner<Self>);
    fn raw_free(&mut self, size: usize, reg: u16);
}
#[derive(Getters, CopyGetters, Debug)]
pub struct BuddyRegisterPool {
    #[getset(get_copy = "pub")]
    max_allocated: u16,
    allocator: [Vec<u16>; u16::BITS as usize],
}
impl RegisterPool for BuddyRegisterPool {
    fn new() -> Rc<RefCell<Self>> {
        let mut allocator: [Vec<u16>; u16::BITS as usize] = Default::default();
        allocator[15].push(0);
        Rc::new(RefCell::new(Self { max_allocated: 0, allocator }))
    }

    fn raw_alloc(&mut self, size: usize) -> Option<u16> {
        let size = if size < 1 { size_of::<usize>() } else { 1 << (usize::BITS - (size - 1).leading_zeros()) };
        let level = size.div_euclid(size_of::<usize>()).trailing_zeros() as usize;
        let (alloc_level, start_offset) = self.allocator[level..16].iter_mut().enumerate().find_map(|(l, v)| v.pop().map(|r| (l + level, r)))?;
        for (l, vec_ref_mut) in self.allocator[level..alloc_level].iter_mut().enumerate() {
            vec_ref_mut.push(start_offset + (1 << (l + level)));
        }
        self.max_allocated = self.max_allocated.max(u16::try_from((start_offset as usize + size + (size_of::<usize>() - 1)) & !(size_of::<usize>() - 1)).ok()?);
        Some(start_offset)
    }

    fn free(&mut self, size: usize, reg: &RegisterInner<Self>) {
        let level = size.trailing_zeros().saturating_sub(3);
        self.allocator[level as usize].push(reg.0);
    }

    fn reserve_range(regs: Range<u16>) -> Rc<RefCell<Self>> {
        let mut allocator: [Vec<u16>; u16::BITS as usize] = Default::default();
        allocator[15].push(0);
        let mut this = Self { max_allocated: 0, allocator };
        let end = if regs.end < 1 { 0 } else { 1 << (u16::BITS - (regs.end - 1).leading_zeros()) };
        let reserve_reg = this.raw_alloc(end * size_of::<usize>());
        assert_eq!(reserve_reg, Some(0));
        Rc::new(RefCell::new(this))
    }

    fn raw_free(&mut self, size: usize, reg: u16) {
        let size = if size < 1 { size_of::<usize>() } else { 1 << (usize::BITS - (size - 1).leading_zeros()) };
        let level = size.div_euclid(size_of::<usize>()).trailing_zeros() as usize;
        self.allocator[level as usize].push(reg);
    }
}
#[derive(Getters, CopyGetters)]
pub struct LinearRegisterPool<const REGISTER_SIZE: usize> {
    #[getset(get_copy = "pub")]
    max_allocated: u16,
    #[getset(get = "pub")]
    free_registers: SmallVec<[u16; 32]>,
}

impl<const REGISTER_SIZE: usize> Default for LinearRegisterPool<REGISTER_SIZE> {
    fn default() -> Self {
        Self { max_allocated: 0, free_registers: SmallVec::new() }
    }
}
impl<const REGISTER_SIZE: usize> RegisterPool for LinearRegisterPool<REGISTER_SIZE> {
    fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self { max_allocated: 0, free_registers: SmallVec::new() }))
    }

    fn raw_alloc(&mut self, size: usize) -> Option<u16> {
        assert!(size <= REGISTER_SIZE);
        let reg;
        if let Some(free_register) = self.free_registers.pop() {
            reg = free_register;
        } else {
            reg = self.max_allocated;
            self.max_allocated += 1;
        }
        Some(reg)
    }

    fn free(&mut self, size: usize, reg: &RegisterInner<Self>) {
        assert!(size <= REGISTER_SIZE);
        self.free_registers.push(reg.0);
    }

    fn raw_free(&mut self, _size: usize, reg: u16) {
        self.free_registers.push(reg);
    }

    fn reserve_range(regs: Range<u16>) -> Rc<RefCell<Self>> {
        let mut this = Self { max_allocated: 0, free_registers: SmallVec::new() };
        this.max_allocated = this.max_allocated.max(regs.end);
        this.free_registers = this.free_registers.iter().filter(|free_register| !regs.contains(free_register)).copied().collect();
        Rc::new(RefCell::new(this))
    }
}
