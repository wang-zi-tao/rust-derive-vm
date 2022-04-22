use crate::memory::{AssociateStubPool, Export};
use std::sync::Arc;
pub trait Deployment {
    fn get_AssociateStubPool(&self) -> Arc<AssociateStubPool>;
    fn get_export(&self) -> &Export;
}

