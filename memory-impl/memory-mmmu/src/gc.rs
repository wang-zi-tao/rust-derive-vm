use async_trait::async_trait;
use std::{
    collections::HashSet,
    ptr::{null_mut, NonNull},
    sync::{
        atomic::{AtomicBool, AtomicPtr, AtomicU32},
        Arc,
    },
};
use util::CowArc;

use crossbeam::atomic::AtomicConsume;
use failure::{Fallible};
use vm_core::VMState;
use os::mem::{MappedVM, Page};
use tokio::sync::{Semaphore};

use crate::{heap::OBJECT_MIN_ALIAS, plan::make_plan, RegistedType};

pub struct GCController {
    trackers: Vec<Box<dyn GCTracker>>,
    logger: Box<dyn GCLogger>,
    stack_scanner: Box<dyn GCStackScanner>,
    root_scanner: Box<dyn GCRootScanner>,
    cleaner: Box<dyn GCCleaner>,
    mark_set: Box<GCMarkSet>,
    safe_point_trigger: SafePointTrigger,
    vm_state: Arc<VMState>,
    tasks_count: AtomicU32,
}
impl GCController {
    async fn gc(&mut self) -> Fallible<()> {
        self.logger.on_gc()?;
        let mut gc_plan = make_plan();
        self.logger.on_plan(&gc_plan)?;
        let markset = self.get_markset(&gc_plan).await?;
        let _tasks = self.vm_state.get_tasks()?;
        self.safe_point_trigger.trigger(&gc_plan, &mut self.stack_scanner, &self.tasks_count).await?;
        self.root_scanner.scan(&mut gc_plan).await?;
        // self.heap_scanner.scan(&mut gc_plan).await?;
        self.cleaner.clean(&mut gc_plan, &markset).await?;
        Ok(())
    }

    async fn make_plan(&mut self) -> Fallible<GCPlan> {
        todo!()
    }

    async fn get_markset(&mut self, _gc_plan: &GCPlan) -> Fallible<GCMarkSet> {
        todo!()
    }

    fn registers_gc_tracker(&mut self, gc_tracker: Box<dyn GCTracker>) -> Fallible<()> {
        self.trackers.push(gc_tracker);
        Ok(())
    }
}
#[async_trait]
pub trait GCCleaner {
    async fn clean(&self, gc_plan: &mut GCPlan, markset: &GCMarkSet) -> Fallible<()>;
}
pub trait GCTracker {
    fn on_gc(&self) -> Fallible<()>;
}
pub trait GCLogger {
    fn on_gc(&self) -> Fallible<()>;
    fn on_plan(&self, gc_plan: &GCPlan) -> Fallible<()>;
    fn on_gc_finish(&self) -> Fallible<()>;
}
#[async_trait]
pub trait GCRootScanner {
    async fn scan(&self, gc_plan: &mut GCPlan) -> Fallible<()>;
}
pub trait GCStackScanner {
    fn scan_current_stack(&self);
}
#[derive(Getters, Default)]
#[getset(get = "pub(crate)")]
pub struct GCPlan {
    pub(crate) clean_types: HashSet<CowArc<'static, RegistedType>>,
    pub(crate) scan_types: HashSet<CowArc<'static, RegistedType>>,
}
impl GCPlan {}
pub struct GCMark(u8);
pub struct GCMarkSet {
    ptr: *mut GCMark,
    mem: MappedVM,
}
impl GCMarkSet {
    fn mark(&self, object_ptr: *const ()) {
        unsafe { self.get_marking_ptr(object_ptr).write(GCMark(1)) }
    }

    fn is_marked(&self, object_ptr: *const ()) -> GCMark {
        unsafe { self.get_marking_ptr(object_ptr).read_volatile() }
    }

    fn get_marking_ptr(&self, object_ptr: *const ()) -> *mut GCMark {
        unsafe { self.ptr.offset((object_ptr as isize) / OBJECT_MIN_ALIAS as isize) }
    }
}
pub struct SafePointTrigger {
    pages: [NonNull<Page>; 2],
}
lazy_static! {
    static ref SAFE_POINT_SEMAPHORE: Semaphore = Semaphore::new(0);
}
static SAFE_POINT_SWITCH: AtomicBool = AtomicBool::new(false);
static SAFE_POINT_SCANNER: AtomicPtr<Box<dyn GCStackScanner>> = AtomicPtr::new(null_mut());
extern "C" fn do_safe_point(_signal: i32) {
    if !SAFE_POINT_SWITCH.load(std::sync::atomic::Ordering::Acquire) {
        return;
    }
    SAFE_POINT_SEMAPHORE.add_permits(1);
    if let Some(scanner) = unsafe { SAFE_POINT_SCANNER.load(std::sync::atomic::Ordering::Acquire).as_ref() } {
        scanner.scan_current_stack();
    }
}
impl SafePointTrigger {
    fn enable_safe_point(&mut self, _index: usize) -> Fallible<()> {
        todo!();
        Ok(())
    }

    fn set_safe_point_callback() -> Fallible<()> {
        unsafe {
            let _ret = libc::signal(libc::SIGSEGV, do_safe_point as usize);
        }
        Ok(())
    }

    async fn trigger(&mut self, _plan: &GCPlan, scanner: &mut Box<dyn GCStackScanner>, task_count: &AtomicU32) -> Fallible<()> {
        SAFE_POINT_SCANNER.store(scanner as *mut Box<_>, std::sync::atomic::Ordering::Release);
        SAFE_POINT_SWITCH.store(true, std::sync::atomic::Ordering::Release);
        let _add_permits_count = task_count.load_consume();
        SAFE_POINT_SEMAPHORE.acquire().await?.forget();
        SAFE_POINT_SWITCH.store(false, std::sync::atomic::Ordering::Release);
        Ok(())
    }
}
