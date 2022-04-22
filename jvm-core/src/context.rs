use std::sync::Arc;

use getset::Getters;

pub trait Context {}
pub trait SubContext: Context {}
/// 作用域
#[derive(Builder, Getters)]
#[builder(pattern = "owned")]
#[getset(get = "pub")]
pub struct Scope {
    parent: Arc<Scope>,
}
