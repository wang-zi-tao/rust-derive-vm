use std::{
    borrow::Borrow,
    convert::TryFrom,
    fmt::Debug,
    hash::Hash,
    mem,
    mem::{align_of, size_of, MaybeUninit},
    ops::Deref,
    ptr::{null_mut, NonNull},
    sync::{Arc, Mutex},
};

use failure::{format_err, Fallible};
use getset::{CopyGetters, Setters};
use hashbrown::HashSet;

use super::buffer::UnsafeBuffer;
use ghost_cell::{GhostCell, GhostToken};
#[derive(Debug, Clone)]
pub enum RelocationKind {
    I8Relative,
    I32Relative,
    Usize,
}
impl RelocationKind {
    pub fn is_relative(&self) -> bool {
        match self {
            RelocationKind::I8Relative | RelocationKind::I32Relative => true,
            RelocationKind::Usize => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Relocation {
    offset: usize,
    relocation_kind: RelocationKind,
}
impl Relocation {
    pub fn relocate(&self, buffer: &mut UnsafeBuffer, arg: *const u8) {
        let offset = self.offset as usize;
        let arg = arg as isize;
        unsafe {
            let ptr = buffer.get_ptr::<u8>(offset).as_ptr() as isize;
            match self.relocation_kind {
                RelocationKind::I8Relative => buffer.get_ptr::<i8>(offset).as_ptr().write_volatile(i8::try_from(arg - ptr).unwrap()),
                RelocationKind::I32Relative => buffer.get_ptr::<i32>(offset).as_ptr().write_volatile(i32::try_from(arg - ptr).unwrap()),
                RelocationKind::Usize => buffer.get_ptr::<usize>(offset).as_ptr().write_volatile(usize::try_from(arg).unwrap()),
            }
        }
    }
}
#[derive(Debug, Clone, Builder)]
pub struct Symbol<T: Clone + Hash + Eq> {
    offset: usize,
    relocation_kind: RelocationKind,
    #[builder(default)]
    usage: HashSet<T>,
}
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct SymbolRef {
    object: ObjectRef,
    index: usize,
}

impl SymbolRef {
    pub fn new() -> Self {
        Self { object: ObjectRef::new(), index: 0 }
    }
}
#[repr(C)]
pub struct UnsafeSymbolRef {
    ptr: *mut u8,
    object: ObjectRef,
    index: usize,
}

impl UnsafeSymbolRef {
    pub unsafe fn new_uninited(object: ObjectRef, index: usize) -> Self {
        Self { ptr: null_mut(), object, index }
    }

    pub unsafe fn init(&mut self) {
        let self_ptr = NonNull::new_unchecked(self as *mut Self);
        let mut object = self.object.lock().unwrap();
        self.ptr = object.get_export_ptr(self.index);
        object.add_unsafe_symbol_ref(self_ptr, self.index);
    }

    pub unsafe fn kill(&mut self) {
        if !self.ptr.is_null() {
            let self_ptr = NonNull::new_unchecked(self as *mut Self);
            let mut object = self.object.lock().unwrap();
            object.remove_unsafe_symbol_ref(self_ptr);
        }
    }

    pub unsafe fn as_ptr(&self) -> *mut u8 {
        self.ptr
    }
}
unsafe impl Send for UnsafeSymbolRef {}
unsafe impl Sync for UnsafeSymbolRef {}
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct ReflexiveSymbolRef(Option<ObjectRef>, usize);
impl Debug for ReflexiveSymbolRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ReflexiveSymbolRef").field(&self.0.as_ref().map(|o| o as *const _)).field(&self.1).finish()
    }
}
/// 自动链接对象
/// 可以包含多个导出符号
#[derive(Debug)]
pub struct Object {
    buffer: UnsafeBuffer,
    symbols: Vec<Symbol<ReflexiveSymbolRef>>,
    relocations: Vec<(Relocation, ReflexiveSymbolRef)>,
    unsafe_symbol_refs: Vec<(NonNull<UnsafeSymbolRef>, usize)>,
    pin: bool,
}
impl Object {
    pub fn new() -> Self {
        Self { buffer: UnsafeBuffer::new(), relocations: Vec::new(), symbols: Vec::new(), pin: false, unsafe_symbol_refs: Vec::new() }
    }

    pub fn from_byes(bytes: &[u8]) -> ObjectRef {
        let mut this = Self::new();
        unsafe {
            this.buffer.push_slice(bytes);
        }
        ObjectRef(Arc::new(Mutex::new(this)))
    }

    pub fn get_buffer(&self) -> &UnsafeBuffer {
        &self.buffer
    }

    pub fn replace(
        &mut self,
        buffer: UnsafeBuffer,
        symbols: Vec<Symbol<ReflexiveSymbolRef>>,
        relocations: Vec<(Relocation, ReflexiveSymbolRef)>,
    ) -> Fallible<(UnsafeBuffer, Vec<Symbol<ReflexiveSymbolRef>>, Vec<(Relocation, ReflexiveSymbolRef)>)> {
        if self.pin {
            return Err(format_err!("The Object is pined"));
        }
        let old_buffer = mem::replace(&mut self.buffer, buffer);
        let old_symbols = mem::replace(&mut self.symbols, symbols);
        let old_relocations = mem::replace(&mut self.relocations, relocations);
        for (_relocation_index, (relocation, ReflexiveSymbolRef(source, symbol_index))) in old_relocations.iter().enumerate() {
            unsafe {
                let value = if let Some(source) = &source {
                    let source = source.lock().unwrap();
                    source.get_export_ptr(*symbol_index)
                } else {
                    self.get_export_ptr(*symbol_index)
                };
                relocation.relocate(&mut self.buffer, value);
            }
        }
        for (symbol_index, symbol) in old_symbols.iter().enumerate() {
            for ReflexiveSymbolRef(usage, relocate_index) in &symbol.usage {
                unsafe {
                    let value = self.get_export_ptr(symbol_index);
                    {
                        if let Some(usage) = usage.as_ref() {
                            let mut usage = usage.lock().unwrap();
                            usage.update_import(*relocate_index, value);
                        } else {
                            self.update_import(*relocate_index, value);
                        }
                    }
                }
            }
        }
        Ok((old_buffer, old_symbols, old_relocations))
    }

    pub fn update_import(&mut self, index: usize, data: *const u8) {
        let (relocation, _from) = &self.relocations[index];
        relocation.relocate(&mut self.buffer, data);
    }

    pub unsafe fn get_export_ptr(&self, index: usize) -> *mut u8 {
        let offset = self.symbols[index].offset;
        self.buffer.get_ptr(offset).as_ptr()
    }

    unsafe fn add_export_record(&mut self, index: usize, target: ReflexiveSymbolRef) {
        self.symbols[index].usage.insert(target);
    }

    unsafe fn add_import_record(&mut self, relocation: Relocation, source: ReflexiveSymbolRef, value: *mut u8) -> Fallible<()> {
        relocation.relocate(&mut self.buffer, value);
        self.relocations.push((relocation, source));
        Ok(())
    }

    pub unsafe fn add_export(&mut self, index: usize, relocation: Relocation, target: ReflexiveSymbolRef) -> Fallible<()> {
        let value = self.get_export_ptr(index);
        if let Some(target_inner) = target.0.as_ref() {
            let mut target_locked = target_inner.lock().unwrap();
            target_locked.add_import_record(relocation, target.clone(), value)?;
        } else {
            self.add_import_record(relocation, target.clone(), value)?;
        }
        self.add_export_record(index, target);
        Ok(())
    }

    pub unsafe fn add_unsafe_symbol_ref(&mut self, ptr: NonNull<UnsafeSymbolRef>, index: usize) {
        self.unsafe_symbol_refs.push((ptr, index))
    }

    pub unsafe fn remove_unsafe_symbol_ref(&mut self, ptr: NonNull<UnsafeSymbolRef>) {
        self.unsafe_symbol_refs.retain(|(p, _)| p != &ptr);
    }
}
unsafe impl Send for Object {}
unsafe impl Sync for Object {}
#[derive(Clone)]
pub struct ObjectRef(Arc<Mutex<Object>>);
impl Debug for ObjectRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.lock().fmt(f)
    }
}
impl ObjectRef {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(Object::new())))
    }
}
impl Deref for ObjectRef {
    type Target = Arc<Mutex<Object>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl PartialEq for ObjectRef {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0.as_ref(), other.0.as_ref())
    }
}
impl Eq for ObjectRef {}
impl Hash for ObjectRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (self.0.as_ref() as *const Mutex<Object>).hash(state);
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum ObjectBuilderImport<'l> {
    Builder(ObjectBuilder<'l>),
    ObjectRef(ObjectRef),
    Reflexive,
}
impl<'l> Debug for ObjectBuilderImport<'l> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjectBuilderImport::Builder(o) => f.debug_tuple("Builder").field(&(&*o.0 as *const GhostCell<_>)).finish(),
            ObjectBuilderImport::ObjectRef(o) => f.debug_tuple("ObjectRef").field(&(&*o.0 as *const Mutex<_>)).finish(),
            ObjectBuilderImport::Reflexive => f.write_str("Reflexive"),
        }
    }
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum ObjectBuilderExport<'l> {
    Builder(ObjectBuilder<'l>),
    Reflexive,
}
impl<'l> Debug for ObjectBuilderExport<'l> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjectBuilderExport::Builder(o) => f.debug_tuple("Builder").field(&(&*o.0 as *const GhostCell<_>)).finish(),
            ObjectBuilderExport::Reflexive => f.write_str("Reflexive"),
        }
    }
}

#[derive(Clone)]
pub struct ObjectBuilder<'b>(Arc<GhostCell<'b, ObjectBuilderInner<'b>>>);

impl<'b> ObjectBuilder<'b> {
    pub fn merge(token: &mut GhostToken<'b>, builder1: ObjectBuilder<'b>, builder2: ObjectBuilder<'b>) -> ObjectBuilder<'b> {
        let b2 = builder2.take(token);
        let b1 = builder1.deref().deref().borrow_mut(token);
        b1.align(b2.get_align());
        b1.set_align(usize::max(b1.get_align(), b2.get_align()));
        let offset = b1.len();
        let symbol_offset = b1.symbols.len();
        let relocation_offset = b1.relocations.len();
        let ObjectBuilderInner { buffer, relocations, symbols, pin: _, align: _ } = b2;
        b1.push_slice(unsafe { buffer.borrow() });
        b1.symbols.extend(symbols);
        b1.relocations.extend(relocations);
        let mut redirect_exports_tasks = Vec::new();
        for (symbol_index, symbol) in b1.symbols.iter_mut().enumerate().take(symbol_offset) {
            for (mut usage, mut relocation_index) in symbol
                .usage
                .drain_filter(|(usage_symbol, _relocation_index)| !matches!(usage_symbol,ObjectBuilderExport::Builder(b)if b==&builder2))
                .collect::<Vec<_>>()
            {
                match &usage {
                    ObjectBuilderExport::Builder(b) if b == &builder2 => {
                        usage = ObjectBuilderExport::Reflexive;
                        relocation_index += relocation_offset;
                    }
                    _ => unreachable!(),
                };
                symbol.usage.insert((usage, relocation_index));
            }
        }
        for (symbol_index, symbol) in b1.symbols.iter_mut().enumerate().skip(symbol_offset) {
            symbol.offset += offset;
            for (mut usage, mut relocation_index) in symbol
                .usage
                .drain_filter(|(usage_symbol, _relocation_index)| !matches!(usage_symbol,ObjectBuilderExport::Builder(b)if b==&builder1 || b==&builder2))
                .collect::<Vec<_>>()
            {
                match &usage {
                    ObjectBuilderExport::Builder(b) if b == &builder1 => {
                        usage = ObjectBuilderExport::Reflexive;
                    }
                    ObjectBuilderExport::Builder(b) if b == &builder2 => {
                        usage = ObjectBuilderExport::Reflexive;
                        relocation_index += relocation_offset;
                    }
                    ObjectBuilderExport::Reflexive => {
                        relocation_index += relocation_offset;
                    }
                    _ => unreachable!(),
                };
                symbol.usage.insert((usage, relocation_index));
            }
            for (usage, relocation_index) in &symbol.usage {
                redirect_exports_tasks.push((usage.clone(), symbol_index, *relocation_index));
            }
        }
        let mut relocation_imports_tasks = Vec::new();
        for (_relocation_index, (source, _relocation, symbol_index)) in b1.relocations.iter_mut().enumerate().take(relocation_offset) {
            match (&source, symbol_index) {
                (ObjectBuilderImport::Builder(b), symbol_index) if b == &builder2 => {
                    *source = ObjectBuilderImport::Reflexive;
                    *symbol_index += symbol_offset;
                }
                _ => {}
            };
        }
        for (relocation_index, (source, relocation, symbol_index)) in b1.relocations.iter_mut().enumerate().skip(symbol_offset) {
            match source {
                ObjectBuilderImport::Builder(b) if b == &builder1 => {
                    *source = ObjectBuilderImport::Reflexive;
                }
                ObjectBuilderImport::Builder(b) if b == &builder2 => {
                    *source = ObjectBuilderImport::Reflexive;
                    *symbol_index += symbol_offset;
                }
                _ => {}
            };
            relocation.offset += offset;
            relocation_imports_tasks.push((source.clone(), *symbol_index, relocation_index));
        }
        let b1 = builder1.deref().deref();
        for (source, symbol_index, relocation_index) in relocation_imports_tasks {
            match source {
                ObjectBuilderImport::Builder(source) => unsafe {
                    let source_inner = source.deref().deref().borrow(token);
                    let value = source_inner.get_export_ptr(symbol_index);
                    b1.borrow(token).relocations[relocation_index].1.clone().relocate(&mut b1.borrow_mut(token).buffer, value);
                    b1.borrow_mut(token).relocations[relocation_index].1.offset += offset;
                },
                ObjectBuilderImport::ObjectRef(source) => unsafe {
                    let source = source.lock().unwrap();
                    let value = source.get_export_ptr(symbol_index);
                    b1.deref().deref().borrow(token).relocations[relocation_index].1.clone().relocate(&mut b1.borrow_mut(token).buffer, value);
                },
                _ => {}
            }
        }
        for (target, relocation_index, symbol_index) in redirect_exports_tasks {
            unsafe {
                match target {
                    ObjectBuilderExport::Builder(target) => {
                        let value = b1.deref().deref().borrow(token).get_export_ptr(symbol_index);
                        target.borrow_mut(token).relocations[relocation_index].1.clone().relocate(&mut b1.borrow_mut(token).buffer, value);
                    }
                    ObjectBuilderExport::Reflexive => {
                        let value = b1.deref().deref().borrow(token).get_export_ptr(symbol_index);
                        b1.borrow_mut(token).relocations[relocation_index].1.clone().relocate(&mut b1.borrow_mut(token).buffer, value);
                    }
                }
            }
            b1.borrow_mut(token).symbols[symbol_index].offset += offset;
        }
        builder1
    }
}
impl<'b> From<ObjectBuilderInner<'b>> for ObjectBuilder<'b> {
    fn from(f: ObjectBuilderInner<'b>) -> Self {
        Self(Arc::new(GhostCell::new(f)))
    }
}
impl<'l> Default for ObjectBuilder<'l> {
    fn default() -> Self {
        Self(Arc::new(GhostCell::new(Default::default())))
    }
}
impl<'b> Deref for ObjectBuilder<'b> {
    type Target = Arc<GhostCell<'b, ObjectBuilderInner<'b>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<'b> PartialEq for ObjectBuilder<'b> {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_ptr() == other.0.as_ptr()
    }
}
impl<'b> Eq for ObjectBuilder<'b> {}
impl<'b> Hash for ObjectBuilder<'b> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (self.0.as_ptr()).hash(state);
    }
}
impl<'b> Debug for ObjectBuilder<'b> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ObjectBuilder").field(&(&*self.0 as *const GhostCell<_>)).finish()
    }
}

#[derive(Debug, CopyGetters, Setters)]
pub struct ObjectBuilderInner<'l> {
    buffer: UnsafeBuffer,
    relocations: Vec<(ObjectBuilderImport<'l>, Relocation, usize)>,
    symbols: Vec<Symbol<(ObjectBuilderExport<'l>, usize)>>,
    #[getset(get_copy = "pub with_prefix", set = "pub with_prefix")]
    pin: bool,
    #[getset(get_copy = "pub with_prefix", set = "pub with_prefix")]
    align: usize,
}
impl<'l> Default for ObjectBuilderInner<'l> {
    fn default() -> Self {
        Self::new(UnsafeBuffer::new())
    }
}
impl<'l> ObjectBuilderInner<'l> {
    pub fn new(buffer: UnsafeBuffer) -> Self {
        Self { buffer, symbols: Vec::new(), relocations: Vec::new(), pin: false, align: 8 }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self { buffer: UnsafeBuffer::with_capacity(capacity), ..Default::default() }
    }

    pub fn push_import(&mut self, import: ObjectBuilderImport<'l>, relocation_kind: RelocationKind, symbol_index: usize) {
        self.relocations.push((import, Relocation { offset: self.len(), relocation_kind }, symbol_index));
        self.receive::<i32>();
    }

    pub fn add_symbol(&mut self, symbol: Symbol<(ObjectBuilderExport<'l>, usize)>) -> usize {
        let index = self.symbols.len();
        self.symbols.push(symbol);
        index
    }

    pub fn set_import(&mut self, offset: usize, import: ObjectRef, relocation_kind: RelocationKind, symbol_index: usize) {
        self.relocations.push((ObjectBuilderImport::ObjectRef(import), Relocation { offset, relocation_kind }, symbol_index));
    }

    pub fn build(self) -> Fallible<ObjectRef> {
        let symbols =
            self.symbols.into_iter().map(|symbol| Symbol { offset: symbol.offset, relocation_kind: symbol.relocation_kind, usage: HashSet::new() }).collect();
        let pool = Arc::new(Mutex::new(Object { buffer: self.buffer, relocations: Vec::new(), symbols, pin: self.pin, unsafe_symbol_refs: Vec::new() }));
        for (relocation_index, (source, relocation, symbol_index)) in self.relocations.into_iter().enumerate() {
            match source {
                ObjectBuilderImport::ObjectRef(object) => unsafe {
                    object.lock().unwrap().add_export(symbol_index, relocation, ReflexiveSymbolRef(Some(ObjectRef(pool.clone())), relocation_index))?;
                },
                ObjectBuilderImport::Reflexive => unsafe {
                    pool.lock().unwrap().add_export(symbol_index, relocation, ReflexiveSymbolRef(None, relocation_index))?;
                },
                o => return Err(format_err!("can not build object with import {:?}", o)),
            };
        }
        Ok(ObjectRef(pool))
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    pub fn grow(&mut self, new_size: usize) {
        unsafe { self.buffer.grow(new_size) }
    }

    pub fn borrow_mut(&mut self) -> &mut [u8] {
        unsafe { self.buffer.borrow_mut() }
    }

    pub fn borrow(&self) -> &[u8] {
        unsafe { self.buffer.borrow() }
    }

    pub fn push_slice(&mut self, value: &[u8]) {
        unsafe { self.buffer.push_slice(value) }
    }

    pub fn set_slice(&mut self, start: usize, value: &[u8]) {
        unsafe { self.buffer.set_slice(start, value) }
    }

    pub fn set<T: Copy>(&mut self, offset: usize, value: T) {
        assert!(offset + size_of::<T>() <= self.len());
        unsafe { self.buffer.get_ptr::<T>(offset).as_ptr().write(value) }
    }

    pub fn get<T: Copy>(&self, offset: usize) -> T {
        if offset + size_of::<T>() >= self.len() {
            panic!();
        }
        unsafe { self.buffer.get_ptr::<T>(offset).as_ptr().read() }
    }

    pub fn try_get<T: Copy>(&self, offset: usize) -> Option<T> {
        if offset + size_of::<T>() >= self.len() {
            None
        } else {
            Some(unsafe { self.buffer.get_ptr::<T>(offset).as_ptr().read() })
        }
    }

    pub fn push<T: Copy>(&mut self, value: T) {
        unsafe { self.buffer.push(value) }
    }

    pub fn align(&mut self, align: usize) {
        unsafe { self.buffer.align(align) }
    }

    pub unsafe fn get_export_ptr(&self, index: usize) -> *mut u8 {
        let offset = self.symbols[index].offset;
        self.buffer.get_ptr(offset).as_ptr()
    }

    pub fn receive<T>(&mut self) -> &mut MaybeUninit<T> {
        self.align(align_of::<T>());
        let offset = self.len();
        self.receive_at(offset)
    }

    pub fn receive_at<T>(&mut self, offset: usize) -> &mut MaybeUninit<T> {
        unsafe {
            for _ in 0..size_of::<T>() {
                self.push(0u8);
            }
            let ptr: NonNull<u8> = self.buffer.get_ptr(offset);
            ptr.cast().as_mut()
        }
    }
}

pub trait MoveIntoObject {
    fn set(self, offset: usize, object_builder: &mut ObjectBuilderInner);
    fn append(self, object_builder: &mut ObjectBuilderInner)
    where
        Self: Sized,
    {
        let offset = object_builder.len();
        object_builder.grow(offset + size_of::<Self>());
        self.set(offset, object_builder)
    }
}
