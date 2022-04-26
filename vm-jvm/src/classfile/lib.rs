use failure::{self, Fallible};
#[macro_use]
extern crate failure_derive;
use attributes::{Attribute, Code, Location};
use constants::{parse_constant_pool, Constant, ConstantClassInfoImpl};
use core::fmt::Debug;
use failure::format_err;
use std::{collections::HashMap, rc::Rc, sync::Arc};
use util::{self, PooledStr};
pub mod attributes;
pub mod constants;
pub mod parser;
pub mod symbol;
pub use crate::parser::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(u16, u16);
impl Version {
    pub fn major(&self) -> u16 {
        self.0
    }

    pub fn minor(&self) -> u16 {
        self.1
    }
}
pub struct ClassFile {
    pub magic: u32,
    pub minor_version: u16,
    pub major_version: u16,
    pub constant_pool: Vec<Constant>,
    pub access_flags: u16,
    pub this_class: Rc<ConstantClassInfoImpl>,
    pub super_class: Option<Rc<ConstantClassInfoImpl>>,
    pub interface: Vec<Rc<ConstantClassInfoImpl>>,
    pub fields: Vec<Field>,
    pub methods: Vec<Method>,
    pub attributes: HashMap<PooledStr, Attribute>,
}
impl ClassFile {
    pub fn new(file: &[u8]) -> Fallible<Self> {
        Self::parser(Parser::from_slice(file))
    }

    fn parser(parser: Parser<'_>) -> Fallible<ClassFile> {
        let mut parser = parser;
        let magic = parser.next_u32()?;
        let minor_version = parser.next_u16()?;
        let major_version = parser.next_u16()?;
        let version = Version(major_version, minor_version);
        let constant_pool = parse_constant_pool(version, &mut parser)?;
        let access_flags = parser.next_u16()?;
        let this_class = parser.next_constant_index(&constant_pool)?.try_as_class()?;
        let super_class = match parser.next_u16()? {
            0 => None,
            i => Some(constant_pool.get(i as usize).ok_or_else(|| format_err!("Illegal super_class index"))?.try_as_class()?),
        };
        let interface_count = parser.next_u16()? as usize;
        let mut interface = Vec::with_capacity(interface_count);
        for _ in 0..interface_count {
            interface.push(parser.next_constant_index(&constant_pool)?.try_as_class()?)
        }
        let field_count = parser.next_u16()? as usize;
        let mut fields = Vec::with_capacity(field_count);
        for _ in 0..field_count {
            fields.push(Field::parser(&mut parser, &constant_pool, version)?);
        }
        let method_count = parser.next_u16()? as usize;
        let mut methods = Vec::with_capacity(method_count);
        for _ in 0..method_count {
            // println!("{},{},{:#?}",method_count,parser.len(),&methods);
            methods.push(Method::parser(&mut parser, &constant_pool, version)?);
        }
        let attributes = Attribute::parse_hashmap(&mut parser, &constant_pool, version, Location::ClassFile)?;
        Ok(Self { magic, minor_version, major_version, constant_pool, access_flags, this_class, super_class, interface, fields, methods, attributes })
    }
}
pub struct ClassIR {
    pub constant_pool: Arc<Vec<Constant>>,
}
#[derive(Debug)]
pub struct Field {
    pub access_flags: u16,
    pub name: PooledStr,
    pub descriptor: PooledStr,
    pub attributes: HashMap<PooledStr, Attribute>,
}
impl Field {
    pub fn parser(parser: &mut Parser<'_>, constant_pool: &Vec<Constant>, version: Version) -> Fallible<Self> {
        let access_flags = parser.next_u16()?;
        let name = parser.next_constant_index(constant_pool)?.try_as_field_name_in_utf8()?;
        let descriptor = parser.next_constant_index(constant_pool)?.try_as_field_descriptor()?;
        let attributes = Attribute::parse_hashmap(parser, constant_pool, version, Location::Field)?;
        Ok(Self { access_flags, name, descriptor, attributes })
    }
}
// #[derive(Fail, Debug)]
// #[fail(display = "Illegal class file structure.")]
// pub struct ClassFileFormatError {}
#[derive(Debug)]
pub struct Method {
    pub access_flags: u16,
    pub name: PooledStr,
    pub descriptor: PooledStr,
    pub attributes: HashMap<PooledStr, Attribute>,
}
impl Method {
    pub fn parser(parser: &mut Parser<'_>, constant_pool: &Vec<Constant>, version: Version) -> Fallible<Self> {
        let access_flags = parser.next_u16()?;
        let name = parser.next_constant_index(constant_pool)?.try_as_method_name_in_utf8()?;
        let descriptor = parser.next_constant_index(constant_pool)?.try_as_method_descriptor()?;
        let attributes = Attribute::parse_hashmap(parser, constant_pool, version, Location::Method)?;
        Ok(Self { access_flags, name, descriptor, attributes })
    }
}
pub struct MethodIR {
    pub constant_pool: Arc<Vec<Constant>>,
    pub code: Code,
}
extern crate test_resources;
#[cfg(test)]
pub mod tests {
    use std::path::{Path, PathBuf};

    use failure::{format_err, Fallible};
}
