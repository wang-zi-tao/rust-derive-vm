use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use classfile::{
    attributes,
    attributes::{Attribute, RuntimeVisibleAnnotations},
    constants,
};

use failure::Fallible;
use util::PooledStr;
use vm_core::OOPRef;

use crate::{
    class::{JavaClass, JavaClassRef},
    class_loader::ClassLoader,
};
#[derive(Debug)]
pub struct Annotation {
    pub elements: Vec<(PooledStr, ElementValue)>,
}
impl Annotation {
    pub fn parse(annotation: &attributes::Annotation, class_loader: &ClassLoader) -> Fallible<(Arc<JavaClass>, Self)> {
        let mut elements = Vec::with_capacity(annotation.elements.len());
        for (n, e) in &annotation.elements {
            elements.push((n.clone(), ElementValue::parse(e, class_loader)?));
        }
        Ok((class_loader.get_class(&annotation.type_descriptor)?, Self { elements }))
    }
}
#[derive(Debug)]
pub struct Annotations {
    annotations: HashMap<Arc<JavaClass>, Annotation>,
}
impl Annotations {}
impl Default for Annotations {
    fn default() -> Self {
        Self { annotations: HashMap::new() }
    }
}
impl Annotations {
    pub fn parse(attributes: &HashMap<PooledStr, Attribute>, class_loader: &ClassLoader) -> Fallible<Self> {
        if let Some(Attribute::RuntimeVisibleAnnotations(attribute)) = attributes.get("RuntimeVisibleAnnotations") {
            Annotations::parse_annotations_attributes(attribute, class_loader)
        } else {
            Ok(Self { annotations: HashMap::new() })
        }
    }

    pub fn parse_annotations_attributes(attribute: &RuntimeVisibleAnnotations, class_loader: &ClassLoader) -> Fallible<Self> {
        let mut annotations = HashMap::with_capacity(attribute.annotations.len());
        for a in &attribute.annotations {
            let (class, annotation) = Annotation::parse(a, class_loader)?;
            annotations.insert(class, annotation);
        }
        Ok(Self { annotations })
    }
}
#[derive(Debug)]
pub enum Constant {
    Integer(i32),
    Double(f64),
    Float(f32),
    Long(i64),
    String(PooledStr),
}
impl Constant {
    pub fn parse(constant: &constants::Constant) -> Self {
        match constant {
            constants::Constant::ConstantInteger(i) => Constant::Integer(*i),
            constants::Constant::ConstantUtf8(s) => Constant::String(s.clone()),
            constants::Constant::ConstantFloat(f) => Constant::Float(*f),
            constants::Constant::ConstantLong(l) => Constant::Long(*l),
            constants::Constant::ConstantDouble(d) => Constant::Double(*d),
            _ => panic!(),
        }
    }
}
#[derive(Debug)]
pub enum ElementValue {
    ConstValue(u8, Constant),
    EnumConstValue { enum_type: Arc<JavaClass>, const_name: PooledStr },
    ClassInfo(Arc<JavaClass>),
    AnnotationValue(Box<(Arc<JavaClass>, Annotation)>),
    ArrayValue(Vec<ElementValue>),
}
impl ElementValue {
    pub fn parse(elements: &attributes::ElementValue, class_loader: &ClassLoader) -> Fallible<Self> {
        Ok(match elements {
            attributes::ElementValue::ConstValue(tag, constant) => ElementValue::ConstValue(*tag, Constant::parse(constant)),
            attributes::ElementValue::EnumConstValue { type_name, const_name } => {
                ElementValue::EnumConstValue { const_name: const_name.clone(), enum_type: class_loader.get_class(type_name)? }
            }
            attributes::ElementValue::ClassInfo(type_name) => ElementValue::ClassInfo(class_loader.get_class(type_name)?),
            attributes::ElementValue::AnnotationValue(annotation) => ElementValue::AnnotationValue(Box::new(Annotation::parse(&*annotation, class_loader)?)),
            attributes::ElementValue::ArrayValue(array) => {
                let mut elements = Vec::with_capacity(array.len());
                for element in array {
                    elements.push(ElementValue::parse(element, class_loader)?);
                }
                ElementValue::ArrayValue(elements)
            }
        })
    }
}

pub trait AnnotatedElement {
    fn get_annotation(&self, annotation_class: &JavaClassRef) -> Fallible<Option<Arc<OOPRef>>>;
    fn get_annotations(&self) -> Fallible<Box<dyn Iterator<Item = Arc<OOPRef>>>>;
    fn get_annotation_by_type(&self, annotation_class: &JavaClassRef) -> Fallible<Option<Arc<OOPRef>>>;
    fn get_declared_annotation(&self, annotation_class: &JavaClassRef) -> Fallible<Option<Arc<OOPRef>>>;
    fn get_declared_annotations(&self) -> Fallible<Box<dyn Iterator<Item = Arc<OOPRef>>>>;
    fn get_declared_annotation_by_type(&self, annotation_class: &JavaClassRef) -> Fallible<Option<Arc<OOPRef>>>;
    fn is_annotation_present(&self, annotation_class: &JavaClassRef) -> Fallible<bool>;
}
