extern crate quote;
use lexical::to_ident;
use std::{iter::Peekable, slice::Iter};

use proc_macro::TokenStream;
use quote::ToTokens;
use synstructure::VariantInfo;
extern crate proc_macro2;
#[macro_use]
extern crate synstructure;
extern crate lazy_static;

use proc_macro2::Span;
use syn::{bracketed, parse::Parse, parse_macro_input, punctuated::Punctuated, spanned::Spanned, token::Bracket, ItemEnum, LitStr};
#[derive(Debug)]
struct Error(TokenStream);

impl Error {
    fn new(span: Span, message: &str) -> Error {
        Error(
            quote_spanned! { span =>
                compile_error!(#message);
            }
            .into(),
        )
    }

    fn into_tokens(self) -> TokenStream {
        self.0
    }
}

impl From<syn::Error> for Error {
    fn from(e: syn::Error) -> Error {
        Error(e.to_compile_error().into())
    }
}
struct VariantAttr {
    match_kind: MatchKind,
    ignore: bool,
}
enum MatchKind {
    String(String),
    Word(String),
    Regex(String),
    Fn(String),
    Writespace,
    Newline,
    Indentation(bool),
}
fn parse_metadata(metadata_name: &str, attrs: &[syn::Attribute]) -> Result<Option<syn::MetaList>, Error> {
    let mut metadata = None;
    for attr in attrs {
        if let Ok(meta) = attr.parse_meta() {
            if meta.path().is_ident(metadata_name) {
                if metadata.is_some() {
                    Err(Error::new(meta.span(), "Cannot have two display attributes"))?;
                } else if let syn::Meta::List(list) = meta {
                    metadata = Some(list);
                } else {
                    Err(Error::new(meta.span(), "fail attribute must take a list in parentheses"))?;
                }
            }
        }
    }
    Ok(metadata)
}
fn conflict_attr_error<T>(span: Span) -> Result<T, Error> {
    Err(Error::new(span, "only one of 'strng' 'regex' 'from_str'(default) can be set"))
}
fn parse_match_kind(s: &synstructure::Structure) -> Result<Vec<VariantAttr>, Error> {
    let mut variants = Vec::with_capacity(s.variants().len());
    for v in s.variants() {
        let metadata =
            parse_metadata("lexical", v.ast().attrs)?.ok_or_else(|| Error::new(v.ast().ident.span(), "All variants must have display attribute."))?;
        if metadata.nested.is_empty() {
            Err(Error::new(v.ast().ident.span(), "Expected at least one argument to lexical attribute"))?;
        }
        let mut match_kind = None;
        let mut ignore = false;
        for nexted in metadata.nested {
            match nexted {
                syn::NestedMeta::Meta(syn::Meta::NameValue(syn::MetaNameValue { path, lit, .. })) if path.is_ident("string") => {
                    if match_kind.is_none() {
                        match_kind = Some(MatchKind::String(match lit {
                            syn::Lit::Str(s) => s.value(),
                            _ => {
                                return Err(Error::new(path.span(), "the value of key 'string' must be a string"));
                            }
                        }))
                    } else {
                        return conflict_attr_error(lit.span());
                    }
                }
                syn::NestedMeta::Meta(syn::Meta::NameValue(syn::MetaNameValue { path, lit, .. })) if path.is_ident("word") => {
                    if match_kind.is_none() {
                        match_kind = Some(MatchKind::Word(match lit {
                            syn::Lit::Str(s) => s.value(),
                            _ => {
                                return Err(Error::new(lit.span(), "the value of key 'word' must be a string"));
                            }
                        }))
                    } else {
                        return conflict_attr_error(lit.span());
                    }
                }
                syn::NestedMeta::Meta(syn::Meta::NameValue(syn::MetaNameValue { path, lit, .. })) if path.is_ident("regex") => {
                    if match_kind.is_none() {
                        match_kind = Some(MatchKind::Regex(match lit {
                            syn::Lit::Str(s) => s.value(),
                            _ => {
                                return Err(Error::new(lit.span(), "the value of key 'regex' must be a string"));
                            }
                        }))
                    } else {
                        return conflict_attr_error(lit.span());
                    }
                }
                syn::NestedMeta::Meta(syn::Meta::NameValue(syn::MetaNameValue { path, lit, .. })) if path.is_ident("fn") => {
                    if match_kind.is_none() {
                        match_kind = Some(MatchKind::Fn(match lit {
                            syn::Lit::Str(s) => s.value(),
                            _ => {
                                return Err(Error::new(lit.span(), "the value of key 'fn' must be a string"));
                            }
                        }))
                    } else {
                        return conflict_attr_error(lit.span());
                    }
                }
                syn::NestedMeta::Meta(syn::Meta::NameValue(syn::MetaNameValue { path, lit, .. })) if path.is_ident("indentation") => {
                    if match_kind.is_none() {
                        match_kind = Some(MatchKind::Indentation(match lit {
                            syn::Lit::Str(s) => match &*s.value() {
                                "decrease" => false,
                                "increase" => true,
                                _ => {
                                    return Err(Error::new(s.span(), "the value of key 'indentation' must be 'decrease' of 'increase' ."));
                                }
                            },
                            _ => {
                                return Err(Error::new(path.span(), "the value of key 'regex' must be a string"));
                            }
                        }))
                    } else {
                        return conflict_attr_error(lit.span());
                    }
                }
                syn::NestedMeta::Meta(syn::Meta::Path(path)) if path.is_ident("whitespace") => {
                    if match_kind.is_none() {
                        match_kind = Some(MatchKind::Writespace);
                    } else {
                        return conflict_attr_error(path.span());
                    }
                }
                syn::NestedMeta::Meta(syn::Meta::Path(path)) if path.is_ident("newline") => {
                    if match_kind.is_none() {
                        match_kind = Some(MatchKind::Newline);
                    } else {
                        return conflict_attr_error(path.span());
                    }
                }
                syn::NestedMeta::Meta(syn::Meta::Path(path)) if path.is_ident("ignore") => {
                    ignore = true;
                }
                o => {
                    return Err(Error::new(o.span(), "unsupported metadata"));
                }
            }
        }
        if let Some(match_kind) = match_kind {
            variants.push(VariantAttr { ignore, match_kind });
        } else {
            return Err(Error::new(
                v.ast().ident.span(),
                "every variant should has a attribute 'string' 'regex' 'newline' 'fn' 'whitespace' or 'indentation' ",
            ));
        }
    }
    Ok(variants)
}
fn generate_match_string(iter: &mut Peekable<Iter<(&String, &VariantAttr, &VariantInfo)>>) -> proc_macro2::TokenStream {
    if let Some(current) = iter.next() {
        let current_str = &**current.0;
        let current_ident = current.2.ast().ident.to_token_stream();
        let mut sub_cases = Vec::new();
        loop {
            if let Some(next) = iter.peek() {
                let next_str = next.0;
                if next_str.starts_with(current_str) {
                    sub_cases.push(generate_match_string(iter));
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        let ignore = current.1.ignore;
        quote! {
          if chars.as_str().starts_with(#current_str){
            #(#sub_cases;)*
            if !#ignore{
              tokens.push(Self::#current_ident);
            }
            chars=chars.as_str().split_at(#current_str.len()).1.chars();
            continue;
          }
        }
    } else {
        quote! {}
    }
}
fn do_lexical_derive(s: synstructure::Structure) -> Result<proc_macro2::TokenStream, Error> {
    let match_kind_list = parse_match_kind(&s)?;
    let variant_info_list = s.variants();
    let variant_list: Vec<_> = match_kind_list.iter().zip(variant_info_list).collect();
    let newline_ident = variant_list.iter().find_map(|m| match (m.0.ignore, &m.0.match_kind) {
        (false, MatchKind::Newline) => Some(m.1.ast().ident),
        _ => None,
    });
    let whitespace_ident = variant_list.iter().find_map(|m| match (m.0.ignore, &m.0.match_kind) {
        (false, MatchKind::Writespace) => Some(m.1.ast().ident),
        _ => None,
    });
    let mut parse_indentatoin = false;
    let indentation_increase_ident = variant_list.iter().find_map(|m| match m.0.match_kind {
        MatchKind::Indentation(true) => {
            parse_indentatoin = true;
            Some((m.1.ast().ident, m.0.ignore))
        }
        _ => None,
    });
    let indentation_decrease_ident = variant_list.iter().find_map(|m| match m.0.match_kind {
        MatchKind::Indentation(false) => {
            parse_indentatoin = true;
            Some((m.1.ast().ident, m.0.ignore))
        }
        _ => None,
    });
    let emit_newline = if newline_ident.is_some() {
        quote! {
          tokens.push(Self::#newline_ident);
        }
    } else {
        quote! {}
    };
    let whitespace_add_char = if whitespace_ident.is_some() {
        quote! {
          whitespace_string.push(b);
        }
    } else {
        quote! {}
    };
    let emit_whitespace = if let Some(whitespace_ident) = whitespace_ident {
        quote! {
          tokens.push(Self::#whitespace_ident(whitespace_string));
          whitespace_string=String::new();
        }
    } else {
        quote! {}
    };
    let emit_parse_indentation = if parse_indentatoin {
        let emit_indentation_increase = if let Some((indentation_increase_ident, ignore)) = indentation_increase_ident {
            quote! {
              if !#ignore{
                tokens.push(Self::#indentation_increase_ident(indentation_string.len(),indentation_string));
              }
            }
        } else {
            quote! {}
        };
        let emit_indentation_decrease = if let Some((indentation_decrease_ident, ignore)) = indentation_decrease_ident {
            quote! {
              if !#ignore{
                tokens.push(Self::#indentation_decrease_ident(indentation_string.len(),indentation_string));
              }
            }
        } else {
            quote! {}
        };
        quote! {
            let prefix_whitespace_count=indentation_string.chars().count();
            if prefix_whitespace_count>*indentation_stack.last().unwrap(){
              indentation_stack.push(prefix_whitespace_count);
              #emit_indentation_increase
              indentation_string=String::new();
              has_emit_indentation=true;
            }
            let prefix_whitespace_count=indentation_string.chars().count();
            if !has_emit_indentation && prefix_whitespace_count<*indentation_stack.last().unwrap(){
              while prefix_whitespace_count<*indentation_stack.last().unwrap(){
                indentation_stack.pop().unwrap();
                #emit_indentation_decrease
                indentation_string=String::new();
              }
            }
        }
    } else {
        quote! {}
    };
    let mut string_variants: Vec<_> = variant_list
        .iter()
        .filter_map(|v| match &v.0.match_kind {
            MatchKind::String(s) => Some((s, v.0, v.1)),
            _ => None,
        })
        .collect();
    string_variants.sort_by(|a, b| a.0.cmp(b.0));
    let mut string_variants_iter = string_variants.iter().peekable();
    let mut match_string = Vec::new();
    while string_variants_iter.peek().is_some() {
        match_string.push(generate_match_string(&mut string_variants_iter));
    }
    let word_variants: Vec<_> = variant_list
        .iter()
        .filter(|v| match v.0.match_kind {
            MatchKind::Word(_) => true,
            _ => false,
        })
        .collect();
    let regex_variants: Vec<_> = variant_list
        .iter()
        .filter(|v| match &v.0.match_kind {
            MatchKind::Regex(_) => true,
            _ => false,
        })
        .collect();
    let fn_variants: Vec<_> = variant_list
        .iter()
        .filter(|v| match &v.0.match_kind {
            MatchKind::Fn(_) => true,
            _ => false,
        })
        .collect();
    let regex_variant_count = regex_variants.len();
    let regex_str = "^".to_string()
        + &regex_variants
            .iter()
            .map(|v| match &v.0.match_kind {
                MatchKind::Regex(regex) => {
                    format!("(?P<{}>{})", v.1.ast().ident, regex)
                }
                _ => unreachable!(),
            })
            .collect::<Vec<_>>()
            .join("|");
    let regex_match_capturing = regex_variants
        .iter()
        .map(|v| {
            let variant = v.1.ast().ident.to_token_stream();
            let name = v.1.ast().ident.to_string();
            let constructor = match v.1.ast().fields {
                syn::Fields::Unit => quote! {Self::#variant},
                syn::Fields::Unnamed(_unnamed) => {
                    quote! {Self::#variant(token_str.parse()?)}
                }
                syn::Fields::Named(named) => {
                    if let Some(first_field) = named.named.first() {
                        let field_name = first_field.ident.as_ref().unwrap().to_token_stream();
                        quote! {Self::#variant{#field_name , FromStr::from_str(token_str)?}}
                    } else {
                        quote! {Self::#variant{}}
                    }
                }
            };
            let ignore = v.0.ignore;
            quote! {
              if let Some(matches)=cap.name(&#name){
                let token_str=matches.as_str();
                if !#ignore{
                    tokens.push(#constructor);
                }
                chars=chars.as_str().split_at(token_str.len()).1.chars();
                continue;
              }

            }
        })
        .collect::<Vec<_>>();
    let fn_match_capturing = fn_variants
        .iter()
        .map(|v| {
            let variant = v.1.ast().ident.to_token_stream();
            let function = match &v.0.match_kind {
                MatchKind::Fn(f) => format_ident!("{}", f),
                _ => unreachable!(),
            };
            let constructor = match v.1.ast().fields {
                syn::Fields::Unit => quote! {Self::#variant},
                syn::Fields::Unnamed(_unnamed) => {
                    quote! {Self::#variant(matches)}
                }
                syn::Fields::Named(named) => {
                    if let Some(first_field) = named.named.first() {
                        let field_name = first_field.ident.as_ref().unwrap().to_token_stream();
                        quote! {Self::#variant{#field_name , matches?}}
                    } else {
                        quote! {Self::#variant{}}
                    }
                }
            };
            quote! {
              {
                  let mut iter=chars.clone();
                  if let Some(matches)=#function(&mut iter){
                    tokens.push(#constructor);
                    chars=iter;
                    continue;
                  }
              }
            }
        })
        .collect::<Vec<_>>();
    let fn_match = if !fn_variants.is_empty() {
        quote! {
          #(#fn_match_capturing;)*
        }
    } else {
        quote! {}
    };
    let regex_match = if !regex_variants.is_empty() {
        quote! {
          if let Some(cap)=REGEX.captures(chars.as_str()){
            #(#regex_match_capturing;)*
          }
        }
    } else {
        quote! {}
    };
    let word_match = if regex_variant_count != 0 {
        let strings = word_variants
            .iter()
            .map(|v| match &v.0.match_kind {
                MatchKind::Word(word) => word,
                _ => unreachable!(),
            })
            .collect::<Vec<_>>();
        let idents = word_variants.iter().map(|v| v.1.ast().ident.to_token_stream()).collect::<Vec<_>>();
        let mut cases = Vec::new();
        for word_variant in word_variants.iter() {
            let string = match &word_variant.0.match_kind {
                MatchKind::Word(word) => word,
                _ => unreachable!(),
            };
            let ident = word_variant.1.ast().ident.clone();
            let ignore = word_variant.0.ignore;
            cases.push(quote! {#string=>{
              if !#ignore{
                  tokens.push(Self::#ident);
              }
              chars=chars.as_str().split_at(token_str.len()).1.chars();
              continue;
            }});
        }
        quote! {
          if let Some(cap)=WORD_REGEX.captures(chars.as_str()){
            if let Some(matches)=cap.name("word"){
              let token_str=matches.as_str();
              match token_str{
                #(#cases),*
                _=>{}
              }
            }
          }
        }
    } else {
        quote! {}
    };
    let r = s.unbound_impl(
        quote!(::lexical::Lexical),
        quote! {
            #[allow(dead_code)]
            fn parse(source:&str)->::lexical::_Fallible<Vec<Self>>{
              ::lexical::_lazy_static::lazy_static!{
                static ref WORD_REGEX:
                  ::lexical::_regex::Regex=::lexical::_regex::RegexBuilder::new(
                    r"^(?P<word>\p{L}+)"
                  ).unicode(true).build().unwrap();
              };
              ::lexical::_lazy_static::lazy_static!{
                static ref REGEX:
                  ::lexical::_regex::Regex=::lexical::_regex::RegexBuilder::new(
                    #regex_str
                  ).unicode(true).build().unwrap();
              };
              let mut indentation_stack=Vec::<usize>::new();
              indentation_stack.push(0);
                let mut index=0;
                let mut tokens=Vec::new();
                let mut whitespace_string=String::new();
                let mut indentation_string = String::new();
                let mut last_char_is_whitespace=false;
                let mut last_char_is_newline=false;
                let mut chars=source.chars();
                while let Some(b)=chars.clone().next(){
                  if b.is_ascii_whitespace(){
                    if b=='\n'{
                      #emit_newline
                    }
                    if last_char_is_whitespace{
                      #whitespace_add_char
                      {
                        if indentation_string.len()!=0 || last_char_is_newline{
                          indentation_string.push(b);
                        }
                      }
                    }
                    last_char_is_whitespace=true;
                    last_char_is_newline=b=='\n';
                    chars.next().unwrap();
                  }else{
                    let mut has_emit_indentation=false;
                    #emit_parse_indentation
                    if last_char_is_whitespace{
                      #emit_whitespace
                    }
                    last_char_is_whitespace=false;
                    // match string
                    #(#match_string);*
                    // match word
                    #word_match
                    // match regex
                    #regex_match
                    // match by function
                    #fn_match
                    Err(::lexical::_format_err!("unexpected lexical :{}",chars.clone().take(32).collect::<std::string::String>()))?;
                  }
                  index+=1;
                }
                Ok(tokens)
            }
        },
    );
    Ok(r)
}
fn lexical_derive(i: synstructure::Structure) -> TokenStream {
    do_lexical_derive(i).map(|f| f.into()).unwrap_or_else(|e| e.into_tokens())
}
decl_derive!([Lexical,attributes(lexical)]=>lexical_derive);
struct Strings {
    _bracket: Bracket,
    tokens: Punctuated<LitStr, Token!(,)>,
}
impl Parse for Strings {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self { _bracket: bracketed!(content in input), tokens: content.parse_terminated(<LitStr as Parse>::parse)? })
    }
}
impl Parse for Attr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self { keywords: input.parse()?, _keyword_comma: input.parse()?, symbols: input.parse()? })
    }
}
struct Attr {
    keywords: Strings,
    _keyword_comma: Token!(,),
    symbols: Strings,
}
#[proc_macro_attribute]
pub fn lexical(attr: TokenStream, input: TokenStream) -> TokenStream {
    let parsed_enum = parse_macro_input!(input as ItemEnum);
    let ItemEnum {
        // attrs,
        vis,
        ident,
        generics,
        attrs,
        variants,
        ..
    } = parsed_enum;
    let parsed_attr = parse_macro_input!(attr as Attr);
    let Attr { keywords, symbols, .. } = parsed_attr;
    let keywords: Vec<_> = keywords
        .tokens
        .iter()
        .map(|s| {
            let value = s.value();
            let ident = format_ident!("{}", to_ident(&*value));
            quote! {
              #[lexical(word = #s)]
              #ident,
            }
        })
        .collect();
    let symbols: Vec<_> = symbols
        .tokens
        .iter()
        .map(|s| {
            let value = s.value();
            let ident = format_ident!("{}", to_ident(&*value));
            quote! {
              #[lexical(string = #s)]
              #ident,
            }
        })
        .collect();
    quote! {
      #(#attrs)*
      #[derive(Lexical)]
      #vis enum #ident #generics {
        #(#keywords)*
        #(#symbols)*
        #variants
      }
    }
    .into()
}
#[proc_macro]
pub fn token(input: TokenStream) -> TokenStream {
    let token = input.to_string();
    let ident = format_ident!("{}", to_ident(&*token));
    quote! {
      #ident
    }
    .into()
}
