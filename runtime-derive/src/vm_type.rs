//! ```rust
//! #[derive(TypeDeclaration)]
//! #[make_type(value_mask=[0..60],tag_mask=[60..64])]
//! enum LuaObjectRef{
//!   Boolean(bool),
//!   Integer(i64),
//!   BigInt(Ref<f64>),
//!   Float(f64),
//!   BigFloat(Ref<f64>),
//!   String(LuaString),
//!   Function,
//!   Table(std::option::Option<&LuaObject>),
//! }
//! #[derive(TypeDeclaration)]
//! struct Object{
//!   #[make_type(mask=[0..48],assert_size=8)]
//!   metadata:ObjectMatedata,
//!   meta_field:LuaObjectRef,
//!   #[make_type(mask=[0..48])]
//!   slow_fields:&[LuaObjectRef],
//!   #[make_type(const,compose,bit_offset=56,mask=[56..64])]
//!   fast_len:u8,
//!   #[make_type(length=fast_len)]
//!   fast_fields:[LuaObjectRef],
//! }
//! ```
//! ```
//! struct LuaObjectRefValue([u8,8])
//! const_assert!(ObjectMatedata::layout.size()<=8);
//! impl TypeImplement for LuaObjectRefValue{
//!   type RustType=LuaObjectRef;
//!   const ty:vm_core::Type;
//!   fn encode(rust_data:Self::RustType)->Self{
//!     ...
//!   }
//!   fn decode(self)->Self::RustType{
//!     ...
//!   }
//! }
//! enum __DeriveEncodeBoolean{};
//! impl __DeriveEncodeBoolean
//! impl TypeEnum for LuaObjectRef{
//!    ...
//! }
//! impl LuaObjectRef{
//!   fn encodeBoolean(f1:bool)->Self{
//!     ...
//!   }
//!   type
//!   fn decodeBoolean(self)->std::option::Option<bool>{
//!     ...
//!   }
//! }
//! struct ObjectValue([u8]);
//! impl TypeDeclaration for Object{
//!   type Impl=ObjectValue;
//! }
//! impl TypeImplement for ObjectValue{
//!   type Declaration=Object;
//!   fn asType()->&'static vm_core::Type{
//!     ...
//!   }
//!   fn encode(rust_data:Self::Declaration)->Self{
//!     ...
//!   }
//!   fn decode(self)->Self::Declaration{
//!     ...
//!   }
//! }
//! impl ObjectValue{
//!   pub const TYPE:vm_core::Type=vm_core::Type::Turple(...);
//!   fn get_metadata(&self)->ObjectMatedata{
//!     ...
//!   }
//!   fn set_metadata(&self,v:ObjectMatedata){
//!     ...
//!   }
//!   ...
//! }
//! ```

use std::{mem::replace, num::NonZeroIsize};

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{
    bracketed, ext::IdentExt, parenthesized, parse::Parse, parse2, parse_macro_input, token::Bracket, Data, DataEnum, DataStruct, DeriveInput, Error, Fields,
    Ident, LitInt, Result, WhereClause,
};
use synstructure::Structure;

pub fn make_type(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let structure = Structure::new(&input);
    || -> Result<TokenStream2> {
        let generated;
        match &input.data {
            Data::Struct(s) => {
                generated = derive_struct(s, &structure)?;
            }
            Data::Enum(e) => {
                generated = derive_enum(e, &structure)?;
            }
            Data::Union(_) => {
                return Err(Error::new(structure.ast().ident.span(), "union is not supported"));
            }
        }
        Ok(generated)
    }()
    .map(TokenStream::from)
    .unwrap_or_else(|e| TokenStream::from(e.into_compile_error()))
}
struct MaskAttribute {
    _wrap: Bracket,
    start: LitInt,
    _split: Token!(..),
    end: LitInt,
}
impl MaskAttribute {
    pub fn to_usize(&self) -> usize {
        (std::usize::MAX << self.start()) & (std::usize::MAX >> (64 - self.end()))
    }

    pub fn start(&self) -> usize {
        self.start.base10_parse().unwrap()
    }

    pub fn end(&self) -> usize {
        self.end.base10_parse().unwrap()
    }
}
impl Parse for MaskAttribute {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let content;
        let r = Ok(Self { _wrap: bracketed!(content in input), start: content.parse()?, _split: content.parse()?, end: content.parse()? });
        if !content.is_empty() {
            return Err(syn::Error::new(content.span(), "unexpected tokens"));
        }
        r
    }
}
#[derive(Default)]
struct FieldAttribute {
    assert_size: std::option::Option<usize>,
    mask: std::option::Option<MaskAttribute>,
    bit_offset: std::option::Option<NonZeroIsize>,
    is_const: bool,
    is_unsized: bool,
}
impl Parse for FieldAttribute {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let content;
        let _wrap = parenthesized!(content in input);
        let mut assert_size = None;
        let mut mask = None;
        let mut bit_offset = None;
        let mut is_const = None;
        let mut is_unsized = None;
        while !content.is_empty() {
            let key: Ident = content.call(Ident::parse_any)?;
            match key.to_string().as_str() {
                "assert_size" => {
                    let _eq: Token!(=) = content.parse()?;
                    let value: LitInt = content.parse()?;
                    if assert_size.replace(value.base10_parse()?).is_some() {
                        return Err(Error::new(key.span(), "Duplicate metadata"));
                    }
                }
                "mask" => {
                    let _eq: Token!(=) = content.parse()?;
                    let value: MaskAttribute = content.parse()?;
                    if mask.replace(value).is_some() {
                        return Err(Error::new(key.span(), "Duplicate metadata"));
                    }
                }
                "bit_offset" => {
                    let _eq: Token!(=) = content.parse()?;
                    let value: LitInt = content.parse()?;
                    if bit_offset.replace(value.base10_parse()?).is_some() {
                        return Err(Error::new(key.span(), "Duplicate metadata"));
                    }
                }
                "const" => {
                    if is_const.replace(true).is_some() {
                        return Err(Error::new(key.span(), "Duplicate metadata"));
                    }
                }
                "unsized" => {
                    if is_unsized.replace(true).is_some() {
                        return Err(Error::new(key.span(), "Duplicate metadata"));
                    }
                }
                o => return Err(Error::new(key.span(), format!("unknown field metadata `{}`", o))),
            }
            if !content.is_empty() {
                let _split: Token!(,) = content.parse()?;
            }
        }
        Ok(Self { assert_size, mask, bit_offset, is_const: is_const.unwrap_or(false), is_unsized: is_unsized.unwrap_or(false) })
    }
}
#[derive(Default)]
struct StructAttribute {
    make_instruction: bool,
}

impl Parse for StructAttribute {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let content;
        let _wrap = parenthesized!(content in input);
        let mut make_instruction = false;
        while !content.is_empty() {
            let key: Ident = content.call(Ident::parse_any)?;
            match key.to_string().as_str() {
                "make_instruction" => {
                    if replace(&mut make_instruction, true) {
                        return Err(Error::new(key.span(), "Duplicate metadata"));
                    }
                }
                o => return Err(Error::new(key.span(), format!("unknown struct metadata `{}`", o))),
            }
            if !content.is_empty() {
                let _split: Token!(,) = content.parse()?;
            }
        }
        Ok(Self { make_instruction })
    }
}
#[derive(Default)]
struct EnumAttribute {
    tag_mask: std::option::Option<MaskAttribute>,
    tag_start: std::option::Option<usize>,
    tag_offset: std::option::Option<i8>,
    make_instruction: bool,
}
impl Parse for EnumAttribute {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let content;
        let _wrap = parenthesized!(content in input);
        let mut tag_mask: std::option::Option<MaskAttribute> = None;
        let mut tag_start: std::option::Option<usize> = None;
        let mut tag_offset: std::option::Option<i8> = None;
        let mut make_instruction = false;
        while !content.is_empty() {
            let key: Ident = content.call(Ident::parse_any)?;
            match key.to_string().as_str() {
                "tag_start" => {
                    let _eq: Token!(=) = content.parse()?;
                    let value: LitInt = content.parse()?;
                    if tag_start.replace(value.base10_parse()?).is_some() {
                        return Err(Error::new(key.span(), "Duplicate metadata"));
                    }
                }
                "tag_offset" => {
                    let _eq: Token!(=) = content.parse()?;
                    let value: LitInt = content.parse()?;
                    if tag_offset.replace(value.base10_parse()?).is_some() {
                        return Err(Error::new(key.span(), "Duplicate metadata"));
                    }
                }
                "tag_mask" => {
                    let _eq: Token!(=) = content.parse()?;
                    let value: MaskAttribute = content.parse()?;
                    if tag_mask.replace(value).is_some() {
                        return Err(Error::new(key.span(), "Duplicate metadata"));
                    }
                }
                "make_instruction" => {
                    if replace(&mut make_instruction, true) {
                        return Err(Error::new(key.span(), "Duplicate metadata"));
                    }
                }
                o => return Err(Error::new(key.span(), format!("unknown enum metadata `{}`", o))),
            }
            if !content.is_empty() {
                let _split: Token!(,) = content.parse()?;
            }
        }
        Ok(Self { tag_mask, tag_start, tag_offset, make_instruction })
    }
}
fn derive_enum(s: &DataEnum, structure: &Structure) -> Result<TokenStream2> {
    let ast = structure.ast();
    let name = &ast.ident;
    let vis = &ast.vis;
    // let mut instructions=Vec::new();
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let impl_type_name = format_ident!("{}Impl", &name);
    let trait_type_name = format_ident!("{}Trait", &name);
    let ty_generics_trubofish = ty_generics.as_turbofish();
    let layout_fn_name = format_ident!("{}_layout", util::camel_case_ident_to_snake_case_ident(&name.to_string()), span = name.span());
    let tag_layout_fn_name = format_ident!("{}_tag_layout", util::camel_case_ident_to_snake_case_ident(&name.to_string()), span = name.span());
    let mod_name = format_ident!("{}", util::camel_case_ident_to_snake_case_ident(&name.to_string()), span = name.span());

    let mut variant_structs = Vec::new();
    let mut variant_functions = Vec::new();
    let mut variant_declarations = Vec::new();
    let mut associations_declaration = Vec::new();
    let mut associations_alias = Vec::new();
    let mut associations = Vec::new();
    let mut content_layout = quote! {vm_core::TypeLayout::new()};
    let tag_count = s.variants.len();
    let tag_size: usize = match tag_count {
        0..=255 => 1,
        256..=65535 => 2,
        65536..=0xffff_ffff_ffff_ffff => 4,
        _ => return Err(Error::new(name.span(), "too many variants")),
    };
    let ast = structure.ast();
    let enum_attr: EnumAttribute = ast
        .attrs
        .iter()
        .find(|attr| attr.path.get_ident().map(|i| i.to_string().as_str() == "make_type").unwrap_or(false))
        .map(|attr| parse2(attr.tokens.clone()))
        .unwrap_or_else(|| Ok(Default::default()))?;
    for (variant_index, variant) in s.variants.iter().enumerate() {
        let variant_name = &variant.ident;
        let variant_type;
        let variant_declaration;
        let fields = &variant.fields;
        match fields {
            Fields::Unit => {
                variant_type = quote! {()};
                variant_declaration = quote! {vm_core::Type::Tuple(vm_core::Tuple::Normal(runtime::_util::CowArc::Ref(&[])))};
            }
            Fields::Named(_) | Fields::Unnamed(_) => match fields.len() {
                0 => {
                    variant_type = quote! {()};
                    variant_declaration = quote! {vm_core::Type::Tuple(vm_core::Tuple::Normal(runtime::_util::CowArc::Ref(&[])))};
                }
                1 => {
                    let field = fields.iter().next().unwrap();
                    let ty = &field.ty;
                    variant_type = quote! {#ty};
                    variant_declaration = quote! {<#ty as vm_core::TypeDeclaration>::TYPE};
                    content_layout = quote! {
                      #content_layout.union(<#variant_type as vm_core::TypeDeclaration>::LAYOUT)
                    }
                }
                _ => {
                    let variant_struct_name = format_ident!("{}{}", &name, variant_name);
                    variant_type = quote! {#variant_struct_name};
                    variant_declaration = quote! {#variant_struct_name::TYPE};
                    variant_structs.push(derive_fields(
                        &variant.fields,
                        structure,
                        variant_struct_name.clone(),
                        Some(StructAttribute { make_instruction: enum_attr.make_instruction }),
                    )?);
                    variant_structs.push(quote! {#[repr(C)] #vis struct #variant_struct_name #fields;});
                    content_layout = quote! {
                      #content_layout.union(<#variant_type as vm_core::TypeDeclaration>::LAYOUT)
                    }
                }
            },
        };
        variant_declarations.push(variant_declaration);
        let variant_name_in_snake_case = util::camel_case_ident_to_snake_case_ident(&variant_name.to_string());
        let variant_name_in_upper_snake_case = util::camel_case_ident_to_upper_snake_case_ident(&variant_name.to_string());
        let encode_fn_ident = format_ident!("encode_{}", &variant_name_in_snake_case, span = variant_name.span());
        let read_fn_ident = format_ident!("read_{}", &variant_name_in_snake_case, span = variant_name.span());
        let write_fn_ident = format_ident!("write_{}", &variant_name_in_snake_case, span = variant_name.span());
        let tag_ident = format_ident!("TAG_{}", &variant_name_in_upper_snake_case, span = variant_name.span());
        variant_functions.push(quote! {
          pub const #tag_ident:usize=#variant_index;
          #[inline(always)]
          pub fn #encode_fn_ident(value:<#variant_type as vm_core::TypeDeclaration>::Impl)->Self{
            use std::mem::MaybeUninit;
            unsafe{
              let mut this = MaybeUninit::<Self>::zeroed();
              this.as_mut_ptr().cast::<<#variant_type as vm_core::TypeDeclaration>::Impl>().write(value);
              #tag_layout_fn_name #ty_generics_trubofish().encode(#variant_index,this.as_mut_ptr().cast());
              MaybeUninit::assume_init(this)
            }
          }
          #[inline(always)]
          pub fn #write_fn_ident(&mut self,value:<#variant_type as vm_core::TypeDeclaration>::Impl){
            unsafe{
              (self as *mut Self).cast::<<#variant_type as vm_core::TypeDeclaration>::Impl>().write(value);
              #tag_layout_fn_name #ty_generics_trubofish().encode(#variant_index,(self as *mut Self).cast());
            }
          }
          #[inline(always)]
          pub fn #read_fn_ident(&mut self)->std::option::Option<<#variant_type as vm_core::TypeDeclaration>::Impl>{
              unsafe {
                  if #tag_layout_fn_name #ty_generics_trubofish().decode((self as *mut Self).cast()) == #variant_index{
                      None
                  }else{
                      let mut value:<#variant_type as vm_core::TypeDeclaration>::Impl = (self as *mut Self).cast::<<#variant_type as vm_core::TypeDeclaration>::Impl>().read();
                      #tag_layout_fn_name #ty_generics_trubofish().earse(&mut value as *mut _ as *mut u8);
                      Some(value)
                  }
              }
          }
        });
        if enum_attr.make_instruction {
            let decode_variant = format_ident!("Decode{}Unchecked", variant_name, span = variant_name.span());
            let encode_variant = format_ident!("Encode{}", variant_name, span = variant_name.span());
            let check_variant = format_ident!("Is{}", variant_name, span = variant_name.span());
            associations_declaration.push(quote! {
                type #decode_variant : runtime::instructions::Instruction;
                type #encode_variant : runtime::instructions::Instruction;
                type #check_variant : runtime::instructions::Instruction;
            });
            associations.push(quote! {
                type #decode_variant = runtime_extra::DecodeVariantUncheckedFor<Self,#variant_type,{#variant_index as i64}>;
                type #encode_variant = runtime_extra::EncodeVariantFor<Self,#variant_type,{#variant_index as i64}>;
                type #check_variant = runtime_extra::GetTagAndCheckFor<Self,{#variant_index as i64}>;
            });
            associations_alias.push(quote! {
                pub type #decode_variant #impl_generics=<super::#name #ty_generics as super::#trait_type_name #ty_generics>::#decode_variant;
                pub type #encode_variant #impl_generics=<super::#name #ty_generics as super::#trait_type_name #ty_generics>::#encode_variant;
                pub type #check_variant #impl_generics=<super::#name #ty_generics as super::#trait_type_name #ty_generics>::#check_variant;
            });
            if enum_attr.tag_mask.is_none() && enum_attr.tag_start.is_none() && enum_attr.tag_offset.is_none() {
                let locate_variant = format_ident!("Locate{}Unchecked", variant_name, span = variant_name.span());
                let read_variant = format_ident!("Read{}Unchecked", variant_name, span = variant_name.span());
                let write_variant = format_ident!("Write{}", variant_name, span = variant_name.span());
                associations_declaration.push(quote! {
                    type #locate_variant : runtime::instructions::Instruction;
                    type #read_variant : runtime::instructions::Instruction;
                    type #write_variant : runtime::instructions::Instruction;
                });
                associations.push(quote! {
                    type #locate_variant = runtime_extra::LocateVariantUncheckedFor<Self,#variant_type,{#variant_index as i64}>;
                    type #read_variant = runtime_extra::ReadVariantUncheckedFor<Self,#variant_type,{#variant_index as i64}>;
                    type #write_variant = runtime_extra::WriteVariantFor<Self,#variant_type,{#variant_index as i64}>;
                });
                associations_alias.push(quote! {
                    pub type #locate_variant #impl_generics=<super::#name #ty_generics as super::#trait_type_name #ty_generics>::#locate_variant;
                    pub type #read_variant #impl_generics=<super::#name #ty_generics as super::#trait_type_name #ty_generics>::#read_variant;
                    pub type #write_variant #impl_generics=<super::#name #ty_generics as super::#trait_type_name #ty_generics>::#write_variant;
                });
            }
        }
    }
    if enum_attr.make_instruction {
        if enum_attr.tag_mask.is_some() || enum_attr.tag_start.is_some() || enum_attr.tag_offset.is_some() {
            associations_declaration.push(quote! {
                type GetTag : runtime::instructions::Instruction;
            });
            associations.push(quote! {
                type GetTag = runtime_extra::GetTagFor<Self>;
            });
            associations_alias.push(quote! {
                pub type GetTag #impl_generics=<super::#name #ty_generics as super::#trait_type_name #ty_generics>::GetTag;
            });
        } else {
            associations_declaration.push(quote! {
                type ReadTag : runtime::instructions::Instruction;
                type WriteTag : runtime::instructions::Instruction;
            });
            associations.push(quote! {
                type ReadTag = runtime_extra::ReadTagFor<Self>;
                type WriteTag = runtime_extra::WriteTagFor<Self>;
            });
            associations_alias.push(quote! {
                pub type ReadTag #impl_generics=<super::#name #ty_generics as super::#trait_type_name #ty_generics>::ReadTag;
                pub type WriteTag #impl_generics=<super::#name #ty_generics as super::#trait_type_name #ty_generics>::WriteTag;
            });
        }
    }
    let tag_offset = enum_attr.tag_offset.unwrap_or(0);
    let tag_layout;
    let mut enum_layout = quote! {
      #content_layout
    };
    if let Some(tag_start) = enum_attr.tag_start {
        let tag_start = tag_start;
        tag_layout = quote! {vm_core::EnumTagLayout::UndefinedValue{
            end: #tag_start + #tag_count - 1,
            start: #tag_start,
        }};
    } else if let Some(tag_mask) = enum_attr.tag_mask {
        let mask = tag_mask.to_usize();
        let tag_offset_i8 = tag_offset as i8;
        tag_layout = quote! {vm_core::EnumTagLayout::SmallField(
        vm_core::SmallElementLayout{mask:#mask,bit_offset:#tag_offset_i8}
        )};
    } else if let Some(tag_offset) = enum_attr.tag_offset {
        let tag_size_u8 = tag_size as u8;
        tag_layout = quote! {vm_core::EnumTagLayout::UnusedBytes{
            offset: #tag_offset,
            size: #tag_size_u8,
        }};
    } else {
        enum_layout = quote! {
            #enum_layout.builder().extend(vm_core::TypeLayout::default().set_size(#tag_size).set_align(#tag_size)).build()
        };
        let tag_size_u8 = tag_size as u8;
        tag_layout = quote! {
            vm_core::EnumTagLayout::AppendTag{
                offset: #enum_layout.size() - #tag_size,
                size: #tag_size_u8,
            }
        };
    };
    let tag_layout_fn = quote! {
      #[allow(dead_code)]
      #vis const fn #tag_layout_fn_name #impl_generics()->vm_core::EnumTagLayout #where_clause{
          #tag_layout
      }
    };
    #[allow(dead_code)]
    let layout_fn = quote! {
      #vis const fn #layout_fn_name #impl_generics()->vm_core::TypeLayout #where_clause{
          #enum_layout
      }
    };
    let where_clause = where_clause
        .map(|WhereClause { where_token, predicates }| {
            quote! {
                #where_token
                    [u8;#layout_fn_name #ty_generics_trubofish().size()]:std::marker::Sized,
                    #predicates
            }
        })
        .unwrap_or_else(|| {
            quote! {
                where
                    [u8;#layout_fn_name #ty_generics_trubofish().size()]:std::marker::Sized,
            }
        });
    let phantom = quote! {
        std::marker::PhantomData<#name #ty_generics>
    };
    Ok(quote! {
      #tag_layout_fn
      #layout_fn
      #vis mod #mod_name {
          #(#associations_alias)*
      }
      #vis trait #trait_type_name #impl_generics #where_clause {
          #(#associations_declaration)*
      }
      #[allow(dead_code)]
      impl #impl_generics #trait_type_name #ty_generics for #name #ty_generics #where_clause{
          #(#associations)*
      }
      #[allow(dead_code)]
      impl #impl_generics vm_core::TypeDeclaration for #name #ty_generics #where_clause{
        type Impl=#impl_type_name #ty_generics;
        const LAYOUT: vm_core::TypeLayout = <#impl_type_name #ty_generics>::LAYOUT;
        const TYPE: vm_core::Type = <#impl_type_name #ty_generics>::TYPE;
      }
      #[repr(C)]
      #vis struct #impl_type_name #impl_generics(#vis[u8;#layout_fn_name #ty_generics_trubofish().size()],#phantom)#where_clause;
      #[allow(dead_code)]
      impl #impl_generics #impl_type_name #ty_generics #where_clause{
        #vis const TAG_LAYOUT:vm_core::EnumTagLayout=#tag_layout_fn_name #ty_generics_trubofish();
        #vis const LAYOUT: vm_core::TypeLayout=#layout_fn_name #ty_generics_trubofish();
        #vis const TYPE:vm_core::Type=vm_core::Type::Enum(runtime::_util::CowArc::Ref(
            runtime::_util::inline_const!(
               #impl_generics[&'static vm_core::Enum]
              &vm_core::Enum{
                variants:runtime::_util::CowArc::Ref(
                  runtime::_util::inline_const!(
                     #impl_generics[&'static [vm_core::Type]]
                    &[#(#variant_declarations),*]
                 )),
                tag_layout: #tag_layout,
            })));
        #(#variant_functions)*
      }
      #(#variant_structs)*
    })
}
fn derive_struct(s: &DataStruct, structure: &Structure) -> Result<TokenStream2> {
    derive_fields(&s.fields, structure, structure.ast().ident.clone(), None)
}
fn derive_fields(fields: &Fields, structure: &Structure, name: Ident, attr: Option<StructAttribute>) -> Result<TokenStream2> {
    let ast = structure.ast();
    let vis = &ast.vis;
    let generics = &ast.generics;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let ty_generics_trubofish = ty_generics.as_turbofish();
    let impl_type_name = format_ident!("{}Impl", &name, span = name.span());
    let trait_type_name = format_ident!("{}Trait", &name, span = name.span());
    let mod_name = format_ident!("{}", util::camel_case_ident_to_snake_case_ident(&name.to_string()), span = name.span());
    let layout_fn_name = format_ident!("{}_layout", util::camel_case_ident_to_snake_case_ident(&name.to_string()), span = name.span());
    let mut struct_layout_builder = quote! {vm_core::StructLayoutBuilder::new()};
    let mut fields_derive = Vec::new();
    let mut fields_declaration = Vec::new();
    let mut associations_declaration = Vec::new();
    let mut associations_alias = Vec::new();
    let mut associations = Vec::new();
    let ast = structure.ast();
    let struct_attr = if let Some(attr) = attr {
        attr
    } else {
        ast.attrs
            .iter()
            .find(|attr| attr.path.get_ident().map(|i| i.to_string().as_str() == "make_type").unwrap_or(false))
            .map(|attr| parse2(attr.tokens.clone()))
            .unwrap_or_else(|| Ok(Default::default()))?
    };
    let mut attrs = Vec::new();
    for (_i, field) in fields.iter().enumerate() {
        let field_attr: FieldAttribute = field
            .attrs
            .iter()
            .find(|attr| attr.path.get_ident().map(|i| i.to_string().as_str() == "make_type").unwrap_or(false))
            .map(|attr| parse2(attr.tokens.clone()))
            .unwrap_or_else(|| Ok(Default::default()))?;
        attrs.push(field_attr);
    }
    let is_unsized = attrs.iter().any(|attr| attr.is_unsized);
    let is_compose = attrs.iter().any(|attr| attr.mask.is_some() || attr.bit_offset.is_some());
    for (field_index, (field, field_attr)) in fields.iter().zip(&attrs).enumerate() {
        let vis = &field.vis;
        let ident = field.ident.as_ref().cloned().unwrap_or_else(|| format_ident!("field{}", field_index.to_string()));
        let ident_name_in_snake_case = util::camel_case_ident_to_snake_case_ident(&ident.to_string());
        let getter = format_ident!("get_{}", &ident_name_in_snake_case, span = ident.span());
        let setter = format_ident!("set_{}", &ident_name_in_snake_case, span = ident.span());
        let get_ref = format_ident!("ref_{}", &ident_name_in_snake_case, span = ident.span());
        let get_ref_mut = format_ident!("ref_{}_mut", &ident_name_in_snake_case, span = ident.span());
        let layout_of = format_ident!("LAYOUT_OF_{}", util::camel_case_ident_to_upper_snake_case_ident(&ident.to_string()), span = ident.span());
        let ty = &field.ty;
        let assert_size = field_attr.assert_size.as_ref().map(|max_size| {
            quote! {const_assert!(#ty::layout().size()<=#max_size);}
        });
        let mut getter_impl;
        let mut setter_impl;
        if field_attr.bit_offset.is_some() || field_attr.mask.is_some() {
            getter_impl = quote! {ptr.cast::<u8>().add(Self::#layout_of.offset()).cast::<usize>().read()};
            setter_impl = quote! {data.to_usize()};
        } else {
            getter_impl = quote! {ptr.cast::<u8>().add(Self::#layout_of.offset()).cast::<<#ty as vm_core::TypeDeclaration>::Impl>().read()};
            setter_impl = quote! {data};
        }
        if let Some(mask) = field_attr.mask.as_ref() {
            let mask = mask.to_usize();
            getter_impl = quote! {(#mask & #getter_impl)};
        }
        if let Some(bit_offset) = field_attr.bit_offset {
            if bit_offset.get() > 0 {
                let bit_offset = bit_offset.get() as usize;
                getter_impl = quote! {(#getter_impl.to_usize() << #bit_offset)};
                setter_impl = quote! {(#setter_impl.to_usize() << #bit_offset)};
            } else {
                let bit_offset = -bit_offset.get() as usize;
                getter_impl = quote! {(#getter_impl.to_usize() >> #bit_offset)};
                setter_impl = quote! {(#setter_impl.to_usize() >> #bit_offset)};
            }
            getter_impl = quote!(#ty::from_usize(#getter_impl));
        }
        if let Some(mask) = field_attr.mask.as_ref() {
            let mask = mask.to_usize();
            setter_impl = quote!((#setter_impl & #mask)|(!mask & ptr.add(Self::#layout_of.offset()).cast::<usize>::().read()));
        }
        if field_attr.bit_offset.is_some() || field_attr.mask.is_some() {
            setter_impl = quote! {ptr.cast::<u8>().add(Self::#layout_of.offset()).cast::<usize>().write(#setter_impl)};
        } else {
            setter_impl = quote! {ptr.cast::<u8>().add(Self::#layout_of.offset()).cast::<<#ty as vm_core::TypeDeclaration>::Impl>().write(#setter_impl)};
        }
        let get_ref_impl = if !is_compose && field_attr.bit_offset.is_none() || field_attr.mask.is_none() {
            Some(quote! {
              #[inline(always)]
              #vis fn #get_ref(&self)->&<#ty as vm_core::TypeDeclaration>::Impl{
                unsafe{Option::unwrap_unchecked((self as *const Self as *const u8).add(Self::#layout_of.offset()).cast::<<#ty as vm_core::TypeDeclaration>::Impl>().as_ref())}
              }
              #[inline(always)]
              #vis fn #get_ref_mut(&mut self)->&mut <#ty as vm_core::TypeDeclaration>::Impl{
                unsafe{Option::unwrap_unchecked((self as *mut Self as *mut u8).add(Self::#layout_of.offset()).cast::<<#ty as vm_core::TypeDeclaration>::Impl>().as_mut())}
              }
            })
        } else {
            None
        };
        let setter_fn = if field_attr.is_const {
            None
        } else {
            Some(quote! {
              #[inline(always)]
              #vis fn #setter(&mut self,data:<#ty as vm_core::TypeDeclaration>::Impl){
                let ptr=self as *mut Self;
                unsafe{#setter_impl}
              }
            })
        };
        if is_compose {
            let bit_offset = field_attr.bit_offset.as_ref().map(|n| n.get()).unwrap_or(0);
            let mask = field_attr.mask.as_ref().map(|m| m.to_usize()).unwrap_or(usize::MAX);
            struct_layout_builder = quote! {
              #struct_layout_builder.extend_compose(#bit_offset,#mask,<#ty as vm_core::TypeDeclaration>::LAYOUT)
            };
        } else {
            struct_layout_builder = quote! {
              #struct_layout_builder.extend(<#ty as vm_core::TypeDeclaration>::LAYOUT)
            };
        }
        fields_derive.push(quote! {
          #vis const #layout_of:vm_core::StructLayoutBuilder=#struct_layout_builder;
          #get_ref_impl
          #assert_size
          #[inline(always)]
          #vis fn #getter(&self)-><#ty as vm_core::TypeDeclaration>::Impl{
            let ptr=self as *const Self;
            unsafe{#getter_impl}
          }
          #setter_fn
        });
        if !is_unsized && !field_attr.is_unsized && struct_attr.make_instruction {
            let ident_name_in_camel_case = util::to_camel_case(&ident.to_string());
            let get_field = format_ident!("Get{}", ident_name_in_camel_case, span = ident.span());
            let set_field = format_ident!("Set{}", ident_name_in_camel_case, span = ident.span());
            associations_declaration.push(quote! {
                type #get_field : runtime::instructions::Instruction;
                type #set_field : runtime::instructions::Instruction;
            });
            associations.push(quote! {
                type #get_field = runtime_extra::GetFieldFor<Self,#ty,{#field_index as i64}>;
                type #set_field = runtime_extra::SetFieldFor<Self,#ty,{#field_index as i64}>;
            });
            associations_alias.push(quote! {
                pub type #get_field #impl_generics=<super::#name #ty_generics as super::#trait_type_name #ty_generics>::#get_field;
                pub type #set_field #impl_generics=<super::#name #ty_generics as super::#trait_type_name #ty_generics>::#set_field;
            });
        }
        if is_compose {
            let mask = field_attr.mask.as_ref().map(|mask| mask.to_usize()).unwrap_or(usize::MAX);
            let bit_offset = field_attr.bit_offset.map(|bit_offset| bit_offset.get()).unwrap_or(0);
            let bit_offset_i8 = bit_offset as i8;
            let small_field = quote! {
                vm_core::SmallElementLayout{mask:#mask,bit_offset:#bit_offset_i8}
            };
            fields_declaration.push(quote! {
                (
                    <#ty as vm_core::TypeDeclaration>::TYPE
                    #small_field,
                )
            });
        } else {
            fields_declaration.push(quote! {
                <#ty as vm_core::TypeDeclaration>::TYPE
            });
            if struct_attr.make_instruction {
                let ident_name_in_camel_case = util::to_camel_case(&ident.to_string());
                let locate_field = format_ident!("Locate{}", ident_name_in_camel_case, span = ident.span());
                associations_declaration.push(quote! {
                    type #locate_field : runtime::instructions::Instruction;
                });
                associations.push(quote! {
                    type #locate_field = runtime_extra::LocateFieldFor<Self,#ty,{#field_index as i64}>;
                });
                associations_alias.push(quote! {
                    pub type #locate_field #impl_generics=<super::#name #ty_generics as super::#trait_type_name #ty_generics>::#locate_field;
                });
                if !field_attr.is_unsized {
                    let read_field = format_ident!("Read{}", ident_name_in_camel_case, span = ident.span());
                    let write_field = format_ident!("Write{}", ident_name_in_camel_case, span = ident.span());
                    associations_declaration.push(quote! {
                        type #read_field : runtime::instructions::Instruction;
                        type #write_field : runtime::instructions::Instruction;
                    });
                    associations.push(quote! {
                        type #read_field = runtime_extra::ReadFieldFor<Self,#ty,{#field_index as i64}>;
                        type #write_field = runtime_extra::WriteFieldFor<Self,#ty,{#field_index as i64}>;
                    });
                    associations_alias.push(quote! {
                        pub type #read_field #impl_generics=<super::#name #ty_generics as super::#trait_type_name #ty_generics>::#read_field;
                        pub type #write_field #impl_generics=<super::#name #ty_generics as super::#trait_type_name #ty_generics>::#write_field;
                    });
                }
            }
        }
    }
    let tuple = if is_compose {
        quote! {
            vm_core::Tuple::Compose(
                runtime::_util::CowArc::Ref(
                    runtime::_util::inline_const!(
                        #impl_generics[&'static[(vm_core::Type,vm_core::SmallElementLayout)]]
                        &[#(#fields_declaration),*]
                        )
                    )
            )
        }
    } else {
        quote! {
            vm_core::Tuple::Normal(
                runtime::_util::CowArc::Ref(
                    runtime::_util::inline_const!(
                        #impl_generics[&'static[vm_core::Type]]
                        &[#(#fields_declaration),*]
                        )
                    )
            )
        }
    };
    let layout_fn = quote! {
      #[allow(dead_code)]
      #vis const fn #layout_fn_name #impl_generics()->vm_core::TypeLayout #where_clause{
        #struct_layout_builder.build()
      }
    };
    let where_clause = where_clause
        .map(|WhereClause { where_token, predicates }| {
            quote! {
                #where_token
                    [u8;#layout_fn_name #ty_generics_trubofish().size()]:std::marker::Sized,
                    #predicates
            }
        })
        .unwrap_or_else(|| {
            quote! {
                where
                    [u8;#layout_fn_name #ty_generics_trubofish().size()]:std::marker::Sized,
            }
        });
    Ok(quote! {
      #layout_fn
      #vis mod #mod_name {
          #(#associations_alias)*
      }
      #vis trait #trait_type_name #generics #where_clause {
          #(#associations_declaration)*
      }
      #[allow(dead_code)]
      impl #impl_generics #trait_type_name #ty_generics for #name #ty_generics #where_clause{
          #(#associations)*
      }
      #[allow(dead_code)]
      impl #impl_generics vm_core::TypeDeclaration for #name #ty_generics #where_clause{
        type Impl=#impl_type_name #ty_generics;
        const LAYOUT: vm_core::TypeLayout = <#impl_type_name #ty_generics>::LAYOUT;
        const TYPE: vm_core::Type = <#impl_type_name #ty_generics>::TYPE;
      }
      #[allow(dead_code)]
      #[repr(C)]
      #vis struct #impl_type_name #generics (#vis[u8;#layout_fn_name #ty_generics_trubofish().size()])#where_clause ;
      impl #generics #impl_type_name #ty_generics #where_clause{
        #vis const LAYOUT:vm_core::TypeLayout=#layout_fn_name #ty_generics_trubofish();
        #vis const TYPE:vm_core::Type=vm_core::Type::Tuple(#tuple);
        #(#fields_derive)*
      }
    })
}
