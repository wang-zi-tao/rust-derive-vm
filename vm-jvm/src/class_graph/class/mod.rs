mod lifetime;
use crate::{
    annotations::AnnotatedElement,
    class_loader::ClassLoader,
    executable::Executable,
    field::Field,
    generate_type::{GenericType, PrimitiveType, TemporaryTypeTreeNode, TypeTreeNode},
    member::Member,
};
use classfile::{attributes::Attribute, ClassFile};
use failure::{format_err, Fallible};
pub use lifetime::*;
use std::ops::Deref;
use vm_core::*;

use std::{
    borrow::Borrow,
    cmp::Ordering,
    collections::HashMap,
    default::default,
    fmt::Debug,
    hash::{Hash, Hasher},
    sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, Weak},
};
use util::{AtomicCell, AtomicLazyArc, BKDRHash, PooledStr};

pub struct JavaClassUsage {
    array_class: Option<Arc<JavaClass>>,
    fields: Vec<Arc<JavaClass>>,
    executables_parameters: Vec<Arc<Executable>>,
    methods_return: Vec<Arc<Executable>>,
}
impl Hash for JavaClass {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash_code);
    }
}
#[derive(Debug)]
pub enum JavaClassKind {
    Primitive(PrimitiveType),
    ClassOrInterface,
    Array(Arc<JavaClass>),
}
#[derive(Debug)]
pub struct JavaClass {
    pub(crate) class_kind: JavaClassKind,
    pub(crate) hash_code: u64,
    pub(crate) name: PooledStr,
    pub(crate) binary_name: PooledStr,
    pub(crate) define_class_loader: Weak<ClassLoader>,
    pub(crate) component: AtomicLazyArc<JavaClassComponent>,
    pub(crate) process_lock: Mutex<()>,
    pub(crate) load_lock: Mutex<()>,
}
pub type JavaClassRef = Arc<JavaClass>;
impl JavaClass {
    // pub fn is_primitive(&self) -> bool {
    // self.modifiers().is_primitive()
    // }
    // pub fn get_component(&self) -> Fallible<&JavaClassComponent> {
    // match
    // }
}
pub fn class_name_to_binary_name(name: &str) -> PooledStr {
    todo!() // TODO
}
impl JavaClass {
    pub fn new(name: PooledStr, class_loader: &Arc<ClassLoader>) -> Self {
        let mut state = BKDRHash::default();
        name.hash(&mut state);
        class_loader.hash(&mut state);
        JavaClass {
            class_kind: JavaClassKind::ClassOrInterface,
            hash_code: state.finish(),
            binary_name: class_name_to_binary_name(&*name),
            name,
            define_class_loader: Arc::downgrade(class_loader),
            component: AtomicLazyArc::new_uninitalized(),
            process_lock: Mutex::new(()),
            load_lock: Mutex::new(()),
        }
    }

    // pub fn equal(&self, other: &JavaClass) -> bool {
    // self.name()==other.name() &&( self.define_class_loader()==other.define_class_loader()||(self.is_loaded()&&))
    // }

    // pub fn get_executables(&self) -> Fallible<&HashSet<ExecutableBox>> {
    // self.after_loaded_do(|c| Ok(c.executables()))
    // }
    pub fn get_executable(&self, name_and_parameters: &(PooledStr, PooledStr)) -> Fallible<Option<&Arc<Executable>>> {
        self.after_loaded_do(|c| Ok(c.executables().get(name_and_parameters)))
    }

    // pub fn get_executable(
    // &self,
    // name_and_parameters: &ExecutableNameAndParameterType,
    // ) -> Option<&Arc<Executable>> {
    // self.loaded()
    // .executables()
    // .get(name_and_parameters)
    // .map(|b| &b.0)
    // }

    // pub fn get_fields(&self) -> &HashSet<FieldBox> {
    // self.loaded().fields()
    // }

    // pub fn get_field(&self, name: &str) -> Option<&Arc<Field>> {
    // self.get_fields().get(name).map(|f| &f.0)
    // }

    // pub fn get_declared_non_static_field(&self, name: &str) -> Option<&Arc<Field>> {
    // self.extend_chain().find_map(|c| {
    // c.get_field(name).and_then(|f| {
    // if !f.modifiers().is_static() {
    // Some(f)
    // } else {
    // None
    // }
    // })
    // })
    // }

    pub fn extend_chain(&self) -> Fallible<&Vec<Arc<JavaClass>>> {
        self.after_loaded_do(|c| Ok(c.extends_chain()))
    }

    pub fn is_assignable(&self, other: &Arc<JavaClass>) -> bool {
        todo!() // TODO
    }

    pub fn super_class(&self) -> Fallible<Option<&Arc<JavaClass>>> {
        self.after_loaded_do(|c| Ok(c.super_class()))
    }
}

#[derive(Debug, Clone)]
pub enum ClassObjectType {
    Primitive(PrimitiveType),
    Array(Arc<JavaClass>),
    Class,
}
impl JavaClass {
    fn ensure<'l>(self: &'l Self, check: impl Fn(&Self) -> bool, action: impl Fn(&Self) -> Fallible<()>) -> Fallible<()> {
        if !check(self) {
            let _guard = self.process_lock.lock().map_err(|e| format_err!("PoisonError:{:?}", e))?;
            {
                if !check(self) {
                    action(self);
                }
            }
        }
        Ok(())
    }

    pub fn ensure_loaded<'l>(self: &'l Self) -> Fallible<()> {
        self.ensure(
            |s| s.component.is_loaded(),
            |s| self.define_class_loader.upgrade().ok_or_else(|| format_err!("class loader has been droped"))?.load_class(self).map(|b| ()),
        )
    }

    pub fn ensure_verifyed(&self) -> Fallible<()> {
        self.after_loaded_do(|c| c.ensure_verifyed())
    }

    pub fn ensure_initialized(&self) -> Fallible<()> {
        self.after_loaded_do(|c| c.ensure_initialized())
    }
}
impl JavaClass {
    pub fn is_loaded(&self) -> bool {
        self.component.is_loaded()
    }

    // fn loaded(&self) -> &JavaClassComponent {
    //     self.component.load_option().unwrap()
    // }

    fn after_loaded_do<'l, R: 'l>(&'l self, action: impl Fn(&'l JavaClassComponent) -> Fallible<R>) -> Fallible<R> {
        let mut c = self.component.load_option();
        let component = if let Some(loaded) = c {
            loaded
        } else {
            self.define_class_loader.upgrade().ok_or_else(|| format_err!("class loader has been droped"))?.load_class(self)?;
            self.component.load()
        };
        action(component)
    }

    fn init_component(&self, component: Arc<JavaClassComponent>) {
        self.component.init(component)
    }

    pub fn processing(&self) -> Fallible<MutexGuard<()>> {
        self.process_lock.lock().map_err(|e| format_err!("PoisonError:{:?}", e))
    }

    pub fn load_lock(&self) -> Fallible<MutexGuard<()>> {
        self.load_lock.lock().map_err(|e| format_err!("PoisonError:{:?}", e))
    }

    pub fn define_class(self: &Arc<Self>, init_class_loader: &Arc<ClassLoader>, name: &PooledStr, class_file: ClassFile) -> Fallible<()> {
        let component = JavaClassComponent::try_load_from_class_file(
            name.clone(),
            self.binary_name.clone(),
            self.clone(),
            class_file,
            init_class_loader,
            init_class_loader.get_class_graph(),
        )?;
        self.init_component(Arc::new(component));
        Ok(())
    }

    pub fn define_from_class(&self, class: &Arc<JavaClass>) -> Fallible<()> {
        let name = self.name();
        if class.name() != name {
            Err(format_err!("no class found,wrong name:{}", name))?;
        }
        self.init_component(class.component.clone_arc());
        Ok(())
    }
}
impl JavaClass {
    fn get_raw_declared_executable(&self, name_and_descriptor: &(PooledStr, PooledStr)) -> Fallible<&Arc<Executable>> {
        self.after_loaded_do(|c| c.declared_executables().get(name_and_descriptor).ok_or_else(|| format_err!("constructor not found")))
    }

    fn get_raw_executable(&self, name_and_descriptor: &(PooledStr, PooledStr)) -> Fallible<&Arc<Executable>> {
        self.after_loaded_do(|c| c.executables().get(name_and_descriptor).ok_or_else(|| format_err!("constructor not found")))
    }

    fn get_raw_declared_field(&self, name: &str) -> Fallible<&Arc<Field>> {
        self.after_loaded_do(|c| c.declared_fields().get(name).ok_or_else(|| format_err!("field not found")))
    }

    fn get_raw_field(&self, name: &str) -> Fallible<&Arc<Field>> {
        self.after_loaded_do(|c| c.fields().get(name).ok_or_else(|| format_err!("field not found")))
    }

    fn get_raw_generic_super_class(&self) -> Fallible<Option<&GenericType>> {
        self.after_loaded_do(|c| Ok(c.super_generic_type().as_ref()))
    }

    fn get_raw_class_loader(&self) -> Fallible<&Arc<ClassLoader>> {
        self.after_loaded_do(|c| Ok(c.init_class_loader()))
    }

    fn get_raw_component_type(&self) -> Fallible<Option<JavaClassRef>> {
        match &self.class_kind {
            JavaClassKind::Array(c) => Ok(Some(c.clone())),
            _ => Ok(None),
        }
    }
}
impl JavaClass {
    pub(crate) fn get_raw_interfaces(&self) -> Fallible<&HashMap<Arc<JavaClass>, (GenericType, Arc<dyn ImplementLayoutTrait>)>> {
        self.after_loaded_do(|c| Ok(c.interfaces()))
    }

    pub(crate) fn get_raw_fields(&self) -> Fallible<&HashMap<PooledStr, Arc<Field>>> {
        self.after_loaded_do(|c| Ok(c.fields()))
    }

    pub(crate) fn get_raw_declared_fields(&self) -> Fallible<&HashMap<PooledStr, Arc<Field>>> {
        self.after_loaded_do(|c| Ok(c.declared_fields()))
    }

    pub(crate) fn get_raw_executables(&self) -> Fallible<&HashMap<(PooledStr, PooledStr), Arc<Executable>>> {
        self.after_loaded_do(|c| Ok(c.executables()))
    }

    pub(crate) fn get_raw_declared_executables(&self) -> Fallible<&HashMap<(PooledStr, PooledStr), Arc<Executable>>> {
        self.after_loaded_do(|c| Ok(c.declared_executables()))
    }
}
impl JavaClass {
    pub fn into_dyn(self: Arc<Self>) -> Arc<JavaClass> {
        self as Arc<JavaClass>
    }
}
impl JavaClass {
    pub fn element_class(&self) -> Option<&Arc<JavaClass>> {
        todo!() // TODO
    }

    pub fn to_array(&self) -> &Arc<JavaClass> {
        todo!() // TODO
    }
}
impl TypeTreeNode for JavaClass {
    fn get_type_variable(&self, _name: &str) -> Option<Type> {
        todo!()
    }
}
impl AnnotatedElement for JavaClass {}
impl PartialEq for JavaClass {
    fn eq(&self, other: &Self) -> bool {
        (self as *const Self) == (other as *const Self)
    }
}
impl Eq for JavaClass {}
// impl PartialOrd for JavaClass {
// fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
// Some(self.cmp(other))
// }
// }
// impl Ord for JavaClass {
// fn cmp(&self, other: &Self) -> std::cmp::Ordering {
// match self.name.cmp(&other.name) {
// Ordering::Equal => self.define_class_loader.cmp(&other.define_class_loader),
// o => o,
// }
// }
// }
pub trait UsageField: Debug + Send + Sync {
    fn get_referrer_java_class(&self) -> Option<Weak<JavaClass>>;
    fn get_referrer_type(&self) -> GenericType;
    fn get_referrer_field(&self) -> Arc<Field>;
}
