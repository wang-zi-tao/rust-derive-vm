use serde::{
    de::{SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use serde_json::{from_reader, to_writer};
use std::{
    collections::hash_map::DefaultHasher,
    fs::{create_dir_all, File},
    hash::{Hash, Hasher},
    io::{BufReader, BufWriter},
    path::PathBuf,
    str::FromStr,
};

use proc_macro2::TokenStream;
#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct CacheMetadata {
    macro_name: String,
    version: String,
}

impl CacheMetadata {
    pub fn new(macro_name: String, version: String) -> Self {
        Self { macro_name, version }
    }
}
#[derive(Serialize, Deserialize)]
pub struct WrapedTokenStream1(String);
pub struct WrapedTokenStream(TokenStream);
struct WrapedTokenStreamVisitor;
impl<'de> Visitor<'de> for WrapedTokenStreamVisitor {
    type Value = TokenStream;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("tuple struct WrapedTokenStream")
    }

    #[inline]
    fn visit_newtype_struct<E>(self, e: E) -> Result<Self::Value, E::Error>
    where
        E: Deserializer<'de>,
    {
        let field0: String = match <String as Deserialize>::deserialize(e) {
            Ok(value) => value,
            Err(e) => {
                return Err(e);
            }
        };
        Ok(TokenStream::from_str(&field0).expect("visit_newtype_struct"))
    }

    #[inline]
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let field0 = match SeqAccess::next_element::<String>(&mut seq)? {
            Some(value) => value,
            None => {
                return Err(serde::de::Error::invalid_length(0usize, &"tuple struct WrapedTokenStream1 with 1 element"));
            }
        };
        Ok(TokenStream::from_str(&field0).expect("visit_seq"))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(TokenStream::from_str(&v).expect("visit_seq"))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(TokenStream::from_str(v).expect("visit_seq"))
    }
}

impl<'de> Deserialize<'de> for WrapedTokenStream {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self(deserializer.deserialize_string(WrapedTokenStreamVisitor)?))
    }
}

impl Serialize for WrapedTokenStream {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_newtype_struct("WrapedTokenStream", &self.0.to_string())
    }
}

impl PartialEq for WrapedTokenStream {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_string() == other.0.to_string()
    }
}

impl Eq for WrapedTokenStream {}

impl Hash for WrapedTokenStream {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_string().hash(state);
    }
}
#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum CacheKey {
    ProcMacro(WrapedTokenStream),
    ProcMacroDerive(WrapedTokenStream),
    ProcMacroAttribute(WrapedTokenStream, WrapedTokenStream),
}

#[derive(Serialize, Deserialize)]
pub struct CacheEntry {
    meta: CacheMetadata,
    key: CacheKey,
    value: WrapedTokenStream,
}
#[macro_export]
macro_rules! cache_meta {
    () => {
        $crate::CacheMetadata::new(env!("CARGO_PKG_NAME").to_string(), env!("CARGO_PKG_VERSION").to_string())
    };
}
pub fn do_cache_macro(meta: CacheMetadata, input: CacheKey, f: impl FnOnce(&CacheKey) -> TokenStream) -> TokenStream {
    if option_env!("ENABLE_MACRO_CACHE").is_some() {
        let version: &str = env!("CARGO_PKG_VERSION");
        let key_json = serde_json::to_string(&input).unwrap();
        let mut hasher = DefaultHasher::default();
        meta.hash(&mut hasher);
        key_json.hash(&mut hasher);
        version.hash(&mut hasher);
        let out_dir = PathBuf::from_str("/tmp/rust/macro_cache").unwrap();
        if !out_dir.exists() {
            create_dir_all(&out_dir).unwrap();
        }
        let mut cache_file = out_dir;
        cache_file.push(&format!("{}-{}-{:16X}.json", &meta.macro_name, &meta.version, hasher.finish()));
        if cache_file.is_file() {
            let entry: CacheEntry = from_reader(BufReader::new(File::open(&cache_file).unwrap())).unwrap();
            if &entry.meta == &meta && &entry.key == &input {
                return entry.value.0;
            }
        }
        let output = f(&input);
        let entry = CacheEntry { meta, key: input, value: WrapedTokenStream(output) };
        to_writer(BufWriter::new(File::options().create(true).write(true).append(false).open(&cache_file).unwrap()), &entry).unwrap();
        entry.value.0
    } else {
        f(&input)
    }
}
pub fn cache_proc_macro(meta: CacheMetadata, input: TokenStream, f: impl FnOnce(TokenStream) -> TokenStream) -> TokenStream {
    do_cache_macro(meta, CacheKey::ProcMacro(WrapedTokenStream(input)), |key| match key {
        CacheKey::ProcMacro(input) => f(input.0.clone()),
        _ => unreachable!(),
    })
}
pub fn cache_proc_macro_derive(meta: CacheMetadata, input: TokenStream, f: impl FnOnce(&TokenStream) -> TokenStream) -> TokenStream {
    do_cache_macro(meta, CacheKey::ProcMacroDerive(WrapedTokenStream(input)), |key| match key {
        CacheKey::ProcMacroDerive(input) => f(&input.0),
        _ => unreachable!(),
    })
}
pub fn cache_proc_macro_attribute(
    meta: CacheMetadata,
    attr: TokenStream,
    item: TokenStream,
    f: impl FnOnce(&TokenStream, &TokenStream) -> TokenStream,
) -> TokenStream {
    do_cache_macro(meta, CacheKey::ProcMacroAttribute(WrapedTokenStream(attr), WrapedTokenStream(item)), |key| match key {
        CacheKey::ProcMacroAttribute(attr, item) => f(&attr.0, &item.0),
        _ => unreachable!(),
    })
}
