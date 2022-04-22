extern crate quote;
mod function;
mod instruction;
mod instruction_set;
mod vm_type;
use proc_macro::TokenStream;

extern crate proc_macro2;
#[macro_use]
extern crate synstructure;

#[proc_macro_derive(TypeDeclaration, attributes(make_type))]
pub fn make_type(input: TokenStream) -> TokenStream {
    vm_type::make_type(input)
}
#[proc_macro_attribute]
pub fn make_native_function(attr: TokenStream, item: TokenStream) -> TokenStream {
    function::make_native_function(attr, item)
}

#[proc_macro]
pub fn make_instruction(input: TokenStream) -> TokenStream {
    instruction::make_instruction(input)
}
#[proc_macro_derive(Instruction, attributes(instruction))]
pub fn make_instruction_by_attr(input: TokenStream) -> TokenStream {
    instruction::make_instruction_by_attr(input)
}

#[proc_macro]
pub fn make_instruction_set(input: TokenStream) -> TokenStream {
    instruction_set::make_instruction_set(input)
}
