use classfile::{
    self,
    attributes::Attribute,
    symbol::{MethodSymbol, MethodTypeSymbol},
};
use failure::{format_err, Fallible};
use jvm_core::ExecutableLayoutTrait;

use crate::{
    annotations::{AnnotatedElement, Annotations},
    class::JavaClass,
    class_loader::ClassLoader,
    generate_type::{
        GenericDeclaration, GenericType, TemporaryTypeTreeNode, TypeTreeNode, TypeVariable,
        TypeVariableSet,
    },
    member::{AccessibleObject, Member},
    modifiers::{parse_modifiers, HasModifier},
    signature::Context as SignatureContext,
    ClassGraph,
};
use std::{
    borrow::Borrow,
    default::default,
    fmt::Debug,
    hash::{Hash, Hasher},
    ops::Deref,
    sync::{Arc, Weak},
};
use util::PooledStr;

pub struct Executable {
    name: PooledStr,
    descriptor: PooledStr,
    parameters: Vec<Parameter>,
    modifiers: i32,
    annotations: Annotations,
    declaring: Weak<JavaClass>,
    return_type: Option<GenericType>,
    throws: Vec<GenericType>,
    layout: Arc<dyn ExecutableLayoutTrait>,
    type_variables: TypeVariableSet,
    extends: Option<Arc<Executable>>,
}
impl TypeTreeNode for Executable {
    fn get_type_variable(&self, name: &str) -> Option<jvm_core::Type> {
        todo!()
    }
}
impl GenericDeclaration for Executable {}
impl AnnotatedElement for Executable {}
impl AccessibleObject for Executable {
    fn can_access(&self, object: Option<&dyn jvm_core::OOPTrait>) -> bool {
        todo!()
    }

    fn is_accessible(&self) -> bool {
        todo!()
    }

    fn try_set_accessible(&self) -> bool {
        todo!()
    }

    fn set_accessible(&self, flat: bool) -> bool {
        todo!()
    }
}
impl Executable {
    pub fn is_constructor(&self) -> bool {
        &**self.name() == "<init>"
    }

    pub fn new(
        super_class: Option<Arc<JavaClass>>,
        declaring: Arc<JavaClass>,
        class_loader: &Arc<ClassLoader>,
        method: &classfile::Method,
        layout: Arc<dyn ExecutableLayoutTrait>,
        class_graph: &ClassGraph,
    ) -> Fallible<Self> {
        let symbol = MethodTypeSymbol::new(&method.descriptor)
            .ok_or(format_err!("illegal executable descriptor"))?;
        let (parameters_type_symbol, return_type_type_symbol) =
            (symbol.parameters, symbol.return_type);
        let mut parameters: Vec<Parameter> = Vec::with_capacity(parameters_type_symbol.len());
        for p in parameters_type_symbol {
            let parameter_type: GenericType = GenericType::from_symbol(class_loader, &p.name)?;
            parameters.push(parameter_type.into());
        }
        let mut throws: Vec<GenericType> =
            if let Some(Attribute::Exception(e)) = method.attributes.get("Exceptions") {
                let mut exceptions = Vec::with_capacity(e.exception_table.len());
                for exception in &e.exception_table {
                    let throwable_name = &exception.symbol.name;
                    exceptions.push(GenericType::from_symbol(class_loader, throwable_name)?);
                }
                exceptions
            } else {
                Vec::new()
            };
        let mut return_type: Option<GenericType> = if &*return_type_type_symbol.name == "V" {
            None
        } else {
            Some(GenericType::from_symbol(
                class_loader,
                &return_type_type_symbol.name,
            )?)
        };
        let declaring_weak = Arc::downgrade(&declaring);
        let type_variables =
            if let Some(Attribute::Signature(signature)) = method.attributes.get("Signature") {
                let executable_signature = MethodSymbol::from_attribute(signature)?;
                let temporary_type_variables = TemporaryTypeTreeNode::new(
                    executable_signature.type_parameters,
                    Some(&*declaring),
                    class_loader,
                )?;
                let context = SignatureContext::new(&**class_loader, &temporary_type_variables);
                for (parameter, parameter_signature) in parameters
                    .iter_mut()
                    .zip(executable_signature.parameters.iter())
                {
                    parameter
                        .parameter_type
                        .set_type(parameter_signature.to_type(&context)?);
                }
                for (throw_type, throw_signature) in
                    throws.iter_mut().zip(executable_signature.throws.iter())
                {
                    throw_type.set_type(throw_signature.to_type(&context)?);
                }
                if let Some(ref mut raw_reutrn_type) = return_type {
                    raw_reutrn_type.set_type(executable_signature.result.to_type(&context)?);
                }
                temporary_type_variables.type_variables
            } else {
                TypeVariable::new()
            };
        // let name_and_parameter = ExecutableNameAndParameterType {
        // name: method.name,
        // parameters,
        // };
        let name_and_parameters = (method.name.clone(), method.descriptor.clone());
        let extends = if let Some(super_class_some) = super_class {
            super_class_some
                .get_executable(&name_and_parameters)?
                .cloned()
        } else {
            None
        };
        Ok(Self {
            declaring: declaring_weak,
            name: name_and_parameters.0,
            descriptor: name_and_parameters.1,
            parameters,
            modifiers: parse_modifiers(method.access_flags, &method.attributes),
            annotations: Annotations::parse(&method.attributes, class_loader)?,
            return_type,
            throws,
            layout,
            type_variables: type_variables.into_iter().collect(),
            extends,
        })
    }
}
impl Hash for Executable {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.descriptor.hash(state);
    }
}
impl PartialEq for Executable {
    fn eq(&self, other: &Self) -> bool {
        (self as *const Executable) == (other as *const Executable)
    }
}
impl Eq for Executable {}
impl HasModifier for Executable {
    fn modifiers(&self) -> i32 {
        self.modifiers
    }
}
impl Member for Executable {
    fn name(&self) -> &PooledStr {
        &self.name
    }

    fn declaring_weak(&self) -> &std::sync::Weak<crate::class::JavaClass> {
        &self.declaring
    }
}
#[derive(Debug)]
pub struct Parameter {
    parameter_type: GenericType,
    annotations: Annotations,
    modifiers: i16,
    name: Option<PooledStr>,
}
impl Parameter {
    pub fn get_type(&self) -> &GenericType {
        &self.parameter_type
    }

    pub fn with_type(parameter_type: GenericType) -> Self {
        Self {
            parameter_type,
            annotations: Annotations::default(),
            modifiers: 0,
            name: None,
        }
    }
}
impl From<GenericType> for Parameter {
    fn from(parameter_type: GenericType) -> Self {
        Self {
            parameter_type,
            annotations: default(),
            modifiers: default(),
            name: default(),
        }
    }
}
impl Debug for Executable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Executable")
            .field(&format!(
                "{:?} {}.{}{}",
                &self.return_type,
                self.declaring().unwrap().binary_name(),
                self.name,
                self.descriptor
            ))
            .finish()
    }
}
impl Executable {
    pub fn get_extends(&self) -> Option<&Arc<Executable>> {
        self.extends.as_ref()
    }

    pub fn get_layout(&self) -> &dyn ExecutableLayoutTrait {
        &*self.layout
    }

    pub fn get_parameters(&self) -> &Vec<Parameter> {
        &self.parameters
    }

    pub fn get_return_type(&self) -> &Option<GenericType> {
        &self.return_type
    }
}
