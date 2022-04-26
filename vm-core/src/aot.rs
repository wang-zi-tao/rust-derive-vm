use failure::Fallible;

pub trait AOTCurable {
    fn solidify(aot: impl AOTCompiler) -> Fallible<()>;
}
pub trait AOTCompiler {}
pub trait Linkable {
    fn symbol(&self) -> &str;
}
pub trait MaybeLinkable {
    fn symbol(&self) -> Option<&str>;
}
