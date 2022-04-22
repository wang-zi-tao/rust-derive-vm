use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc, RwLock},
};

use failure::{format_err, Fallible};
use getset::Getters;

pub struct VMState {
    tasks: RwLock<HashMap<TaskID, Arc<Task>>>,
}
impl VMState {
    pub fn get_tasks(&self) -> Fallible<Vec<Arc<Task>>> {
        let tasks = self.tasks.read().map_err(|e| format_err!("{}", e))?;
        let task_list: Vec<_> = tasks.values().cloned().collect();
        Ok(task_list)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskID(pub usize);
#[derive(Builder, Getters)]
#[builder(pattern = "owned")]
#[getset(get = "pub")]
pub struct Task {
    id: TaskID,
    open_safe_point: AtomicBool,
    gc_state: *mut usize,
}
