use std::sync::Arc;

pub trait DefaultArc {
    fn default_arc() -> Arc<Self>;
}
