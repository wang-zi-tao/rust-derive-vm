use syn_serde::json;
pub enum CacheEntry {
    ProcMacro { macro_name: String, version: String, key: String, value: String },
}
pub fn cache_proc_macro(input: TokenStream2, f: impl FnOnce(TokenStream2) -> TokenStream2) -> TokenStream2 {}
