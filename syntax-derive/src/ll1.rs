use super::{parse::*, production::*};
use std::collections::{hash_map::Entry, HashMap};

use failure::{format_err, Fallible};
use proc_macro2::{Ident, TokenStream as TokenStream2};

use syn::Result;
extern crate proc_macro2;

use syn::Error;
struct SyntaxLL1 {
    syntax: Syntax,
}
impl SyntaxLL1 {
    pub fn generate(&self, name: Ident) -> Fallible<TokenStream2> {
        let mut nonterminal_parser_list = Vec::new();
        let token_type = &self.syntax.token_type;
        let select_set = select_set(&self.syntax.productions);
        for (nonterminal, productions) in self.syntax.productions.iter() {
            let mut nonterminal_parser_excepts = Vec::new();
            let nonterminal_name = nonterminal.ident.to_string();
            let mut select_reverse_map = HashMap::with_hasher(ahash::RandomState::with_seed(0));
            for production in productions {
                if let Some(selects) = select_set.get(&*production) {
                    nonterminal_parser_excepts.extend(selects.iter().filter_map(|select| select.as_ref().map(|select| select.ident.to_string())));
                    for select in selects {
                        if select_reverse_map.insert(select, production).is_some() {
                            return Err(format_err!(
                                "not a LL(1) syntax.nonterminal:{},select:{}",
                                nonterminal_name,
                                select.as_ref().map(|select| select.ident.to_string()).unwrap_or_else(|| "None".to_string())
                            ));
                        }
                    }
                }
            }
            let nonterminal_parser_ident = &nonterminal.ident;
            let nonterminal_parser_name = nonterminal_parser_ident.to_string();
            let nonterminal_parser_return = &nonterminal.output;
            let nonterminal_parser_return_unwraped = nonterminal_parser_return.as_ref().map(|output| quote! {#output}).unwrap_or(quote! {()});
            let mut nonterminal_parser_match_entitys = Vec::new();
            for (select, production) in select_reverse_map.iter() {
                if let Some(select) = select {
                    let mut production_parser_step_list = Vec::new();
                    for (symbol, ident) in production.right_part.iter() {
                        let ident_unwrap = ident.clone().unwrap_or_else(|| quote! {_});
                        let production_parser_step = match symbol {
                            Symbol::Terminal(terminal) => {
                                let step_ident = &terminal.ident;
                                let step_match = match &terminal.variant_kind {
                                    VariantKind::None => quote!(Some(#step_ident)=>(),),
                                    VariantKind::Truple => quote!(Some(#step_ident(inner))=>inner,),
                                };
                                let terminal_name = terminal.ident.to_string();
                                quote! {
                                  let #ident_unwrap = match iter.next(){
                                    #step_match
                                    _=>return Err(::syntax::_format_err!("syntax error,nonterminal:{},except:{}",#nonterminal_name,#terminal_name)),
                                  };
                                }
                            }
                            Symbol::NonTerminal(step_nonterminal) => {
                                let variant_ident = &step_nonterminal.ident;
                                quote! {
                                  let #ident_unwrap = #variant_ident (iter)?;
                                }
                            }
                        };
                        production_parser_step_list.push(production_parser_step);
                    }
                    let production_parser_callback = production.callback.as_ref().map(|c| quote! {#c}).unwrap_or(quote! {Ok(())});
                    let production_parser = quote! {
                      #(#production_parser_step_list)*
                      #production_parser_callback
                    };
                    let select_ident = &select.ident;
                    let select_match = match &select.variant_kind {
                        VariantKind::None => quote!(),
                        VariantKind::Truple => quote!((inner)),
                    };
                    nonterminal_parser_match_entitys.push(quote! {
                      Some( #select_ident #select_match )=>{
                        #production_parser
                      },
                    });
                }
            }
            if let Entry::Occupied(o) = select_reverse_map.entry(&None) {
                let select_callback = o.get().callback.as_ref().map(|c| quote! {#c}).unwrap_or(quote! {Ok(())});
                nonterminal_parser_match_entitys.push(quote! {
                  _=>{
                    #select_callback
                  },
                })
            }
            let nonterminal_parser = quote! {
              fn #nonterminal_parser_ident( iter: &mut ::syntax::_Iter<#token_type> ) -> ::syntax::_Fallible<#nonterminal_parser_return_unwraped> {
                match iter.peek(){
                  #(#nonterminal_parser_match_entitys)*
                  o=>return Err(::syntax::_format_err!("syntax error when parsing {},excepts {:?}",#nonterminal_parser_name,&[#(#nonterminal_parser_excepts),*])),
                }
              }
            };
            nonterminal_parser_list.push(nonterminal_parser);
        }
        let root_output = &self.syntax.start.output;
        let root_output_unwraped = root_output.as_ref().map(|output| quote! {#output}).unwrap_or(quote! {()});
        let root_parser = &self.syntax.start.ident;
        let parser = quote! {
          fn #name(tokens:&[#token_type])->::syntax::_Fallible<#root_output_unwraped>{
            #(#nonterminal_parser_list)*
            let mut iter=tokens.iter().peekable();
            let syntax=#root_parser(&mut iter)?;
            if iter.peek().is_some(){
              return Err(::syntax::_format_err!("except EOF"));
            }
            Ok(syntax)
          }
        };
        Ok(parser)
    }
}
fn link_ll1(syntax: SyntaxDeclaration) -> Result<SyntaxLL1> {
    let syntax = parse_syntax_declaration(syntax)?;
    Ok(SyntaxLL1 { syntax })
}
pub(crate) fn do_generate_recursive_predictive_parser(syntax_declaration: SyntaxDeclaration) -> Result<TokenStream2> {
    let span = syntax_declaration.brace.span;
    let name = syntax_declaration.ident.clone();
    let syntax = link_ll1(syntax_declaration)?;
    let parser = syntax.generate(name).map_err(|e| Error::new(span, e))?;
    Ok(parser)
}
