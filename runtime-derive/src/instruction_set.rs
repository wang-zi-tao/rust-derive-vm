use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};

use syn::{bracketed, parse::Parse, parse_macro_input, punctuated::Punctuated, token::Bracket, Result};

use crate::instruction::InstructionDeclaration;

pub fn make_instruction_set(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as InstructionSetDeclaration);
    || -> Result<TokenStream2> {
        let mut instructions = Vec::new();
        let mut opcodes = Vec::new();
        let mut next_opcode = quote!(0);
        let ident = &input.ident;
        for i in &input.list {
            instructions.push(i.build_instruction(&Default::default())?);
            let instruction_ident = i.get_name();
            opcodes.push(quote! {
                impl runtime::instructions::InstructionOf<#ident> for #instruction_ident{
                    const OPCODE: usize = #next_opcode;
                }
            });
            next_opcode = quote! {
                <#instruction_ident as runtime::instructions::InstructionOf<#ident>>::OPCODE
                    + <#instruction_ident as runtime::instructions::Instruction>::STATE_COUNT
            };
        }
        let mut elements = Vec::new();
        for i in &input.list {
            let type_ident = i.get_name();
            elements.push(quote! {
              (<#type_ident as runtime::instructions::InstructionOf<#ident>>::OPCODE,<#type_ident as runtime::instructions::Instruction>::INSTRUCTION_TYPE)
            });
        }
        Ok(quote! {
            #(#instructions)*
            #(#opcodes)*
            pub struct #ident;
            impl runtime::instructions::InstructionSet for #ident{
                const INSTRUCTIONS : runtime::_util::CowSlice<'static, (usize,runtime::instructions::InstructionType)> =
                    runtime::_util::CowSlice::Ref(
                        runtime::_util::inline_const!(
                              [&'static[(usize,runtime::instructions::InstructionType)]]
                              &[ #(#elements),* ])
                    );
                const INSTRUCTION_COUNT: usize = #next_opcode;
            }
        })
    }()
    .map(TokenStream::from)
    .unwrap_or_else(|e| TokenStream::from(e.into_compile_error()))
}
struct InstructionSetDeclaration {
    ident: Ident,
    _eq: Token!(=),
    _bracket: Bracket,
    list: Punctuated<InstructionDeclaration, Token!(,)>,
}
impl Parse for InstructionSetDeclaration {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let content;
        Ok(Self {
            ident: input.parse()?,
            _eq: input.parse()?,
            _bracket: bracketed!(content in input),
            list: content.parse_terminated(InstructionDeclaration::parse)?,
        })
    }
}
