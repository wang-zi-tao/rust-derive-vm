use super::Environment;
use failure::format_err;
use util::Result;
use vm_core::{FieldRef, FieldTrait, JavaClassRef, JavaClassTrait, MemberTrait, Modifier};
pub fn resove_field(environment: &Environment, index: u16) -> Result<FieldRef> {
    let field_symbol = environment.constants.get(index as usize).ok_or_else(|| format_err!("invalid index of constant pool"))?.try_as_field_ref()?;
    let class = environment.class_loader.get_loaded_class(&field_symbol.class.symbol.name)?;
    let field = class.get_declared_field(field_symbol.name())?;
    let modifiers = field.modifiers();
    if modifiers.is_private() && !environment.class.equal((&*field.declaring().unwrap()) as &dyn JavaClassTrait)?
        || modifiers.is_protected() && !environment.class.is_assignable(&field.declaring().unwrap())?
        || !modifiers.is_public()
    {
        todo!()
    };
    todo!();
    Ok(field)
}
pub fn resolve_class_or_interface(environment: &Environment, index: u16) -> Result<JavaClassRef> {
    todo!()
}
