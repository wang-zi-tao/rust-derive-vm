pub trait Singleton {
    fn get_instance<'l>() -> &'l Self;
}
pub trait SingletonDyn<T: 'static>: Singleton {
    fn get_instance<'l>() -> &'l T;
}
