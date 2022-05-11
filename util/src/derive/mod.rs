use syn_serde::json;
pub enum CacheEntry {
    ProcMacro { macro_name: String, version: String, key: String, value: String },
}
pub fn cache_proc_macro<I: Syn, O>(input: I, f: impl FnOnce(I) -> O) -> O {
    todo!();
}
