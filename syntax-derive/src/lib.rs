pub(crate) mod ll1;
pub(crate) mod lr1;
pub(crate) mod parse;
pub(crate) mod production;
extern crate quote;

use proc_macro::TokenStream;

use syn::parse_macro_input;
extern crate proc_macro2;
#[macro_use]
extern crate synstructure;

use crate::parse::SyntaxDeclaration;
#[proc_macro]
pub fn recursive_predictive_analysis(input: TokenStream) -> TokenStream {
    let syntax_declaration = parse_macro_input!(input as SyntaxDeclaration);
    ll1::do_generate_recursive_predictive_parser(syntax_declaration)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
#[proc_macro]
pub fn lr1_analyser(input: TokenStream) -> TokenStream {
    let syntax_declaration = parse_macro_input!(input as SyntaxDeclaration);
    lr1::do_generate_parser(syntax_declaration, false)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
#[proc_macro]
pub fn lalr1_analyser(input: TokenStream) -> TokenStream {
    let syntax_declaration = parse_macro_input!(input as SyntaxDeclaration);
    lr1::do_generate_parser(syntax_declaration, true)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
