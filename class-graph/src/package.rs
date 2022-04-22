use crate::{annotations::Annotations, class_loader::ClassLoader};
use std::{fmt::Debug, hash::Hash, sync::Arc};
use util::PooledStr;
#[derive(Debug)]
pub struct Package {
    name: PooledStr,
    class_loader: Arc<ClassLoader>,
    implementation_title: Option<PooledStr>,
    implementation_vendor: Option<PooledStr>,
    implementation_version: Option<PooledStr>,
    specification_title: Option<PooledStr>,
    specification_vendor: Option<PooledStr>,
    specification_version: Option<PooledStr>,
    annotations: Annotations,
}
pub type PackageRef = Arc<Package>;
impl Package {
    pub fn new(class_loader: Arc<ClassLoader>, name: PooledStr) -> Self {
        Self {
            name,
            class_loader,
            implementation_title: None,
            implementation_vendor: None,
            implementation_version: None,
            specification_title: None,
            specification_vendor: None,
            specification_version: None,
            annotations: Annotations::default(),
        }
    }
}
