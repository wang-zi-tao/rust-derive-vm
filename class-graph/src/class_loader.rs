use crate::{
    class::{JavaClass, JavaClassRef},
    package::{Package, PackageRef},
    ClassGraph,
};

use classfile::ClassFile;
use dashmap::{mapref::entry::Entry, DashMap};
use failure::{format_err, Error, Fallible};
use jvm_core::Component;
use std::{
    any::Any,
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    sync::{Arc, RwLock, Weak},
};
use util::PooledStr;
pub trait ClassFileLoader: Sync + Send + Debug {
    fn load_class(&self, name: &str, class_loader: &Arc<ClassLoader>) -> Fallible<Arc<JavaClass>>;
}

// impl Debug for ClassLoaderRef {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Fallible {
//         f.debug_tuple("ClassLoaderRef")
//             .field(&Arc::as_ptr(&self))
//             .finish()
//     }
// }
#[derive(Debug)]
pub struct ClassLoader {
    class_graph: &'static ClassGraph,
    children: Vec<Arc<ClassLoader>>,
    // parent: Option<Arc<ClassLoader>>,
    java_classes: DashMap<PooledStr, Arc<JavaClass>>,
    packages: DashMap<PooledStr, Arc<Package>>,
    bootstrap_class_loader: Option<Arc<ClassLoader>>,
    bootstrap_class_set: Arc<BootstrapClassSet>,
    class_file_loader: Box<dyn ClassFileLoader>,
    is_bootstrap: bool,
    self_ref: Weak<Self>,
}
impl ClassLoader {
    pub fn self_ref(&self) -> Arc<Self> {
        self.self_ref.upgrade().unwrap()
    }

    pub fn get_package(self: &Arc<ClassLoader>, name: &str) -> Arc<Package> {
        self.packages
            .get(name)
            .map(|p| p.clone())
            .unwrap_or_else(|| Arc::new(Package::new(self.self_ref(), name.into())))
    }

    pub fn get_class(&self, name: &str) -> Fallible<Arc<JavaClass>> {
        if let Some(class) = self.java_classes.get(&*name) {
            return Ok(class.clone());
        }
        let name_pooled = PooledStr::from(name);
        let class = Arc::new(JavaClass::new(name_pooled.clone(), &self.self_ref()));
        self.java_classes.insert(name_pooled, class.clone());
        Ok(class)
    }

    pub fn get_loaded_class(self: &Arc<Self>, name: &str) -> Fallible<Arc<JavaClass>> {
        let class = self.get_class(name)?;
        class.ensure_loaded()?;
        Ok(class)
    }

    pub fn get_initialized_class(self: &Arc<Self>, name: &str) -> Fallible<Arc<JavaClass>> {
        let class = self.get_class(name)?;
        class.ensure_initialized()?;
        Ok(class)
    }

    pub fn define_class(
        self: &Arc<Self>,
        name_option: Option<PooledStr>,
        class_file: &[u8],
    ) -> Fallible<Arc<JavaClass>> {
        let parsed_class_file = ClassFile::new(&*class_file)?;
        let name = if let Some(name) = name_option {
            name
        } else {
            parsed_class_file.this_class.symbol.name.clone()
        };
        let class = self.get_class(&name)?;
        {
            let _guard = class.load_lock()?;
            {
                if class.is_loaded() {
                    Err(format_err!("attempted duplicate class definition:{}", name))?;
                } else {
                    class.define_class(self, &name, parsed_class_file)?;
                }
            }
        }
        Ok(class)
    }

    pub fn get_class_graph(&self) -> &ClassGraph {
        self.class_graph
    }

    pub fn load_class(self: &Arc<Self>, class: &JavaClass) -> Fallible<bool> {
        let _guard = class.load_lock()?;
        Ok({
            if class.is_loaded() {
                false
            } else {
                let name = class.name();
                let c = self.class_file_loader.load_class(name, self)?;
                if Weak::as_ptr(c.define_class_loader()) != Arc::as_ptr(self) {
                    class.define_from_class(&c)?;
                }
                true
            }
        })
    }

    pub fn get_and_load_class(self: &Arc<Self>, name: &str) -> Fallible<Arc<JavaClass>> {
        let class = self.get_class(name)?;
        if !class.is_loaded() {
            let name: PooledStr = name.into();
            match self.java_classes.entry(name.clone()) {
                Entry::Occupied(o) => {
                    let class = o.get();
                    if !class.is_loaded() {
                        self.load_class(class)?;
                    }
                }
                Entry::Vacant(v) => {}
            }
        }
        class.ensure_loaded()?;
        Ok(class)
    }

    pub fn is_bootstrap(&self) -> bool {
        self.is_bootstrap
    }

    pub fn get_bootstrap_class_loader(&self) -> &ClassLoader {
        if let Some(bootstrap) = self.bootstrap_class_loader.as_ref() {
            bootstrap
        } else {
            self
        }
    }

    pub fn get_bootstrap_class_set(&self) -> &BootstrapClassSet {
        &self.bootstrap_class_set
    }

    pub fn create_bootstrap_class_loader() -> Fallible<Arc<ClassLoader>> {
        todo!() // TODO
    }
}
impl ClassLoader {
    fn init_class(&self, class: &JavaClass) -> Fallible<()> {
        class.ensure_initialized()
    }

    fn define_package(
        &self,
        name: PooledStr,
        spec_title: Option<PooledStr>,
        spec_version: Option<PooledStr>,
        spec_vendor: Option<PooledStr>,
        impl_title: Option<PooledStr>,
        impl_version: Option<PooledStr>,
        impl_vendor: Option<PooledStr>,
        seal_base: Option<PooledStr>,
    ) -> Fallible<PackageRef> {
        todo!()
    }

    fn resolve_class(&self, class: &JavaClassRef) -> Fallible<()> {
        todo!()
    }

    fn get_class_from_symbol(&self, symbol: &str) -> Fallible<JavaClassRef> {
        todo!()
    }
}
impl Hash for ClassLoader {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (self as *const ClassLoader).hash(state);
    }
}
impl PartialEq for ClassLoader {
    fn eq(&self, other: &Self) -> bool {
        self as *const ClassLoader == other as *const ClassLoader
    }
}
impl Eq for ClassLoader {}
impl PartialOrd for ClassLoader {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (self as *const ClassLoader).partial_cmp(&(other as *const ClassLoader))
    }
}
impl Ord for ClassLoader {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self as *const ClassLoader).cmp(&(other as *const ClassLoader))
    }
}
#[derive(Debug)]
pub struct BootstrapClassSet {
    pub boolean: Arc<JavaClass>,
    pub byte: Arc<JavaClass>,
    pub short: Arc<JavaClass>,
    pub char: Arc<JavaClass>,
    pub int: Arc<JavaClass>,
    pub long: Arc<JavaClass>,
    pub float: Arc<JavaClass>,
    pub double: Arc<JavaClass>,

    pub java_lang_object: Arc<JavaClass>,
    pub java_lang_class: Arc<JavaClass>,
    pub java_lang_string: Arc<JavaClass>,
    pub java_lang_invoke_method_type: Arc<JavaClass>,
    pub java_lang_invoke_method_handle: Arc<JavaClass>,

    pub java_lang_throwable: Arc<JavaClass>,
    pub java_lang_exception: Arc<JavaClass>,
    pub java_lang_runtime_exception: Arc<JavaClass>,

    pub java_lang_arithmetic_exception: Arc<JavaClass>,
}
