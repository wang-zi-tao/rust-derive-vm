use super::{JavaClass, JavaClassUsage, UsageField};
use crate::{
    annotations::Annotations,
    class_loader::ClassLoader,
    executable::{self, Executable},
    field::Field,
    generate_type::{GenericType, TemporaryTypeTreeNode, TypeTreeNode, TypeVariableSet},
    modifiers::parse_modifiers,
    package::Package,
    signature::{ClassSignature, Context as SignatureContext},
    ClassGraph,
};
use classfile::{attributes::Attribute, ClassFile};
use crossbeam::atomic::AtomicCell as CrossbeamAtomicCell;
use dashmap::lock::{RwLock, RwLockReadGuard};
use failure::{format_err, Fallible};
use getset::{CopyGetters, Getters, MutGetters, Setters};
use std::{
    collections::{HashMap, HashSet},
    default::default,
    ops::Deref,
    sync::{Arc, Weak},
};
use util::PooledStr;
use vm_core::{ImplementLayoutTrait, TypeLayoutTrait};
#[derive(Debug, Copy, Clone)]
pub enum LifeTime {
    Loaded,
    Verified,
    Initialized,
}
pub struct JavaClassComponent {
    name: PooledStr,
    binary_name: PooledStr,
    declaring_class: Option<Arc<JavaClass>>,
    modifiers: i32,
    annotations: Annotations,

    // non_static_fields: HashMap<PooledStr, Arc<Field>>,
    // static_fields: HashMap<PooledStr, Arc<Field>>,
    // non_static_declared_fields: HashMap<PooledStr, Arc<Field>>,
    fields: HashMap<PooledStr, Arc<Field>>,
    declared_fields: HashMap<PooledStr, Arc<Field>>,

    // non_static_methods: HashMap<(PooledStr, PooledStr), Arc<Executable>>,
    // static_methods: HashMap<(PooledStr, PooledStr), Arc<Executable>>,
    // non_static_declared_methods: HashMap<(PooledStr, PooledStr), Arc<Executable>>,
    // executables: HashMap<(PooledStr, PooledStr), Arc<Executable>>,
    // constructors: HashMap<PooledStr, Arc<Executable>>,
    executables: HashMap<(PooledStr, PooledStr), Arc<Executable>>,
    declared_executables: HashMap<(PooledStr, PooledStr), Arc<Executable>>,

    interfaces: HashMap<Arc<JavaClass>, (GenericType, Arc<dyn ImplementLayoutTrait>)>,
    declared_interface: HashMap<Arc<JavaClass>, (GenericType, Arc<dyn ImplementLayoutTrait>)>,

    super_generic_type: Option<GenericType>,
    extends_chain: Vec<Arc<JavaClass>>,

    layout: Arc<dyn TypeLayoutTrait>,

    type_variables: TypeVariableSet,

    lifetime: CrossbeamAtomicCell<LifeTime>,

    init_class_loader: Arc<ClassLoader>,
}
impl Drop for JavaClassComponent {
    fn drop(&mut self) {
        todo!()
    }
}
impl JavaClassComponent {
    pub fn is_primitive(&self) -> bool {
        self.modifiers.is_primitive()
    }

    pub fn is_final(&self) -> bool {
        self.modifiers.is_final()
    }

    pub fn super_class(&self) -> Option<&Arc<JavaClass>> {
        self.super_generic_type.as_ref().map(|s| s.get_class())
    }

    pub fn try_load_from_class_file(
        name: PooledStr,
        binary_name: PooledStr,
        java_class: Arc<JavaClass>,
        class_file: ClassFile,
        class_loader: &Arc<ClassLoader>,
        class_graph: &ClassGraph,
    ) -> Fallible<Self> {
        if &binary_name != &class_file.this_class.symbol.name {
            Err(format_err!("wrong class name"))?;
        }
        let modifiers = parse_modifiers(class_file.access_flags, &class_file.attributes);
        let annotations = Annotations::parse(&class_file.attributes, &**class_loader)?;
        let mut outer: Option<Arc<JavaClass>> = None;
        if let Some(Attribute::InnerClasses(inner_class_attribute)) = class_file.attributes.get("InnerClasses") {
            for i in &inner_class_attribute.calsses {
                if &i.inner_class_info.symbol.name == &binary_name {
                    if let Some(outer_class_info) = &i.outer_class_info {
                        if outer.is_none() {
                            outer = Some(class_loader.get_class(&outer_class_info.symbol.name)?)
                        } else {
                            Err(format_err!("too much outer class"))?;
                        }
                    }
                }
            }
        }
        let super_class: Option<Arc<JavaClass>> = if let Some(c) = class_file.super_class {
            Some(class_loader.get_and_load_class(&*c.symbol.name)?)
        } else {
            None
        };
        let super_class_component = if let Some(ref super_class_some) = super_class {
            Some(super_class_some.after_loaded_do(|c| Ok(c))?)
        } else {
            None
        };

        let mut layout_builder = class_graph.memory.create_layout_builder();
        layout_builder.object_type(true)?;

        if let Some(ref super_class_some) = super_class {
            super_class_some.ensure_loaded()?;
        }
        if let Some(parents) = &super_class {
            layout_builder.extends(parents.get_layout()?.clone())?;
        }

        for field in &class_file.fields {
            let value_type: Arc<JavaClass> = class_loader.get_class(&field.descriptor)?;
            layout_builder.field(value_type.get_layout()?.clone(), field.access_flags);
        }
        for method in &class_file.methods {
            layout_builder.executable(method.access_flags)?;
        }
        for interface in &class_file.interface {
            let interface_class: Arc<JavaClass> = class_loader.get_class(&interface.symbol.name)?;
            layout_builder.implement(interface_class.get_layout()?.clone())?;
        }
        let (type_layout, implement_layouts, field_layouts, executable_layouts) = layout_builder.finish()?;
        let interface_count = class_file.interface.len();
        let mut declared_interface = super_class_component.map(|c| c.declared_interface().clone()).unwrap_or_else(|| HashMap::with_capacity(interface_count));
        let mut interfaces = HashMap::with_capacity(interface_count);
        for (interface, implement_layout) in class_file.interface.iter().zip(implement_layouts.into_iter()) {
            let interface = GenericType::from_symbol(class_loader, &interface.symbol.name)?;
            interfaces.insert(interface.get_class().clone(), (interface.clone(), implement_layout.clone()));
            declared_interface.insert(interface.get_class().clone(), (interface, implement_layout));
        }

        let mut declared_fields: HashMap<_, _> = super_class_component
            .iter()
            .map(|c| c.fields().iter().filter(|(n, f)| f.modifiers().is_final()).map(|(n, f)| (n.clone(), f.clone())))
            .flatten()
            .collect();
        let mut fields = HashMap::with_capacity(class_file.fields.len());
        for (field, field_layout) in class_file.fields.iter().zip(field_layouts) {
            let field = Arc::new(Field::new(&java_class, class_loader, field, field_layout, class_graph)?);
            fields.insert(field.name().clone(), field.clone());
            declared_fields.insert(field.name().clone(), field);
        }

        let method_count = class_file.methods.len();
        let mut declared_executables = super_class_component.map(|s| s.executables.clone()).unwrap_or_else(|| HashMap::with_capacity(method_count));
        let mut executables = HashMap::with_capacity(method_count);
        for (method, executable_layout) in class_file.methods.iter().zip(executable_layouts) {
            let executable = Arc::new(Executable::new(super_class.clone(), java_class.clone(), class_loader, method, executable_layout, class_graph)?);
            let descriptor = method.descriptor.clone();
            declared_executables.insert((executable.name().clone(), descriptor.clone()), executable.clone());
            executables.insert((executable.name().clone(), descriptor), executable);
        }

        let mut super_generic_type = if let Some(super_class_some) = &super_class {
            Some(GenericType::from_non_primitive_class(class_loader, super_class_some.clone())?)
        } else {
            None
        };
        let type_variables = if let Some(Attribute::Signature(signature_attribute)) = class_file.attributes.get("Signature") {
            let signature = ClassSignature::from_attribute(signature_attribute)?;
            let temporary_type_variables = TemporaryTypeTreeNode::new(signature.type_parameters, outer.as_ref().map(|o| &**o), class_loader)?;
            let context = SignatureContext::new(&**class_loader, &temporary_type_variables);
            if let Some(super_generic_type) = &mut super_generic_type {
                super_generic_type.set_type(signature.super_class.to_type(&context)?);
            }
            for interface_signature in signature.interfaces.bound {
                let interface_type = interface_signature.to_type(&context)?;
                let interface_class = Arc::downcast(interface_type.get_class(&**class_loader)?.clone().as_any_arc()).unwrap();
                interfaces.get_mut(&interface_class).unwrap().0.set_type(interface_type);
            }
            temporary_type_variables.type_variables
        } else {
            TypeVariableSet::new()
        };

        // let package_name=name.split_at(name.rfind('.')).0;
        let (package_name, package) = if let Some(index) = name.rfind('.') {
            let package_name = name.split_at(index).0;
            let package = class_loader.get_package(package_name);
            (Some(package_name), Some(package))
        } else {
            (None, None)
        };
        let mut extends_chain = super_class_component.map(|s| s.extends_chain().clone()).unwrap_or_else(|| Vec::new());
        extends_chain.push(java_class);
        Ok(Self {
            name,
            binary_name,
            declaring_class: outer,
            modifiers,
            annotations,

            fields,
            declared_fields,

            executables,
            declared_executables,

            interfaces,
            declared_interface,

            super_generic_type,
            extends_chain,

            type_variables: type_variables.into_iter().collect(),
            layout: type_layout,
            lifetime: CrossbeamAtomicCell::new(LifeTime::Loaded),
            init_class_loader: class_loader.clone(),
        })
    }
}
impl JavaClassComponent {
    pub fn ensure_verifyed(&self) -> Fallible<()> {
        todo!(); // TODO
    }

    pub fn ensure_initialized(&self) -> Fallible<()> {
        todo!(); // TODO
    }
}
impl PartialEq for JavaClassComponent {
    fn eq(&self, other: &Self) -> bool {
        (self as *const Self) == (other as *const Self)
    }
}
impl Eq for JavaClassComponent {}
