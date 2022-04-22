extern crate proc_macro2;
extern crate quote;
#[macro_use]
extern crate synstructure;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{bracketed, parse::Parse, parse_macro_input, token::Bracket, Expr, Generics, Type};
fn derive_as_any(s: synstructure::Structure) -> TokenStream2 {
    s.unbound_impl(
      quote!(::util::AsAny),
        quote! {
              #[allow(unreachable_code)]
              fn as_any(&self) -> &dyn std::any::Any{
                  self
              }
              #[allow(unreachable_code)]
              fn as_any_arc(self:std::sync::Arc<Self>)->std::sync::Arc<dyn std::any::Any + std::marker::Sync + std::marker::Send>{
                  self
              }
        }
      )
}
decl_derive!([AsAny]=>derive_as_any);

struct InlineConst {
    generics: Generics,
    _wrap: Bracket,
    ty: Type,
    expr: Expr,
}

impl Parse for InlineConst {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            generics: input.parse()?,
            _wrap: bracketed!(content in input),
            ty: content.parse()?,
            expr: input.parse()?,
        })
    }
}
#[proc_macro]
pub fn inline_const(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as InlineConst);
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let phantom_tuple = input
        .generics
        .params
        .iter()
        .filter_map(|param| match param {
            syn::GenericParam::Type(t) => Some(t.ident.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let ty = input.ty;
    let expr = input.expr;
    let turbofish = ty_generics.as_turbofish();
    quote! {{
        struct Const #impl_generics((#(std::marker::PhantomData<#phantom_tuple>),*)) #where_clause;
        impl #impl_generics Const #ty_generics #where_clause {
            const C: #ty = #expr;
        }
        Const #turbofish ::C
    }}
    .into()
}
