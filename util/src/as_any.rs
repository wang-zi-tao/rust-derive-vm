use std::{any::Any, sync::Arc};

pub trait AsAny: Any {
    fn as_any(&self) -> &(dyn Any);
    fn as_any_arc(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;
}
#[cfg(test)]
mod tests {
    use super::AsAny;
    use std::{
        any::{Any, TypeId},
        sync::Arc,
    };
    #[test]
    fn test() {
        trait T: AsAny {}
        struct S(Vec<u8>);
        impl AsAny for S {
            fn as_any(&self) -> &(dyn Any) {
                self
            }

            fn as_any_arc(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
                self
            }
        }
        impl T for S {}

        let s = S(Vec::new());
        let r = &s;
        let t: &dyn T = r;
        let type_id = t.type_id();
        assert_eq!(type_id, TypeId::of::<S>());
        let a: &dyn Any = t.as_any();
        let s1: &S = a.downcast_ref().unwrap();
        assert_eq!(s1 as *const S, r as *const S)
    }
}
