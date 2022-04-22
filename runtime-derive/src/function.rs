use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};

use syn::{parse::Parse, parse_macro_input, spanned::Spanned, Error, ItemFn, Result};

struct NativeFunctionAttr {
    name: Ident,
}

impl Parse for NativeFunctionAttr {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Ok(Self { name: input.parse()? })
    }
}

pub(crate) fn make_native_function(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let function = parse_macro_input!(item as ItemFn);
    let attr = parse_macro_input!(attr as NativeFunctionAttr);
    do_make_native_function(function, attr).map(TokenStream::from).unwrap_or_else(|e| TokenStream::from(e.into_compile_error()))
}
fn do_make_native_function(function: ItemFn, attr: NativeFunctionAttr) -> Result<TokenStream2> {
    let sig = &function.sig;
    let c_name = &sig.ident;
    let c_name_str = c_name.to_string();
    let name = &attr.name;
    let mut input_metadata = Vec::new();
    let mut input_type = Vec::new();
    let mut input = Vec::new();
    let output_metadata;
    let output_type;
    let output;
    let vis = &function.vis;
    for i in sig.inputs.iter() {
        match i {
            syn::FnArg::Receiver(i) => return Err(Error::new(i.span(), "except `arg_name:ArgType`")),
            syn::FnArg::Typed(t) => match &*t.pat {
                syn::Pat::Ident(arg_name) => {
                    let arg_type = &t.ty;
                    input_metadata.push(quote! {#arg_name:#arg_type});
                    input_type.push(quote! {<#arg_type as jvm_core::TypeDeclaration>::TYPE});
                    input.push(quote! {%#arg_name});
                }
                _ => return Err(Error::new(t.pat.span(), "except ident")),
            },
        };
    }
    match &sig.output {
        syn::ReturnType::Default => {
            output_type = quote! {None};
            output_metadata = None;
            output = None;
        }
        syn::ReturnType::Type(_, return_type) => {
            output_type = quote! {Some(<#return_type as jvm_core::TypeDeclaration>::TYPE)};
            output_metadata = Some(quote! {o:#return_type});
            output = Some(quote! {%o =});
        }
    }
    Ok(quote! {
        #[no_mangle]
        #function
        #[derive(Instruction)]
        #[instruction(
            #name->fn<>(#(#input_metadata),*)->(#output_metadata){entry: {
                #output runtime::instructions::bootstrap::NativeCall<#name::TYPE,#c_name,#c_name_str>(#(#input),*);
            }},#c_name={kind=RustFn}
        )]
        #vis enum #name{}
        impl #name{
            pub const TYPE: jvm_core::Type = jvm_core::Type::Function(
                runtime::_util::CowArc::Ref(
                    runtime::_util::inline_const!([&'static jvm_core::FunctionType]
                        &jvm_core::FunctionType{
                            dispatch:runtime::_util::CowSlice::new(),
                            va_arg:None,
                            return_type: #output_type,
                            args:runtime::_util::CowSlice::Ref(
                                runtime::_util::inline_const!(
                                    [&'static[jvm_core::Type]]
                                    &[#(#input_type),*]
                                )
                            ),
                        }
                    )
                )
            );
        }
    })
}
