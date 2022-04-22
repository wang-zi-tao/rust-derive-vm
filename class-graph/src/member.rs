use std::sync::{Arc, Weak};

use crate::{annotations::Annotations, class::JavaClass};
use jvm_core::OOPTrait;
use util::PooledStr;
pub const DECLARED: i32 = 0;
pub const PUBLIC: i32 = 1;
pub trait Member: Send + Sync {
    fn name(&self) -> &PooledStr;
    fn declaring(&self) -> Option<Arc<JavaClass>>;
    fn is_synthetic(&self) -> bool {
        self.modifiers().is_synthetic()
    }
    fn declaring_weak(&self) -> &Weak<JavaClass>;
}
#[derive(Debug)]
pub struct MemberInfo {
    pub name: PooledStr,
    pub modifiers: i32,
    pub annotations: Annotations,
    pub declaring: Weak<JavaClass>,
}
impl MemberInfo {}

pub trait AccessibleObject {
    fn can_access(&self, object: Option<&dyn OOPTrait>) -> bool;
    fn is_accessible(&self) -> bool;
    fn try_set_accessible(&self) -> bool;
    fn set_accessible(&self, flat: bool) -> bool;
}
