use std::{
    convert::TryFrom,
    fmt::Debug,
    hash::Hash,
    mem,
    mem::{align_of, size_of, MaybeUninit},
    ops::Deref,
    ptr::{null_mut, slice_from_raw_parts_mut, NonNull},
    sync::{Arc, Mutex, Weak},
};

use super::buffer::UnsafeBuffer;
use arc_swap::{ArcSwap, ArcSwapWeak};
use failure::{format_err, Fallible};
use getset::{CopyGetters, Getters, Setters};
use ghost_cell::{GhostCell, GhostToken};
use hashbrown::HashSet;
#[derive(Debug, Clone)]
pub enum RelocationKind {
    I8Relative,
    I32Relative,
    UsizePtrAbsolute,
}
impl RelocationKind {
    pub fn is_relative(&self) -> bool {
        match self {
            RelocationKind::I8Relative | RelocationKind::I32Relative => true,
            RelocationKind::UsizePtrAbsolute => false,
        }
    }

    pub fn size(&self) -> usize {
        match self {
            RelocationKind::I8Relative => 1,
            RelocationKind::I32Relative => 4,
            RelocationKind::UsizePtrAbsolute => size_of::<usize>(),
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
                RelocationKind::UsizePtrAbsolute => buffer.get_ptr::<usize>(offset).as_ptr().write_volatile(usize::try_from(arg).unwrap()),
            }
        }
    }
}
#[derive(Debug, Clone)]
pub enum SymbolKind {
    Ptr,
    Value,
}

impl Default for SymbolKind {
    fn default() -> Self {
        Self::Ptr
    }
}

#[derive(Debug, Clone, Builder)]
pub struct Symbol<T: Clone + Hash + Eq> {
    offset: usize,
    #[builder(default)]
    symbol_kind: SymbolKind,
    #[builder(default)]
    usage: HashSet<T>,
}
#[derive(Clone, Debug, Hash, PartialEq, Eq, Default, Getters, CopyGetters)]
pub struct SymbolRef {
    #[getset(get = "pub")]
    pub object: ObjectRef,
    #[getset(get_copy = "pub")]
    pub index: usize,
}

impl SymbolRef {
    pub fn new(object: ObjectRef, index: usize) -> Self {
        Self { object, index }
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
pub struct ObjectImport(Option<ObjectRef>, usize);
impl Debug for ObjectImport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ObjectImport").field(&self.0.as_ref().map(|o| o as *const _)).field(&self.1).finish()
    }
}
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct ObjectExport(Option<ObjectWeekRef>, usize);
impl Debug for ObjectExport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ObjectExport").field(&self.0.as_ref().map(|o| o as *const _)).field(&self.1).finish()
    }
}
/// 自动链接对象
/// 可以包含多个导出符号
#[derive(Debug)]
pub struct Object {
    buffer: UnsafeBuffer,
    symbols: Vec<Symbol<ObjectExport>>,
    relocations: Vec<(Relocation, ObjectImport)>,
    unsafe_symbol_refs: Vec<(NonNull<UnsafeSymbolRef>, usize)>,
    pin: bool,
}

impl Default for Object {
    fn default() -> Self {
        Self::new()
    }
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
        this: &ObjectRef, mut buffer: UnsafeBuffer, symbols: Vec<Symbol<ObjectExport>>, relocations: Vec<(Relocation, ObjectImport)>,
    ) -> Fallible<(UnsafeBuffer, Vec<Symbol<ObjectExport>>, Vec<(Relocation, ObjectImport)>)> {
        let mut this_guard = this.lock().unwrap();
        if this_guard.pin {
            return Err(format_err!("The Object is pined"));
        }
        for (_relocation_index, (relocation, ObjectImport(source, symbol_index))) in relocations.iter().enumerate() {
            if let Some(source) = &source {
                let source = source.lock().unwrap();
                let value = source.get_export_ptr(*symbol_index);
                relocation.relocate(&mut buffer, value);
            } else {
                let index = *symbol_index;
                let symbol = &symbols[index];
                let ptr: *mut u8 = buffer.get_ptr(symbol.offset).as_ptr();
                let value = match symbol.symbol_kind {
                    SymbolKind::Ptr => ptr,
                    SymbolKind::Value => unsafe { ptr.cast::<*mut u8>().read() },
                };
                relocation.relocate(&mut buffer, value);
            };
        }
        let old_buffer = mem::replace(&mut this_guard.buffer, buffer);
        let old_symbols = mem::replace(&mut this_guard.symbols, symbols);
        let old_relocations = mem::replace(&mut this_guard.relocations, relocations);
        for (relocation_index, (_relocation, ObjectImport(source, symbol_index))) in old_relocations.iter().enumerate() {
            if let Some(source) = &source {
                let mut source = source.lock().unwrap();
                source.remove_usage(*symbol_index, &ObjectExport(Some(this.downgrade()), relocation_index));
            };
        }
        for (symbol_index, symbol) in old_symbols.iter().enumerate() {
            for ObjectExport(usage, relocate_index) in &symbol.usage {
                let value = this_guard.get_export_ptr(symbol_index);
                {
                    if let Some(usage) = usage.as_ref() {
                        if let Some(usage) = usage.upgrade() {
                            let mut usage = usage.lock().unwrap();
                            usage.update_import(*relocate_index, value);
                        }
                    } else {
                        this_guard.update_import(*relocate_index, value);
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

    pub fn remove_usage(&mut self, symbol_index: usize, target: &ObjectExport) {
        self.symbols[symbol_index].usage.remove(target);
    }

    pub fn get_export_ptr(&self, index: usize) -> *mut u8 {
        let symbol = &self.symbols[index];
        let ptr = self.buffer.get_ptr(symbol.offset).as_ptr();
        match symbol.symbol_kind {
            SymbolKind::Ptr => ptr,
            SymbolKind::Value => unsafe { ptr.cast::<*mut u8>().read() },
        }
    }

    unsafe fn add_export_record(&mut self, index: usize, target: ObjectExport) {
        self.symbols[index].usage.insert(target);
    }

    unsafe fn add_import_record(&mut self, relocation: Relocation, source: ObjectImport, value: *mut u8) -> Fallible<()> {
        relocation.relocate(&mut self.buffer, value);
        self.relocations.push((relocation, source));
        Ok(())
    }

    pub unsafe fn add_export(&mut self, index: usize, relocation: Relocation, target: ObjectImport) -> Fallible<()> {
        let value = self.get_export_ptr(index);
        if let Some(target_inner) = target.0.as_ref() {
            let mut target_locked = target_inner.lock().unwrap();
            target_locked.add_import_record(relocation, target.clone(), value)?;
        } else {
            self.add_import_record(relocation, target.clone(), value)?;
        }
        self.add_export_record(index, target.downgrade());
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
pub type LockedObjectInner = Mutex<Object>;
pub struct AtomicObjectWeekRef(pub ArcSwapWeak<Mutex<Object>>);

impl std::ops::Deref for AtomicObjectWeekRef {
    type Target = ArcSwapWeak<Mutex<Object>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AtomicObjectWeekRef {
    pub fn upgrade(&self) -> Option<AtomicObjectRef> {
        self.0.load().upgrade().map(|r| AtomicObjectRef(r.into()))
    }
}
impl Into<ObjectWeekRef> for AtomicObjectWeekRef {
    fn into(self) -> ObjectWeekRef {
        ObjectWeekRef(self.0.into_inner())
    }
}

impl Clone for AtomicObjectWeekRef {
    fn clone(&self) -> Self {
        Self(self.0.load().clone().into())
    }
}

impl Default for AtomicObjectWeekRef {
    fn default() -> Self {
        Self(ArcSwapWeak::from(Weak::default()))
    }
}
#[derive(Default, Debug)]
pub struct AtomicObjectRef(pub ArcSwap<Mutex<Object>>);

impl std::ops::Deref for AtomicObjectRef {
    type Target = ArcSwap<Mutex<Object>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Clone for AtomicObjectRef {
    fn clone(&self) -> Self {
        Self(self.0.load().clone().into())
    }
}
impl Into<ObjectRef> for AtomicObjectRef {
    fn into(self) -> ObjectRef {
        ObjectRef(self.0.into_inner())
    }
}
#[derive(Clone, Default)]
pub struct ObjectWeekRef(pub Weak<Mutex<Object>>);

impl Hash for ObjectWeekRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.as_ptr().hash(state);
    }
}

impl PartialEq for ObjectWeekRef {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for ObjectWeekRef {}

impl std::ops::Deref for ObjectWeekRef {
    type Target = Weak<Mutex<Object>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[derive(Clone, Default)]
pub struct ObjectRef(pub Arc<Mutex<Object>>);
impl Debug for ObjectRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.lock().fmt(f)
    }
}
impl ObjectRef {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(Object::new())))
    }

    fn downgrade(&self) -> ObjectWeekRef {
        ObjectWeekRef(Arc::downgrade(&self.0))
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

impl<'l> From<ObjectRef> for ObjectBuilderImport<'l> {
    fn from(i: ObjectRef) -> Self {
        Self::ObjectRef(i)
    }
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
        let ObjectBuilderInner { buffer: b2_buffer, relocations, symbols, pin: _, align: _ } = b2;
        b1.push_slice(unsafe { b2_buffer.borrow() });
        b1.symbols.extend(symbols);
        b1.relocations.extend(relocations);
        for (_symbol_index, symbol) in b1.symbols.iter_mut().enumerate().take(symbol_offset) {
            for (mut usage, mut relocation_index) in symbol
                .usage
                .drain_filter(|(usage_symbol, _relocation_index)| matches!(usage_symbol,ObjectBuilderExport::Builder(b)if b==&builder2))
                .collect::<Vec<_>>()
            {
                match &usage {
                    ObjectBuilderExport::Builder(b) if b == &builder2 => {
                        usage = ObjectBuilderExport::Reflexive;
                        relocation_index += relocation_offset;
                    }
                    _ => unreachable!(),
                };
                let not_already_exist = symbol.usage.insert((usage, relocation_index));
                assert!(not_already_exist);
            }
        }
        let mut redirect_exports_tasks = Vec::new();
        for (symbol_index, symbol) in b1.symbols.iter_mut().enumerate().skip(symbol_offset) {
            symbol.offset += offset;
            for (mut usage, mut relocation_index) in symbol
                .usage
                .drain_filter(|(usage_symbol, _relocation_index)| match usage_symbol {
                    ObjectBuilderExport::Reflexive => true,
                    ObjectBuilderExport::Builder(b) if b == &builder1 || b == &builder2 => true,
                    _ => false,
                })
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
                let not_already_exist = symbol.usage.insert((usage, relocation_index));
                assert!(not_already_exist);
            }
            for (usage, relocation_index) in &symbol.usage {
                redirect_exports_tasks.push((usage.clone(), symbol_index, *relocation_index));
            }
        }
        for (_relocation_index, (source, _relocation, symbol_index)) in b1.relocations.iter_mut().enumerate().take(relocation_offset) {
            match (&source, symbol_index) {
                (ObjectBuilderImport::Builder(b), symbol_index) if b == &builder2 => {
                    *source = ObjectBuilderImport::Reflexive;
                    *symbol_index += symbol_offset;
                }
                _ => {}
            };
        }
        let mut relocation_imports_tasks = Vec::new();
        for (relocation_index, (source, relocation, symbol_index)) in b1.relocations.iter_mut().enumerate().skip(relocation_offset) {
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
                ObjectBuilderImport::Builder(source) => {
                    let source_inner = source.deref().deref().borrow(token);
                    let value = source_inner.get_export_ptr(symbol_index);
                    b1.borrow_mut(token).relocate(relocation_index, (ObjectBuilderImport::Reflexive, symbol_index), value);
                    {
                        let usage = &mut source.borrow_mut(token).symbols[symbol_index].usage;
                        let already_exist = usage.remove(&(ObjectBuilderExport::Builder(builder2.clone()), relocation_index - relocation_offset));
                        assert!(already_exist, "{:?} remove {:?}", usage, (builder2, relocation_index - relocation_offset));
                        let not_already_exist = usage.insert((ObjectBuilderExport::Builder(builder1.clone()), relocation_index));
                        assert!(not_already_exist, "{:?} insert {:?}", usage, (builder1, relocation_index));
                    }
                }
                ObjectBuilderImport::ObjectRef(source) => {
                    let value = {
                        let source_guard = source.lock().unwrap();
                        source_guard.get_export_ptr(symbol_index)
                    };
                    b1.borrow_mut(token).relocate(relocation_index, (ObjectBuilderImport::ObjectRef(source), symbol_index), value);
                }
                ObjectBuilderImport::Reflexive => {
                    let value = b1.borrow(token).get_export_ptr(symbol_index);
                    b1.borrow_mut(token).relocate(relocation_index, (ObjectBuilderImport::Reflexive, symbol_index), value);
                }
            }
        }
        for (target, symbol_index, relocation_index) in redirect_exports_tasks {
            match target {
                ObjectBuilderExport::Builder(target) => {
                    let value = b1.deref().deref().borrow(token).get_export_ptr(symbol_index);
                    target.borrow_mut(token).relocate(relocation_index, (ObjectBuilderImport::Builder(builder1.clone()), symbol_index), value);
                }
                ObjectBuilderExport::Reflexive => {
                    let value = b1.deref().deref().borrow(token).get_export_ptr(symbol_index);
                    b1.borrow_mut(token).relocate(relocation_index, (ObjectBuilderImport::Reflexive, symbol_index), value);
                }
            }
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

#[derive(CopyGetters, Getters, Setters)]
pub struct ObjectBuilderInner<'l> {
    #[getset(get = "pub with_prefix")]
    buffer: UnsafeBuffer,
    #[getset(get = "pub with_prefix")]
    relocations: Vec<(ObjectBuilderImport<'l>, Relocation, usize)>,
    #[getset(get = "pub with_prefix")]
    symbols: Vec<Symbol<(ObjectBuilderExport<'l>, usize)>>,
    #[getset(get_copy = "pub with_prefix", set = "pub with_prefix")]
    pin: bool,
    #[getset(get_copy = "pub with_prefix", set = "pub with_prefix")]
    align: usize,
}

impl<'l> Debug for ObjectBuilderInner<'l> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&format!("ObjectBuilderInner@{:?}", self as *const Self))
            .field("buffer", &self.buffer)
            .field("symbols", &self.symbols)
            .field("relocations", &self.relocations)
            .field("pin", &self.pin)
            .field("align", &self.align)
            .finish()
    }
}
impl<'l> Default for ObjectBuilderInner<'l> {
    fn default() -> Self {
        Self::new(UnsafeBuffer::new())
    }
}
impl<'l> ObjectBuilderInner<'l> {
    pub fn new(buffer: UnsafeBuffer) -> Self {
        Self { buffer, symbols: Vec::new(), relocations: Vec::new(), pin: false, align: 1 }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self { buffer: UnsafeBuffer::with_capacity(capacity), ..Default::default() }
    }

    pub fn push_import(
        this: &ObjectBuilder<'l>, token: &mut GhostToken<'l>, import: ObjectBuilderImport<'l>, relocation_kind: RelocationKind, symbol_index: usize,
    ) -> usize {
        let relocation_index = this.borrow(token).relocations.len();
        let offset = this.borrow(token).len();
        match &import {
            ObjectBuilderImport::Builder(source) => {
                source.borrow_mut(token).symbols[symbol_index].usage.insert((ObjectBuilderExport::Builder(this.clone()), relocation_index));
            }
            ObjectBuilderImport::ObjectRef(_) => {}
            ObjectBuilderImport::Reflexive => {
                this.borrow_mut(token).symbols[symbol_index].usage.insert((ObjectBuilderExport::Reflexive, relocation_index));
            }
        }
        match relocation_kind {
            RelocationKind::I8Relative => {
                this.borrow_mut(token).receive::<i8>();
            }
            RelocationKind::I32Relative => {
                this.borrow_mut(token).receive::<i32>();
            }
            RelocationKind::UsizePtrAbsolute => {
                this.borrow_mut(token).receive::<usize>();
            }
        }
        this.borrow_mut(token).relocations.push((import, Relocation { offset, relocation_kind }, symbol_index));
        offset
    }

    pub fn set_import(
        this: &ObjectBuilder<'l>, token: &mut GhostToken<'l>, offset: usize, import: ObjectBuilderImport<'l>, relocation_kind: RelocationKind,
        symbol_index: usize,
    ) {
        let relocation_index = this.borrow(token).relocations.len();
        match &import {
            ObjectBuilderImport::Builder(source) => {
                source.borrow_mut(token).symbols[symbol_index].usage.insert((ObjectBuilderExport::Builder(this.clone()), relocation_index));
            }
            ObjectBuilderImport::ObjectRef(_) => {}
            ObjectBuilderImport::Reflexive => {
                this.borrow_mut(token).symbols[symbol_index].usage.insert((ObjectBuilderExport::Reflexive, relocation_index));
            }
        }
        this.borrow_mut(token).relocations.push((import, Relocation { offset, relocation_kind }, symbol_index));
    }

    pub fn add_symbol(&mut self, symbol: Symbol<(ObjectBuilderExport<'l>, usize)>) -> usize {
        let index = self.symbols.len();
        self.symbols.push(symbol);
        index
    }

    fn relocate(&mut self, relocation_index: usize, source: (ObjectBuilderImport<'l>, usize), ptr: *const u8) {
        let relocation = &mut self.relocations[relocation_index];
        relocation.1.relocate(&mut self.buffer, ptr);
        relocation.0 = source.0;
        relocation.2 = source.1;
    }

    pub fn build(self) -> Fallible<ObjectRef> {
        let symbols = self.symbols.into_iter().map(|symbol| Symbol { offset: symbol.offset, symbol_kind: symbol.symbol_kind, usage: HashSet::new() }).collect();
        let pool = Arc::new(Mutex::new(Object { buffer: self.buffer, relocations: Vec::new(), symbols, pin: self.pin, unsafe_symbol_refs: Vec::new() }));
        for (relocation_index, (source, relocation, symbol_index)) in self.relocations.into_iter().enumerate() {
            match source {
                ObjectBuilderImport::ObjectRef(object) => unsafe {
                    object.lock().unwrap().add_export(symbol_index, relocation, ObjectImport(Some(ObjectRef(pool.clone())), relocation_index))?;
                },
                ObjectBuilderImport::Reflexive => unsafe {
                    pool.lock().unwrap().add_export(symbol_index, relocation, ObjectImport(None, relocation_index))?;
                },
                o => return Err(format_err!("can not build object with import {:?}", o)),
            };
        }
        Ok(ObjectRef(pool))
    }

    pub fn build_into(self, output: ObjectRef) -> Fallible<ObjectRef> {
        let symbols = self.symbols.into_iter().map(|symbol| Symbol { offset: symbol.offset, symbol_kind: symbol.symbol_kind, usage: HashSet::new() }).collect();
        let mut reloations = Vec::with_capacity(self.relocations.len());
        for (relocation_index, (source, relocation, symbol_index)) in self.relocations.into_iter().enumerate() {
            match source {
                ObjectBuilderImport::ObjectRef(object) => {
                    reloations.push((relocation, ObjectImport(Some(object.clone()), symbol_index)));
                }
                ObjectBuilderImport::Reflexive => {
                    reloations.push((relocation, ObjectImport(None, relocation_index)));
                }
                o => return Err(format_err!("can not build object with import {:?}", o)),
            };
        }
        Object::replace(&output, self.buffer, symbols, reloations)?;
        Ok(output)
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
        self.align = self.align.max(align);
        unsafe { self.buffer.align(align) }
    }

    pub fn get_export_ptr(&self, index: usize) -> *mut u8 {
        let symbol = &self.symbols[index];
        let ptr = self.buffer.get_ptr(symbol.offset).as_ptr();
        match symbol.symbol_kind {
            SymbolKind::Ptr => ptr,
            SymbolKind::Value => unsafe { ptr.cast::<*mut u8>().read() },
        }
    }

    pub fn receive<T>(&mut self) -> &mut MaybeUninit<T> {
        self.align(align_of::<T>());
        let offset = self.len();
        self.receive_at(offset)
    }

    pub fn receive_slice<T: Sized>(&mut self, len: usize) -> &mut [MaybeUninit<T>] {
        self.align(align_of::<T>());
        let offset = self.len();
        self.receive_slice_at(offset, len)
    }

    pub fn receive_at<T>(&mut self, offset: usize) -> &mut MaybeUninit<T> {
        unsafe {
            self.buffer.reserve_at(offset, size_of::<T>());
            let ptr: NonNull<u8> = self.buffer.get_ptr(offset);
            ptr.cast().as_mut()
        }
    }

    pub fn receive_slice_at<T>(&mut self, offset: usize, len: usize) -> &mut [MaybeUninit<T>] {
        unsafe {
            self.buffer.reserve_at(offset, len * size_of::<T>());
            let ptr: NonNull<u8> = self.buffer.get_ptr(offset);
            NonNull::new_unchecked(slice_from_raw_parts_mut(ptr.cast().as_ptr(), len)).as_mut()
        }
    }
}
#[derive(Clone, Debug, Hash, PartialEq, Eq, Getters, CopyGetters)]
pub struct SymbolBuilderRef<'b> {
    #[getset(get = "pub")]
    object_builder: ObjectBuilder<'b>,
    #[getset(get_copy = "pub")]
    index: usize,
}

impl<'b> SymbolBuilderRef<'b> {
    pub fn new(object_builder: ObjectBuilder<'b>, index: usize) -> Self {
        Self { object_builder, index }
    }
}

pub trait MoveIntoObject<'l> {
    type Carrier;
    fn set(carrier: Self::Carrier, offset: usize, object_builder: &ObjectBuilder<'l>, token: &mut GhostToken<'l>);
    fn append(carrier: Self::Carrier, object_builder: &ObjectBuilder<'l>, token: &mut GhostToken<'l>)
    where
        Self: Sized,
    {
        let offset = object_builder.borrow(token).len();
        object_builder.borrow_mut(token).receive_slice::<u8>(size_of::<Self>());
        Self::set(carrier, offset, object_builder, token)
    }
}
impl ObjectImport {
    fn downgrade(&self) -> ObjectExport {
        ObjectExport(self.0.as_ref().map(|o| o.downgrade()), self.1)
    }
}
