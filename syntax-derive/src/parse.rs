use lexical::to_ident;
use proc_macro2::{Ident, TokenTree};


use syn::{
    bracketed,
    ext::IdentExt,
    parenthesized,
    parse::{Parse},
    punctuated::Punctuated,
    token::{Brace, Bracket, Paren},
    Expr, Type,
};
extern crate proc_macro2;
use proc_macro2::TokenStream as TokenStream2;
use syn::Error;
pub(crate) struct SyntaxDeclaration {
    pub(crate) ident: Ident,
    _after_name: Token!(:),
    pub(crate) lexical: Type,
    _after_lexical: Token!(->),
    pub(crate) output_type: Type,
    pub(crate) brace: Brace,
    pub(crate) nonterminals: Punctuated<NonTerminalDeclaration, Token!(,)>,
}
impl Parse for SyntaxDeclaration {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            ident: input.parse()?,
            _after_name: input.parse()?,
            lexical: input.parse()?,
            _after_lexical: input.parse()?,
            output_type: input.parse()?,
            brace: braced!(content in input),
            nonterminals: content.parse_terminated(NonTerminalDeclaration::parse)?,
        })
    }
}
pub(crate) struct NonTerminalDeclaration {
    pub(crate) ident: Ident,
    pub(crate) output: OutputDeclaration,
    pub(crate) _derive: Token!(->),
    pub(crate) _brace: Brace,
    pub(crate) productions: Punctuated<ProductionDeclaration, Token!(|)>,
}
impl Parse for NonTerminalDeclaration {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            ident: input.parse()?,
            output: input.parse()?,
            _derive: input.parse()?,
            _brace: braced!(content in input),
            productions: content.parse_terminated(ProductionDeclaration::parse)?,
        })
    }
}
pub(crate) struct ProductionDeclaration {
    pub(crate) _bracket: Bracket,
    pub(crate) symbols: Punctuated<SymbolDeclaration, Token!(,)>,
    pub(crate) callback: CallbackDeclaration,
}
impl Parse for ProductionDeclaration {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self { _bracket: bracketed!(content in input), symbols: content.parse_terminated(SymbolDeclaration::parse)?, callback: input.parse()? })
    }
}
pub(crate) enum CallbackDeclaration {
    None,
    Some { _prefix: Token!(=>), callback: Box<Expr>, _semicolon: Token!(;) },
}
impl Parse for CallbackDeclaration {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Token!(=>)) {
            Ok(Self::Some { _prefix: input.parse()?, callback: Box::new(input.parse()?), _semicolon: input.parse()? })
        } else {
            Ok(Self::None)
        }
    }
}
pub(crate) struct SymbolDeclaration {
    pub(crate) ident: Ident,
    pub(crate) value: MatchValueDeclaration,
}
impl Parse for SymbolDeclaration {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.call(Ident::parse_any)?;
        let lookahead = input.lookahead1();
        let symbol = if lookahead.peek(Token!(!)) {
            if &*ident.to_string() != "t" {
                return Err(Error::new(ident.span(), "except 't'"));
            }
            let _: Token!(!) = input.parse()?;
            let content;
            let _ = parenthesized!(content in input);
            let r = format_ident!("{}", to_ident(&*content.to_string()));
            while !content.is_empty() {
                let _: TokenTree = content.parse()?;
            }
            r
        } else {
            ident
        };
        Ok(Self { ident: symbol, value: input.parse()? })
    }
}
pub(crate) enum MatchValueDeclaration {
    Some { _bracket: Paren, token: TokenStream2 },
    None,
}
impl Parse for MatchValueDeclaration {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Paren) {
            let content;
            let ret = Self::Some { _bracket: parenthesized!(content in input), token: content.parse()? };
            if !content.is_empty() {
                return Err(Error::new(content.span(), r#"except ")""#));
            }
            Ok(ret)
        } else {
            Ok(Self::None)
        }
    }
}
pub(crate) enum OutputDeclaration {
    Some { _prefix: Token!(=>), output: Box<Type> },
    None,
}
impl Parse for OutputDeclaration {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Token!(=>)) {
            Ok(Self::Some { _prefix: input.parse()?, output: Box::new(input.parse()?) })
        } else {
            Ok(Self::None)
        }
    }
}
