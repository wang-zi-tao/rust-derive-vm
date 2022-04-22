use classfile;
use failure::{format_err, Fallible};

use crate::{
    annotations::{AnnotatedElement, Annotations},
    class::{JavaClass, JavaClassRef},
    class_loader::ClassLoader,
    generate_type::GenericType,
    member::{Member, MemberInfo},
    modifiers::{parse_modifiers, HasModifier},
    ClassGraph,
};
use jvm_core::FieldLayoutTrait;
use std::{
    borrow::Borrow,
    fmt::Debug,
    hash::Hash,
    ops::{CoerceUnsized, Deref},
    sync::{Arc, Weak},
};
use util::PooledStr;
#[derive(Debug)]
pub struct Field {
    member_info: MemberInfo,
    field_type: GenericType,
    layout: Arc<dyn FieldLayoutTrait>,
}
impl Field {
    pub fn new(
        declaring: &Arc<JavaClass>,
        class_loader: &Arc<ClassLoader>,
        field: &classfile::Field,
        layout: Arc<dyn FieldLayoutTrait>,
        class_graph: &ClassGraph,
    ) -> Fallible<Self> {
        let class: Arc<JavaClass> = class_loader.get_class(&field.descriptor)?;
        let modifiers = parse_modifiers(field.access_flags, &field.attributes);
        let annotations = Annotations::parse(&field.attributes, class_loader)?;
        let field_type = GenericType::from_field_signature(
            &field.descriptor,
            &class,
            &field.attributes,
            class_loader,
            &**declaring,
        )?;

        Ok(Self {
            member_info: MemberInfo {
                name: field.name.clone(),
                modifiers,
                annotations,
                declaring: Arc::downgrade(&declaring),
            },
            layout,
            field_type,
        })
    }
}
impl HasModifier for Field {
    fn modifiers(&self) -> i32 {
        self.member_info.modifiers
    }
}
impl Member for Field {
    fn name(&self) -> &PooledStr {
        &self.member_info.name
    }

    fn declaring_weak(&self) -> &Weak<JavaClass> {
        &self.member_info.declaring
    }
}
/// 比较名字
impl PartialEq for Field {
    fn eq(&self, other: &Self) -> bool {
        self.member_info.name == other.member_info.name
    }
}
impl Eq for Field {}
impl PartialOrd for Field {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.member_info.name.partial_cmp(&other.member_info.name)
    }
}
impl Ord for Field {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.member_info.name.cmp(&other.member_info.name)
    }
}
impl Field {
    pub fn get_type(&self) -> &GenericType {
        &self.field_type
    }

    pub fn get_layout(&self) -> &Arc<dyn FieldLayoutTrait> {
        &self.layout
    }
}
impl AnnotatedElement for Field {
    fn get_annotation(
        &self,
        annotation_class: &JavaClassRef,
    ) -> Fallible<Option<Arc<jvm_core::OOPRef>>> {
        todo!()
    }

    fn get_annotations(&self) -> Fallible<Box<dyn Iterator<Item = Arc<jvm_core::OOPRef>>>> {
        todo!()
    }

    fn get_annotation_by_type(
        &self,
        annotation_class: &JavaClassRef,
    ) -> Fallible<Option<Arc<jvm_core::OOPRef>>> {
        todo!()
    }

    fn get_declared_annotation(
        &self,
        annotation_class: &JavaClassRef,
    ) -> Fallible<Option<Arc<jvm_core::OOPRef>>> {
        todo!()
    }

    fn get_declared_annotations(
        &self,
    ) -> Fallible<Box<dyn Iterator<Item = Arc<jvm_core::OOPRef>>>> {
        todo!()
    }

    fn get_declared_annotation_by_type(
        &self,
        annotation_class: &JavaClassRef,
    ) -> Fallible<Option<Arc<jvm_core::OOPRef>>> {
        todo!()
    }

    fn is_annotation_present(&self, annotation_class: &JavaClassRef) -> Fallible<bool> {
        todo!()
    }
}
