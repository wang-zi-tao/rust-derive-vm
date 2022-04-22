use classfile::attributes::Signature;
use failure::{format_err, Fallible};
use util::PooledStr;

use std::{
    convert::{TryFrom, TryInto},
    iter::Peekable,
    str::Bytes,
};

use crate::{
    class_loader::ClassLoader,
    generate_type::{ParameterizedType, PrimitiveType, Type, TypeTreeNode, WildcardType},
};
pub struct Context<'a, N: TypeTreeNode> {
    class_loader: &'a ClassLoader,
    type_tree_node: &'a N,
}
impl<'a, N: TypeTreeNode> Context<'a, N> {
    pub fn new(class_loader: &'a ClassLoader, type_tree_node: &'a N) -> Self {
        Self {
            class_loader,
            type_tree_node,
        }
    }
}
pub type Parser<'a> = Peekable<Bytes<'a>>;
fn parse_vec<'a, T>(
    iter: &mut Parser<'a>,
    parse: impl Fn(&mut Parser<'a>) -> Option<T>,
    predict: impl Fn(&mut Parser<'a>) -> bool,
) -> Option<Vec<T>> {
    let mut vec = Vec::new();
    while predict(iter) {
        vec.push(parse(iter)?);
    }
    Some(vec)
}
fn parse_vec_with_split<'a, T>(
    iter: &mut Parser<'a>,
    parse: impl Fn(&mut Parser<'a>) -> Option<T>,
    parse_split: impl Fn(&mut Parser<'a>) -> bool,
) -> Option<Vec<T>> {
    let mut vec = Vec::new();
    vec.push(parse(iter)?);
    while parse_split(iter) {
        vec.push(parse(iter)?);
    }
    Some(vec)
}
pub struct Identifier {
    pub string: PooledStr,
}
impl Identifier {
    fn parse<'a>(iter: &mut Parser<'a>) -> Option<Self> {
        let mut string = Vec::<u8>::new();
        while match iter.peek() {
            Some(b'.') | Some(b';') | Some(b'[') | Some(b'/') | Some(b'<') | Some(b'>')
            | Some(b':') => false,
            _ => true,
        } {
            string.push(iter.next().unwrap());
        }
        if string.len() > 0 {
            string.try_into().ok().map(|s| Self { string: s })
        } else {
            None
        }
    }

    pub fn to_str(&self) -> &str {
        &*self.string
    }
}
pub enum JavaTypeSignature {
    ReferenceTypeSignature(Box<ReferenceTypeSignature>),
    BaseType(BaseType),
}
impl JavaTypeSignature {
    fn parse<'a>(iter: &mut Parser<'a>) -> Option<Self> {
        match iter.peek() {
            Some(b'B') => Some(JavaTypeSignature::BaseType(BaseType::B)),
            Some(b'C') => Some(JavaTypeSignature::BaseType(BaseType::C)),
            Some(b'D') => Some(JavaTypeSignature::BaseType(BaseType::D)),
            Some(b'F') => Some(JavaTypeSignature::BaseType(BaseType::F)),
            Some(b'I') => Some(JavaTypeSignature::BaseType(BaseType::I)),
            Some(b'J') => Some(JavaTypeSignature::BaseType(BaseType::J)),
            Some(b'S') => Some(JavaTypeSignature::BaseType(BaseType::S)),
            Some(b'Z') => Some(JavaTypeSignature::BaseType(BaseType::Z)),
            Some(b'L') | Some(b'T') | Some(b'[') => {
                Some(JavaTypeSignature::ReferenceTypeSignature(Box::new(
                    ReferenceTypeSignature::parse(iter)?,
                )))
            }
            _ => None,
        }
    }

    pub fn to_type<'a, N: TypeTreeNode>(&self, context: &Context<'a, N>) -> Fallible<Type> {
        match self {
            JavaTypeSignature::ReferenceTypeSignature(r) => r.to_type(context),
            JavaTypeSignature::BaseType(b) => b.to_type(context),
        }
    }
}
pub enum BaseType {
    B,
    C,
    D,
    F,
    I,
    J,
    S,
    Z,
}
impl BaseType {
    pub fn to_type<'a, N: TypeTreeNode>(&self, _context: &Context<'a, N>) -> Fallible<Type> {
        Ok(Type::Primitive(match self {
            BaseType::B => PrimitiveType::Byte,
            BaseType::C => PrimitiveType::Char,
            BaseType::D => PrimitiveType::Double,
            BaseType::F => PrimitiveType::Float,
            BaseType::I => PrimitiveType::Int,
            BaseType::J => PrimitiveType::Long,
            BaseType::S => PrimitiveType::Short,
            BaseType::Z => PrimitiveType::Boolean,
        }))
    }
}
pub enum ReferenceTypeSignature {
    ClassTypeSignature(Box<ClassTypeSignature>),
    TypeVariableSignature(TypeVariableSignature),
    ArrayTypeSignature(ArrayTypeSignature),
}
impl ReferenceTypeSignature {
    pub fn to_type<'a, N: TypeTreeNode>(&self, context: &Context<'a, N>) -> Fallible<Type> {
        match self {
            ReferenceTypeSignature::ClassTypeSignature(c) => c.to_type(context),
            ReferenceTypeSignature::TypeVariableSignature(t) => t.to_type(context),
            ReferenceTypeSignature::ArrayTypeSignature(a) => a.inner.to_type(context)?.to_array(),
        }
    }

    fn parse<'a>(iter: &mut Parser<'a>) -> Option<Self> {
        match iter.peek() {
            Some(b'L') => Some(ReferenceTypeSignature::ClassTypeSignature(Box::new(
                ClassTypeSignature::parse(iter)?,
            ))),
            Some(b'T') => Some(ReferenceTypeSignature::TypeVariableSignature(
                TypeVariableSignature::parse(iter)?,
            )),
            Some(b'[') => Some(ReferenceTypeSignature::ArrayTypeSignature(
                ArrayTypeSignature::parse(iter)?,
            )),
            _ => None,
        }
    }
}
pub struct ClassTypeSignature {
    pub simple_name_list: Vec<Identifier>,
    pub type_arguments: Vec<TypeArgument>,
    pub class_type_suffix: Vec<ClassTypeSignatureSuffix>,
}
impl ClassTypeSignature {
    pub fn to_type<'a, N: TypeTreeNode>(&self, context: &Context<'a, N>) -> Fallible<Type> {
        self.class_type_suffix.iter().try_fold(
            Type::Parameterized(Box::new(ParameterizedType {
                owner: None,
                raw: {
                    let mut name = String::from('L');
                    let mut s = self.simple_name_list.iter().map(|i| &*i.string);
                    while let Some(n) = s.next() {
                        name.push_str(n);
                        if !s.is_empty() {
                            name.push('/');
                        }
                    }
                    name.push(';');
                    context.class_loader.get_class(&name.into())?
                },
                arguments: {
                    let mut arguments = Vec::with_capacity(self.type_arguments.len());
                    for type_argument in &self.type_arguments {
                        arguments.push(type_argument.to_type(context)?);
                    }
                    arguments
                },
            })),
            |t, s| {
                t.child_with_type_arguments(
                    context.class_loader.get_class(&s.identifier.string)?,
                    {
                        let mut args = Vec::with_capacity(s.arguments.len());
                        for argument in &s.arguments {
                            args.push(argument.to_type(context)?);
                        }
                        args
                    },
                )
            },
        )
    }

    fn parse<'a>(iter: &mut Parser<'a>) -> Option<Self> {
        if iter.next() != Some(b'L') {
            return None;
        }
        let this = Some(Self {
            simple_name_list: parse_vec_with_split(
                iter,
                |i| Identifier::parse(i),
                |i| {
                    if i.peek() == Some(&&b'/') {
                        i.next().unwrap();
                        true
                    } else {
                        false
                    }
                },
            )?,
            type_arguments: TypeArgument::parse_vec(iter)?,
            class_type_suffix: ClassTypeSignatureSuffix::parse_vec(iter)?,
        });
        if iter.next() != Some(b';') {
            return None;
        }
        this
    }
}
pub enum TypeArgument {
    ExtendsWildcard(ReferenceTypeSignature),
    SuperWildcard(ReferenceTypeSignature),
    Wildcard,
    NoWildcard(ReferenceTypeSignature),
}
impl TypeArgument {
    pub fn to_type<'a, N: TypeTreeNode>(&self, context: &Context<'a, N>) -> Fallible<Type> {
        // context.type_tree_node.get_type_variable(self.)
        match self {
            TypeArgument::ExtendsWildcard(r) => r
                .to_type(context)
                .map(|t| Type::Wildcard(Box::new(WildcardType::Extends(t)))),
            TypeArgument::SuperWildcard(r) => r
                .to_type(context)
                .map(|t| Type::Wildcard(Box::new(WildcardType::Super(t)))),
            TypeArgument::Wildcard => Ok(Type::Wildcard(Box::new(WildcardType::NoBound))),
            TypeArgument::NoWildcard(r) => r.to_type(context),
        }
    }

    fn parse<'a>(iter: &mut Parser<'a>) -> Option<Self> {
        match iter.peek() {
            Some(b'*') => {
                iter.next().unwrap();
                Some(TypeArgument::Wildcard)
            }
            Some(b'+') => {
                iter.next().unwrap();
                Some(TypeArgument::ExtendsWildcard(
                    ReferenceTypeSignature::parse(iter)?,
                ))
            }
            Some(b'-') => {
                iter.next().unwrap();
                Some(TypeArgument::SuperWildcard(ReferenceTypeSignature::parse(
                    iter,
                )?))
            }
            Some(_) => Some(TypeArgument::NoWildcard(ReferenceTypeSignature::parse(
                iter,
            )?)),
            None => None,
        }
    }

    fn parse_vec<'a>(iter: &mut Parser<'a>) -> Option<Vec<Self>> {
        if iter.next() != Some(b'<') {
            return None;
        }
        let mut vec = Vec::new();
        while iter.peek() != Some(&b'>') {
            vec.push(Self::parse(iter)?);
        }
        if iter.next() != Some(b'>') {
            return None;
        }
        Some(vec)
    }
}
pub struct ClassTypeSignatureSuffix {
    pub identifier: Identifier,
    pub arguments: Vec<TypeArgument>,
}
impl ClassTypeSignatureSuffix {
    fn parse<'a>(iter: &mut Parser<'a>) -> Option<Self> {
        if iter.next() != Some(b'.') {
            return None;
        }
        Some(Self {
            identifier: Identifier::parse(iter)?,
            arguments: TypeArgument::parse_vec(iter)?,
        })
    }

    fn parse_vec<'a>(iter: &mut Parser<'a>) -> Option<Vec<Self>> {
        let mut vec = Vec::new();
        while iter.peek() == Some(&b'.') {
            vec.push(Self::parse(iter)?);
        }
        Some(vec)
    }
}
pub struct TypeVariableSignature {
    pub name: Identifier,
}
impl TypeVariableSignature {
    pub fn to_type<'a, N: TypeTreeNode>(&self, context: &Context<'a, N>) -> Fallible<Type> {
        context
            .type_tree_node
            .get_type_variable(self.name.to_str())
            .ok_or_else(|| format_err!("NoneError"))
    }

    fn parse<'a>(iter: &mut Parser<'a>) -> Option<Self> {
        if iter.next() != Some(b'T') {
            return None;
        }
        let this = Some(Self {
            name: Identifier::parse(iter)?,
        });
        if iter.next() != Some(b';') {
            return None;
        }
        this
    }
}
pub struct ArrayTypeSignature {
    pub inner: JavaTypeSignature,
}
impl ArrayTypeSignature {
    fn parse<'a>(iter: &mut Parser<'a>) -> Option<Self> {
        if iter.next() != Some(b'[') {
            return None;
        }
        Some(Self {
            inner: JavaTypeSignature::parse(iter)?,
        })
    }
}
pub struct ClassSignature {
    pub type_parameters: Vec<TypeParameter>,
    pub super_class: ClassTypeSignature,
    pub interfaces: SuperinterfaceSignature,
}
impl ClassSignature {
    pub fn parse<'a>(iter: &mut Parser<'a>) -> Option<Self> {
        Some(Self {
            type_parameters: TypeParameter::parse_vec_option(iter)?,
            super_class: ClassTypeSignature::parse(iter)?,
            interfaces: SuperinterfaceSignature::parse(iter)?,
        })
    }

    pub fn from_attribute(attribute: &SignatureAttribute) -> Fallible<Self> {
        let mut iter = attribute.signature.bytes().peekable();
        Self::parse(&mut iter).ok_or_else(|| format_err!("NoneError"))
    }

    pub fn from_signature(signature: Signature) -> Fallible<Self> {
        let s = &*signature.signature;
        let mut iter = s.bytes().peekable();
        Self::parse(&mut iter).ok_or(format_err!("error on parse class signature"))
    }
}
pub struct TypeParameter {
    pub name: Identifier,
    pub bounds: ClassBound,
    pub super_interface: SuperinterfaceSignature,
}
impl TypeParameter {
    fn parse<'a>(iter: &mut Parser<'a>) -> Option<Self> {
        Some(Self {
            name: Identifier::parse(iter)?,
            bounds: ClassBound::parse(iter)?,
            super_interface: SuperinterfaceSignature::parse(iter)?,
        })
    }

    fn parse_vec_option<'a>(iter: &mut Parser<'a>) -> Option<Vec<Self>> {
        let mut vec = Vec::new();
        if iter.peek() == Some(&b'<') {
            iter.next().unwrap();
            while iter.peek() != Some(&b'>') {
                vec.push(TypeParameter::parse(iter)?);
            }
            iter.next();
        }
        Some(vec)
    }
}
pub struct ClassBound {
    pub bound: Option<ReferenceTypeSignature>,
}
impl ClassBound {
    fn parse<'a>(iter: &mut Parser<'a>) -> Option<Self> {
        if iter.next() != Some(b':') {
            return None;
        }
        if iter.peek() != Some(&b':') {
            Some(Self {
                bound: Some(ReferenceTypeSignature::parse(iter)?),
            })
        } else {
            Some(Self { bound: None })
        }
    }
}
pub struct SuperinterfaceSignature {
    pub bound: Vec<ReferenceTypeSignature>,
}
impl SuperinterfaceSignature {
    fn parse<'a>(iter: &mut Parser<'a>) -> Option<Self> {
        Some(Self {
            bound: parse_vec(
                iter,
                |i| {
                    if i.next() != Some(b':') {
                        None
                    } else {
                        Some(ReferenceTypeSignature::parse(i)?)
                    }
                },
                |i| i.peek() == Some(&b':'),
            )?,
        })
    }
}
pub struct MethodSignature {
    pub type_parameters: Vec<TypeParameter>,
    pub parameters: Vec<JavaTypeSignature>,
    pub result: ReturnValue,
    pub throws: Vec<ThrowsSignature>,
}
impl MethodSignature {
    fn parse<'a>(iter: &mut Parser<'a>) -> Option<Self> {
        Some(Self {
            type_parameters: TypeParameter::parse_vec_option(iter)?,
            parameters: {
                if iter.next() != Some(b'(') {
                    None
                } else {
                    let vec =
                        parse_vec(iter, JavaTypeSignature::parse, |i| i.peek() != Some(&b')'))?;
                    iter.next();
                    Some(vec)
                }
            }?,
            result: ReturnValue::parse(iter)?,
            throws: ThrowsSignature::parse_vec(iter)?,
        })
    }

    pub fn from_attribute(attribute: &SignatureAttribute) -> Fallible<Self> {
        let mut iter = attribute.signature.bytes().peekable();
        Self::parse(&mut iter).ok_or_else(|| format_err!("NoneError"))
    }

    pub fn from_signature(signature: Signature) -> Fallible<Self> {
        let s = &*signature.signature;
        let mut iter = s.bytes().peekable();
        Self::parse(&mut iter).ok_or(format_err!("error on parse class signature"))
    }
}
pub enum ReturnValue {
    Void,
    Type(JavaTypeSignature),
}
impl ReturnValue {
    fn parse<'a>(iter: &mut Parser<'a>) -> Option<Self> {
        match iter.peek() {
            Some(b'V') => Some(ReturnValue::Void),
            Some(_) => Some(ReturnValue::Type(JavaTypeSignature::parse(iter)?)),
            None => None,
        }
    }

    pub fn to_type<'a, N: TypeTreeNode>(&self, context: &Context<'a, N>) -> Fallible<Type> {
        match self {
            ReturnValue::Void => Ok(Type::Void),
            ReturnValue::Type(java_type) => java_type.to_type(context),
        }
    }
}
pub enum ThrowsSignature {
    Class(ClassTypeSignature),
    TypeVariable(TypeVariableSignature),
}
impl ThrowsSignature {
    fn parse<'a>(iter: &mut Parser<'a>) -> Option<Self> {
        if iter.next() != Some(b'^') {
            return None;
        }
        match iter.peek() {
            Some(b'L') => Some(ThrowsSignature::Class(ClassTypeSignature::parse(iter)?)),
            Some(b'T') => Some(ThrowsSignature::TypeVariable(TypeVariableSignature::parse(
                iter,
            )?)),
            _ => None,
        }
    }

    fn parse_vec<'a>(iter: &mut Parser<'a>) -> Option<Vec<Self>> {
        parse_vec(iter, ThrowsSignature::parse, |i| i.peek() == Some(&b'^'))
    }

    pub fn to_type<'a, N: TypeTreeNode>(&self, context: &Context<'a, N>) -> Fallible<Type> {
        match self {
            ThrowsSignature::Class(class) => class.to_type(context),
            ThrowsSignature::TypeVariable(type_variable) => type_variable.to_type(context),
        }
    }
}
pub struct FieldSignature {
    pub class: ReferenceTypeSignature,
}
impl FieldSignature {
    pub fn to_type<'a, N: TypeTreeNode>(&self, context: &Context<'a, N>) -> Fallible<Type> {
        self.class.to_type(context)
    }

    pub fn parse<'a>(iter: &mut Parser<'a>) -> Option<Self> {
        Some(Self {
            class: ReferenceTypeSignature::parse(iter)?,
        })
    }

    pub fn from_attribute(attribute: &SignatureAttribute) -> Fallible<Self> {
        let mut iter = attribute.signature.bytes().peekable();
        Self::parse(&mut iter).ok_or_else(|| format_err!("NoneError"))
    }

    pub fn from_signature(signature: Signature) -> Fallible<Self> {
        let s = &*signature.signature;
        let mut iter = s.bytes().peekable();
        Self::parse(&mut iter).ok_or(format_err!("error on parse class signature"))
    }
}
