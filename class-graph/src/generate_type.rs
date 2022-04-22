use crate::{
    class::{JavaClass, JavaClassRef},
    class_loader::ClassLoader,
    signature::{FieldSignature, JavaTypeSignature},
};
use classfile::attributes::Attribute;
use dashmap::lock::RwLock;
use failure::Fallible;
use std::{
    borrow::Borrow,
    cmp::Ordering,
    collections::{HashMap, HashSet},
    fmt::Debug,
    hash::{Hash, Hasher},
    ops::{Deref, DerefMut},
    sync::Arc,
};
use util::{AtomicLazyArc, PooledStr};

use crate::signature::{Context as SignatureContext, TypeParameter as TypeParameterSignature};
#[derive(Debug, Clone, Copy)]
pub enum PrimitiveType {
    Byte,
    Char,
    Double,
    Float,
    Int,
    Long,
    Short,
    Boolean,
}

#[derive(Debug, Clone)]
pub struct ParameterizedType {
    pub arguments: Vec<Type>,
    pub owner: Option<Type>,
    pub raw: Arc<JavaClass>,
}
#[derive(Debug, Clone)]
pub enum WildcardType {
    NoBound,
    Extends(Type),
    Super(Type),
}
#[derive(Clone)]
pub struct TypeVariable {
    pub bounds: AtomicLazyArc<Vec<Type>>,
    pub name: PooledStr,
}
impl TypeVariable {
    pub fn new(type_parameter: &TypeParameterSignature) -> Fallible<Self> {
        Ok(Self {
            name: type_parameter.name.string.clone(),
            bounds: Default::default(),
        })
    }

    pub fn update_bounds<T: TypeTreeNode>(
        &self,
        type_parameter: TypeParameterSignature,
        signature_context: &SignatureContext<'_, T>,
    ) -> Fallible<()> {
        let mut bounds = if let Some(class_bound) = type_parameter.bounds.bound {
            let mut bounds = Vec::with_capacity(1 + type_parameter.super_interface.bound.len());
            bounds.push(class_bound.to_type(signature_context)?);
            bounds
        } else {
            Vec::with_capacity(type_parameter.super_interface.bound.len())
        };
        for interface in type_parameter.super_interface.bound {
            bounds.push(interface.to_type(signature_context)?)
        }
        self.bounds.init(Arc::new(bounds));
        Ok(())
    }
}
impl Debug for TypeVariable {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
#[derive(Debug)]
pub struct TypeVariableReference(Arc<TypeVariable>);
pub trait TypeTreeNode {
    fn get_type_variable(&self, name: &str) -> Option<Type>;
    // fn get_type_parameters(&self) -> Box<dyn Iterator<Item = &Type>>;
}
impl Borrow<str> for TypeVariableReference {
    fn borrow(&self) -> &str {
        &*self.0.name
    }
}
impl Clone for TypeVariableReference {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl Deref for TypeVariableReference {
    type Target = TypeVariable;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}
impl From<TypeVariable> for TypeVariableReference {
    fn from(type_variable: TypeVariable) -> Self {
        Self(Arc::new(type_variable))
    }
}
impl Hash for TypeVariableReference {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.name.hash(state)
    }
}
impl PartialEq for TypeVariableReference {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for TypeVariableReference {}
pub type TypeVariableSet = HashSet<TypeVariableReference>;
#[derive(Clone)]
pub enum Type {
    JavaClass(Arc<JavaClass>),
    Array(Box<Type>),
    Parameterized(Box<ParameterizedType>),
    Wildcard(Box<WildcardType>),
    TypeVariable(TypeVariableReference),
    Primitive(PrimitiveType),
    Void,
}
impl Type {
    pub fn child_with_type_arguments(
        self,
        _raw: Arc<JavaClass>,
        _args: Vec<Type>,
    ) -> Fallible<Type> {
        todo!(); // TODO
    }

    pub fn to_array(self) -> Fallible<Type> {
        Ok(Type::Array(Box::new(self)))
    }

    pub fn get_class(&self, _class_loader: &ClassLoader) -> Fallible<Arc<JavaClass>> {
        todo!()
    }

    pub fn from_symbol(class_loader: &ClassLoader, symbol: &str) -> Fallible<Self> {
        Ok(match symbol {
            "I" => Type::Primitive(PrimitiveType::Int),
            "J" => Type::Primitive(PrimitiveType::Long),
            "S" => Type::Primitive(PrimitiveType::Short),
            "C" => Type::Primitive(PrimitiveType::Char),
            "B" => Type::Primitive(PrimitiveType::Byte),
            "Z" => Type::Primitive(PrimitiveType::Boolean),
            "D" => Type::Primitive(PrimitiveType::Double),
            "F" => Type::Primitive(PrimitiveType::Float),
            "V" => Type::Void,
            _ => Type::JavaClass(class_loader.get_class_from_symbol(symbol)?),
        })
    }
}
impl Debug for Type {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
impl From<Arc<JavaClass>> for Type {
    fn from(class: Arc<JavaClass>) -> Self {
        Type::JavaClass(class)
    }
}
pub struct GenericType {
    java_type: Type,
    raw_class: Arc<JavaClass>,
}
impl GenericType {
    pub fn new(java_type: Type, raw_class: Arc<JavaClass>) -> Self {
        Self {
            java_type,
            raw_class,
        }
    }

    pub fn from_non_primitive_class(
        class_loader: &Arc<ClassLoader>,
        class: Arc<JavaClass>,
    ) -> Fallible<Self> {
        Ok(Self {
            java_type: Type::JavaClass(class.clone()),
            raw_class: class,
        })
    }

    pub fn from_symbol(class_loader: &Arc<ClassLoader>, symbol: &PooledStr) -> Fallible<Self> {
        Ok(match &**symbol {
            "I" => Self::new(
                Type::Primitive(PrimitiveType::Int),
                class_loader.get_bootstrap_class_set().int.clone(),
            ),
            "J" => Self::new(
                Type::Primitive(PrimitiveType::Long),
                class_loader.get_bootstrap_class_set().long.clone(),
            ),
            "S" => Self::new(
                Type::Primitive(PrimitiveType::Short),
                class_loader.get_bootstrap_class_set().short.clone(),
            ),
            "C" => Self::new(
                Type::Primitive(PrimitiveType::Char),
                class_loader.get_bootstrap_class_set().char.clone(),
            ),
            "B" => Self::new(
                Type::Primitive(PrimitiveType::Byte),
                class_loader.get_bootstrap_class_set().byte.clone(),
            ),
            "Z" => Self::new(
                Type::Primitive(PrimitiveType::Boolean),
                class_loader.get_bootstrap_class_set().boolean.clone(),
            ),
            "D" => Self::new(
                Type::Primitive(PrimitiveType::Double),
                class_loader.get_bootstrap_class_set().double.clone(),
            ),
            "F" => Self::new(
                Type::Primitive(PrimitiveType::Float),
                class_loader.get_bootstrap_class_set().float.clone(),
            ),
            _ => {
                let name = if (&**symbol).bytes().next() == Some(b'[') {
                    &**symbol
                } else {
                    &symbol[0..symbol.len() - 1]
                };
                let class = class_loader.get_class(name)?;
                Self::new(Type::JavaClass(class.clone()), class)
            }
        })
    }

    pub fn from_java_type_signature(
        class: Arc<JavaClass>,
        signature: &JavaTypeSignature,
        class_loader: &Arc<ClassLoader>,
        type_tree_node: &impl TypeTreeNode,
    ) -> Fallible<Self> {
        let context = SignatureContext::new(&**class_loader, type_tree_node);
        let java_type = signature.to_type(&context)?;
        Ok(GenericType {
            java_type,
            raw_class: class,
        })
    }

    pub fn from_field_signature(
        descriptor: &PooledStr,
        class: &Arc<JavaClass>,
        attributes: &HashMap<PooledStr, Attribute>,
        class_loader: &Arc<ClassLoader>,
        type_tree_node: &impl TypeTreeNode,
    ) -> Fallible<Self> {
        if let Some(Attribute::Signature(signature_attribute)) = attributes.get("Signature") {
            let signature = FieldSignature::from_attribute(signature_attribute)?;
            let context = SignatureContext::new(&**class_loader, type_tree_node);
            let java_type = signature.to_type(&context)?;
            Ok(GenericType {
                java_type,
                raw_class: class.clone(),
            })
        } else {
            Ok(GenericType::from_symbol(class_loader, descriptor)?)
        }
    }

    pub fn get_class(&self) -> &Arc<JavaClass> {
        &self.raw_class
    }

    pub fn get_type(&self) -> &Type {
        &self.java_type
    }

    pub fn set_type(&mut self, r#type: Type) {
        self.java_type = r#type;
    }

    pub fn is_primitive(&self) -> bool {
        if let Type::Primitive(_) = &self.java_type {
            true
        } else {
            false
        }
    }
}
pub struct TemporaryTypeTreeNode<'a> {
    pub type_variables: HashSet<TypeVariableReference>,
    pub parents: Option<&'a dyn TypeTreeNode>,
}
impl<'a> TypeTreeNode for TemporaryTypeTreeNode<'a> {
    fn get_type_variable(&self, name: &str) -> Option<Type> {
        if let Some(type_variable) = self.type_variables.get(name) {
            Some(Type::TypeVariable(type_variable.clone()))
        } else {
            self.parents.and_then(|p| p.get_type_variable(name))
        }
    }
}
impl<'a> TemporaryTypeTreeNode<'a> {
    pub fn new(
        type_parameter_signature: Vec<TypeParameterSignature>,
        parents: Option<&'a dyn TypeTreeNode>,
        class_loader: &Arc<ClassLoader>,
    ) -> Fallible<Self> {
        let _len = type_parameter_signature.len();
        let mut type_variables = HashSet::with_capacity(type_parameter_signature.len());
        for type_parameter in &type_parameter_signature {
            type_variables.insert(TypeVariable::new(&type_parameter)?.into());
        }
        let this = Self {
            type_variables,
            parents,
        };
        for type_parameter in type_parameter_signature {
            let type_variable_reference = this
                .type_variables
                .get(type_parameter.name.to_str())
                .unwrap();
            type_variable_reference.update_bounds(
                type_parameter,
                &SignatureContext::new(&**class_loader, &this),
            )?;
        }
        Ok(this)
    }
}

pub trait GenericDeclaration: TypeTreeNode {}
