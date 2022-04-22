use std::sync::Arc;

use crate::{class::JavaClass, class_loader::ClassLoader, executable::Executable, member::Member};
use failure::{format_err, Fallible};

macro_rules! err {
    ($message:expr) => {
        Err(format_err!($message))?;
    };
}
fn class_is_type_safe(class: &Arc<JavaClass>) -> Fallible<()> {
    class_name_is_safe(class)?;
    let class_loader = class
        .define_class_loader()
        .upgrade()
        .ok_or_else(|| format_err!("the defined class loader has droped"))?;
    if let Some(super_class) = class.super_class()? {
        if super_class.modifiers()?.is_final() {
            err!("super class is final");
        }
    } else {
        if &**class.name() != "Ljava/lang/Object;" {
            err!("only 'java.lang.Object' can have no super class");
        }
        if !class_loader.is_bootstrap() {
            err!("'java.lang.Object' should be load by boost ClassLoaderRef");
        }
    }
    for e in class.get_raw_executables()?.values() {
        if !e.is_constructor() {
            method_is_type_safe(class, &*e, &class_loader)?;
        }
    }
    Ok(())
}
fn class_name_is_safe(_class: &Arc<JavaClass>) -> Fallible<()> {
    // done in parsing CONSTANT_CLASS_INFO_IMPL
    Ok(())
}
fn method_is_type_safe(
    _class: &Arc<JavaClass>,
    method: &Executable,
    _class_loader: &ClassLoader,
) -> Fallible<()> {
    if method.get_extends().map(|m| m.modifiers().is_final()) == Some(true) {
        err!("can not override a final method");
    }
    let modifiers = method.modifiers();
    check_method_access_flags(modifiers)?;
    if modifiers.is_abstract() || modifiers.is_native() {
        Ok(())
    } else {
        // let code = method.get_code()?;
        todo!()
    }
}
fn check_method_access_flags(modifier: i32) -> Fallible<()> {
    if modifier & (!0x1fff) != 0 {
        Err(format_err!("illegal access flag of method:{}", modifier))
    } else {
        Ok(())
    }
}
