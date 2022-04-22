//! ```
//! make_instruction![
//!       NOP->bootstrap::NOP,
//!       ADD->fn<const c:i64>(i1:i64,i2:i64)->{i2:i64}{
//!         entry:{
//!           %i2=bootstrap::Add<c>(%i1,%i2);
//!         }
//!       },
//! ]
//! ```
//!
//! ```
//!  enum ADD {}
//!  impl ItemOf<InstructionSetDemo> for ADD {
//!      const OPCODE:usize =1 ;
//!  }
//!  impl ADD {
//!      pub fn emit<S>(
//!          b: &mut Builder,
//!          int_kind: IntKind,
//!          arg0: Register,
//!          arg1: Register,
//!      ) -> Fallible<()>
//!      where
//!          Self: ItemOf<S>,
//!      {
//!          unsafe {
//!              b.put_opcode(<Self as ItemOf<S>>::get_opcode())?;
//!              b.align(2);
//!              b.put_int_type(int_kind)?;
//!              b.put_register(arg0)?;
//!              b.put_register(arg1)?;
//!          }
//!          Ok(())
//!      }
//!  }
//!  impl Instruction for ADD {
//!      const instruction_type:InstructionType= ... ;
//!  }
//! ```

use std::{
    collections::{HashMap, HashSet},
    iter::FromIterator,
};

use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};

use quote::ToTokens;
use syn::{
    bracketed,
    ext::IdentExt,
    parenthesized,
    parse::Parse,
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{self, Brace, Bracket, Paren},
    DeriveInput, Error, ImplGenerics, Lit, Path, Result, Type,
};
use synstructure::Structure;

#[derive(Clone)]
pub enum GenericKind {
    Type,
    Int,
    Float,
    Str,
    Bytes,
    Bool,
    Byte,
    RustFn,
}

impl Parse for GenericKind {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let kind: Ident = input.parse()?;
        Ok(match kind.to_string().as_str() {
            "Str" => Self::Str,
            "Type" => Self::Type,
            "Int" => Self::Int,
            "Float" => Self::Float,
            "Bytes" => Self::Bytes,
            "Bool" => Self::Bool,
            "Byte" => Self::Byte,
            "RustFn" => Self::RustFn,
            _ => return Err(Error::new(kind.span(), "except one of `Type` `Str` `Int` `Float` `Bytes` `Bool` `Byte` `RustFn` ")),
        })
    }
}
pub struct InstructionAttr {
    instruction: InstructionDeclaration,
    generic_params_kind: HashMap<String, GenericKind>,
}

impl Parse for InstructionAttr {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let instruction = input.parse()?;
        let mut generic_params_kind = HashMap::new();
        while input.peek(Token!(,)) {
            let _: Token!(,) = input.parse()?;
            let generic: Ident = input.parse()?;
            let _: Token!(=) = input.parse()?;
            let attr: GenericKindAttr = input.parse()?;
            if let Some(kind) = attr.kind {
                generic_params_kind.insert(generic.to_string(), kind);
            }
        }
        let _: Option<Token!(,)> = input.parse()?;
        Ok(Self { instruction, generic_params_kind })
    }
}
pub struct GenericKindAttr {
    kind: Option<GenericKind>,
}

impl Parse for GenericKindAttr {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let content;
        let _wrap = braced!(content in input);
        let mut kind = None;
        while !content.is_empty() {
            let key: Ident = content.call(Ident::parse_any)?;
            match key.to_string().as_str() {
                "kind" => {
                    let _eq: Token!(=) = content.parse()?;
                    let value: GenericKind = content.parse()?;
                    if kind.replace(value).is_some() {
                        return Err(Error::new(key.span(), "Duplicate metadata"));
                    }
                }
                o => return Err(Error::new(key.span(), format!("unknown metadata `{}`", o))),
            }
            if !content.is_empty() {
                let _split: Token!(,) = content.parse()?;
            }
        }
        Ok(Self { kind })
    }
}

pub fn make_instruction_by_attr(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let structure = Structure::new(&input);
    || -> Result<TokenStream2> {
        let ast = structure.ast();
        let instruction_attr: InstructionAttr = ast
            .attrs
            .iter()
            .find(|attr| attr.path.get_ident().map(|ident| ident.to_string().as_str() == "instruction").unwrap_or(false))
            .ok_or_else(|| Error::new(ast.ident.span(), "attribute `instruction` not found"))?
            .parse_args()?;
        let (impl_generics, _ty_generics, _where_clause) = ast.generics.split_for_impl();
        let derive = instruction_attr.instruction.build_instruction(&BuildInstructionConfig {
            structure: Some(structure),
            impl_generics: Some(impl_generics),
            generic_params: instruction_attr.generic_params_kind,
            ..Default::default()
        })?;
        // panic!("{}", derive.to_string());
        Ok(quote! {
            #derive
        })
    }()
    .map(TokenStream::from)
    .unwrap_or_else(|e| TokenStream::from(e.into_compile_error()))
}
pub fn make_instruction(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as InstructionDeclaration);
    || -> Result<TokenStream2> { input.build_instruction(&Default::default()) }()
        .map(TokenStream::from)
        .unwrap_or_else(|e| TokenStream::from(e.into_compile_error()))
}
pub struct InstructionDeclaration {
    pub(crate) name: Path,
    _derive: Token!(->),
    pub(crate) instruction_kind: InstructionKindDeclaration,
}
pub(crate) enum InstructionKindDeclaration {
    DirectMap { instruction: Path },
    Compresion { _wrap: Bracket, list: Punctuated<InstructionDeclaration, Token!(,)> },
    Complex { metadata: MetadataDeclaration, function_boby: FunctionBobyDeclartion },
    State { metadata: MetadataDeclaration, state_machine: StateMachineDeclaration },
}
struct StateDeclaration {
    name: Ident,
    _split: Token!(:),
    inner: FunctionBobyDeclartion,
}
pub(crate) struct StateMachineDeclaration {
    wrap: Brace,
    state_list: Punctuated<StateDeclaration, Token!(,)>,
}
enum GenericcArgumentsDeclaration {
    Var(VarDeclaration),
    Lit(Lit),
    Path(Path),
}
enum ExprDeclaration {
    Call(FunctionCall),
    Var(VarDeclaration),
    Lit(Lit),
    Path(Path),
    RustFn(Path),
}
struct FunctionCall {
    function: Path,
    call_body: CallBody,
}
struct CallBody {
    _paren: Paren,
    generics: Vec<GenericcArgumentsDeclaration>,
    args: Punctuated<ExprDeclaration, Token!(,)>,
}
pub(crate) struct FunctionBobyDeclartion {
    _wrap: Brace,
    basic_blocks: Vec<BasicBlockDeclartion>,
}
struct BasicBlockDeclartion {
    name: Ident,
    _split: Token!(:),
    _wrap: Brace,
    instruction_list: Punctuated<StatDeclaration, Token!(;)>,
}
struct VarDeclaration {
    _prefix: Token!(%),
    name: Ident,
}
enum StatDeclaration {
    Call(FunctionCall),
    Put { vars: Vec<VarDeclaration>, put: Token!(=), expr: ExprDeclaration },
    Branch { prefix: Ident, target: VarDeclaration },
    BranchIf { prefix: Token!(if), expr: ExprDeclaration, true_target: VarDeclaration, false_target: VarDeclaration },
    Return { prefix: Token!(return), values: Vec<ExprDeclaration> },
    Phi { variable: VarDeclaration, _pre_type: Token!(:), ty: Type, _split: Token!(=), _wrap: Brace, from: Punctuated<PhiElementDeclaration, Token!(,)> },
}
struct PhiElementDeclaration {
    block: VarDeclaration,
    _split: Token!(=>),
    var: VarDeclaration,
}
pub(crate) struct MetadataDeclaration {
    _wrap: Paren,
    generics: Vec<GenericsDeclaration>,
    args: Vec<ArgumentDeclaration>,
    returns: Vec<ArgumentDeclaration>,
}
struct ArgumentDeclaration {
    name: Ident,
    _split: Token!(:),
    ty: Type,
}
#[derive(Clone)]
enum GenericsDeclarationKind {
    Constant { ty: Type, mutable: bool },
    BasicBlock,
    Type,
}
#[derive(Clone)]
struct GenericsDeclaration {
    name: Ident,
    kind: GenericsDeclarationKind,
}

impl Parse for InstructionKindDeclaration {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Token!(fn)) {
            let _fn: Token!(fn) = input.parse()?;
            let metadata = input.parse()?;
            Ok(Self::Complex { metadata, function_boby: input.parse()? })
        } else if lookahead.peek(token::Brace) {
            let content;
            let _wrap = braced!(content in input);
            let metadata = content.parse()?;
            Ok(Self::State { metadata, state_machine: content.parse()? })
        } else if lookahead.peek(token::Bracket) {
            let content;
            Ok(Self::Compresion { _wrap: bracketed!(content in input), list: content.parse_terminated(InstructionDeclaration::parse)? })
        } else {
            Ok(Self::DirectMap { instruction: input.parse()? })
        }
    }
}
impl Parse for StateDeclaration {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Ok(Self { name: input.call(Ident::parse_any)?, _split: input.parse()?, inner: input.parse()? })
    }
}
impl Parse for StateMachineDeclaration {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let content;
        Ok(Self { wrap: braced!(content in input), state_list: content.parse_terminated(StateDeclaration::parse)? })
    }
}
impl Parse for GenericcArgumentsDeclaration {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let lookahead1 = input.lookahead1();
        if lookahead1.peek(Token!(%)) {
            Ok(Self::Var(input.parse()?))
        } else if lookahead1.peek(Lit) {
            Ok(Self::Lit(input.parse()?))
        } else {
            Ok(Self::Path(input.parse()?))
        }
    }
}
impl Parse for ExprDeclaration {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let lookahead1 = input.lookahead1();
        if lookahead1.peek(Token!(%)) {
            Ok(Self::Var(input.parse()?))
        } else if lookahead1.peek(Token!(fn)) {
            Ok(Self::RustFn(input.parse()?))
        } else if lookahead1.peek(Lit) {
            Ok(Self::Lit(input.parse()?))
        } else {
            let path: Path = input.call(Path::parse_mod_style)?;
            if input.peek(Paren) || input.peek(Token!(<)) {
                Ok(Self::Call(FunctionCall { function: path, call_body: input.parse()? }))
            } else {
                Ok(Self::Path(path))
            }
        }
    }
}
impl Parse for FunctionCall {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Ok(Self { function: input.call(Path::parse_mod_style)?, call_body: input.parse()? })
    }
}
impl Parse for CallBody {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let content;
        Ok(Self {
            generics: {
                if input.peek(Token!(<)) {
                    let mut r = Vec::new();
                    let _: Token!(<) = input.parse()?;
                    if !input.peek(Token!(>)) {
                        r.push(input.parse()?);
                        while !input.peek(Token!(>)) {
                            let _: Token!(,) = input.parse()?;
                            r.push(input.parse()?);
                        }
                    }
                    let _: Token!(>) = input.parse()?;
                    r
                } else {
                    Vec::new()
                }
            },
            _paren: parenthesized!(content in input),
            args: content.parse_terminated(ExprDeclaration::parse)?,
        })
    }
}
impl Parse for VarDeclaration {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Ok(Self { _prefix: input.parse()?, name: input.call(Ident::parse_any)? })
    }
}
impl Parse for StatDeclaration {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let lookahead1 = input.lookahead1();
        if lookahead1.peek(Token!(if)) {
            Ok(Self::BranchIf { prefix: input.parse()?, expr: input.parse()?, true_target: input.parse()?, false_target: input.parse()? })
        } else if lookahead1.peek(Token!(return)) {
            let prefix: Token!(return) = input.parse()?;
            let mut values = Vec::new();
            if !input.peek(Token!(;)) {
                values.push(input.parse()?);
                while !input.peek(Token!(;)) {
                    let _split: Token!(,) = input.parse()?;
                    values.push(input.parse()?);
                }
            }
            Ok(Self::Return { prefix, values })
        } else if lookahead1.peek(Token!(%)) {
            let ident = input.parse()?;
            let mut vars = vec![ident];
            while input.peek(Token!(%)) {
                vars.push(input.parse()?);
            }
            let put: Token!(=) = input.parse()?;
            Ok(Self::Put { vars, put, expr: input.parse()? })
        } else {
            let path: Path = input.call(Path::parse_mod_style)?;
            match path.get_ident().map(|i| i.to_string()).as_deref() {
                Some("phi") => {
                    let content;
                    Ok(Self::Phi {
                        variable: input.parse()?,
                        _pre_type: input.parse()?,
                        ty: input.parse()?,
                        _split: input.parse()?,
                        _wrap: braced!(content in input),
                        from: content.parse_terminated(PhiElementDeclaration::parse)?,
                    })
                }
                Some("branch") => Ok(Self::Branch { prefix: path.get_ident().unwrap().clone(), target: input.parse()? }),
                _ => Ok(Self::Call(FunctionCall { function: path, call_body: input.parse()? })),
            }
        }
    }
}
impl Parse for BasicBlockDeclartion {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let content;
        Ok(Self {
            name: input.call(Ident::parse_any)?,
            _split: input.parse()?,
            _wrap: braced!(content in input),
            instruction_list: content.parse_terminated(StatDeclaration::parse)?,
        })
    }
}
impl Parse for FunctionBobyDeclartion {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let content;
        Ok(Self {
            _wrap: braced!(content in input),
            basic_blocks: {
                let basic_block_list: Punctuated<BasicBlockDeclartion, Token!(,)> = content.parse_terminated(BasicBlockDeclartion::parse)?;
                Vec::from_iter(basic_block_list.into_iter())
            },
        })
    }
}
impl Parse for PhiElementDeclaration {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Ok(Self { block: input.parse()?, _split: input.parse()?, var: input.parse()? })
    }
}
impl Parse for GenericsDeclaration {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let ident: Ident = input.call(Ident::parse_any)?;
        match &*ident.to_string() {
            "const" => Ok(Self {
                name: input.call(Ident::parse_any)?,
                kind: {
                    let _: Token!(:) = input.parse()?;
                    GenericsDeclarationKind::Constant { mutable: false, ty: input.parse()? }
                },
            }),
            "mut" => Ok(Self {
                name: input.call(Ident::parse_any)?,
                kind: {
                    let _: Token!(:) = input.parse()?;
                    GenericsDeclarationKind::Constant { mutable: true, ty: input.parse()? }
                },
            }),
            "block" => Ok(Self { name: input.call(Ident::parse_any)?, kind: GenericsDeclarationKind::BasicBlock }),
            "type" => Ok(Self { name: input.call(Ident::parse_any)?, kind: GenericsDeclarationKind::Type }),
            _ => Err(Error::new(ident.span(), "unknown keyword")),
        }
    }
}
impl Parse for ArgumentDeclaration {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Ok(Self { name: input.call(Ident::parse_any)?, _split: input.parse()?, ty: input.parse()? })
    }
}
impl Parse for MetadataDeclaration {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let content;
        Ok(Self {
            generics: {
                let mut generics = Vec::new();
                if input.peek(Token!(<)) {
                    let _: Token!(<) = input.parse()?;
                    while !input.peek(Token!(>)) {
                        generics.push(input.parse()?);
                        let _: Option<Token!(,)> = input.parse()?;
                    }
                    let _: Token!(>) = input.parse()?;
                }
                generics
            },
            _wrap: parenthesized!(content in input),
            args: {
                let args: Punctuated<_, Token!(,)> = content.parse_terminated(ArgumentDeclaration::parse)?;
                args.into_iter().collect()
            },
            returns: {
                if input.peek(Token!(->)) {
                    let _split: Token!(->) = input.parse()?;
                    let content;
                    let _brace = parenthesized!(content in input);
                    let returns_row: Punctuated<_, Token!(,)> = content.parse_terminated(ArgumentDeclaration::parse)?;
                    returns_row.into_iter().collect()
                } else {
                    Vec::new()
                }
            },
        })
    }
}
impl Parse for InstructionDeclaration {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Ok(Self { name: input.parse()?, _derive: input.parse()?, instruction_kind: input.parse()? })
    }
}

fn wrap_struct(name: &Path, emit: TokenStream2, instruction_type: TokenStream2, structure: &Option<Structure>, state_count: usize) -> Result<TokenStream2> {
    if let Some(structure) = structure {
        let ast = structure.ast();
        let (_impl_generics, impl_generics, where_clause) = ast.generics.split_for_impl();
        let generics = &ast.generics;
        let name = &ast.ident;
        Ok(quote! {
          impl #generics #name #impl_generics #where_clause{
            #emit
          }
          impl #generics runtime::instructions::Instruction for #name #impl_generics #where_clause{
            const INSTRUCTION_TYPE: runtime::instructions::InstructionType = #instruction_type;
            const STATE_COUNT: usize = #state_count;
          }
        })
    } else {
        Ok(quote! {
            pub enum #name{}
            impl #name{
                #emit
            }
            impl runtime::instructions::Instruction for #name{
                const INSTRUCTION_TYPE: runtime::instructions::InstructionType = #instruction_type;
                const STATE_COUNT: usize = #state_count;
            }
        })
    }
}
struct FunctionBobyContext {
    template_variable_count: usize,
    _generic_map: HashMap<String, GenericsDeclaration>,
}
#[derive(Default, Clone)]
pub struct BuildInstructionConfig<'a> {
    structure: Option<Structure<'a>>,
    impl_generics: Option<ImplGenerics<'a>>,
    generic_params: HashMap<String, GenericKind>,
}
impl InstructionDeclaration {
    pub fn get_name(&self) -> &Path {
        &self.name
    }

    pub fn build_instruction(&self, config: &BuildInstructionConfig) -> Result<TokenStream2> {
        let name = &self.name;
        let impl_generics = &config.impl_generics;
        let vis = config
            .structure
            .as_ref()
            .map(|s| {
                let vis = &s.ast().vis;
                quote! {#vis}
            })
            .unwrap_or_else(|| quote! {pub});
        match &self.instruction_kind {
            InstructionKindDeclaration::DirectMap { instruction } => Ok(quote! {
              #vis type #name=#instruction;
            }),
            InstructionKindDeclaration::Compresion { _wrap, list } => {
                let mut r = Vec::new();
                for e in list {
                    let sub_config = BuildInstructionConfig { ..config.clone() };
                    r.push(e.build_instruction(&sub_config)?);
                }
                Ok(quote! {
                  #(#r)*
                })
            }
            InstructionKindDeclaration::Complex { metadata, function_boby } => {
                let emit = metadata.generate_emit()?;
                let instruction_metadata = metadata.generate_matedata(false, config)?;
                let instruction_boby = function_boby.generate(&*metadata.generics, config)?;
                let complex_instruction_name = name.into_token_stream().to_string();
                let instruction_type = quote! {
                      runtime::instructions::InstructionType::Complex(
                        runtime::_util::CowArc::Ref(
                          runtime::_util::inline_const!(
                            #impl_generics[&'static runtime::instructions::ComplexInstruction ]
                            &runtime::instructions::ComplexInstruction{
                              name: std::borrow::Cow::Borrowed(&#complex_instruction_name),
                              metadata: #instruction_metadata,
                              blocks: #instruction_boby,
                        }
                      )
                    ))
                };
                wrap_struct(name, emit, instruction_type, &config.structure, 1)
            }
            InstructionKindDeclaration::State { metadata, state_machine } => {
                let emit = metadata.generate_emit()?;
                let instruction_metadata = metadata.generate_matedata(true, config)?;
                let boost = state_machine.state_list.first().ok_or_else(|| Error::new(state_machine.wrap.span, "no state found"))?.name.to_string();
                let stateful_instruction_name = name.into_token_stream().to_string();
                let state_body = state_machine.generate(&*metadata.generics, config, &instruction_metadata, &*stateful_instruction_name)?;
                let instruction_type = quote! {
                      runtime::instructions::InstructionType::Stateful(
                        runtime::_util::CowArc::Ref(
                          runtime::_util::inline_const!(#impl_generics[&'static runtime::instructions::StatefulInstruction]
                            &runtime::instructions::StatefulInstruction{
                              metadata: #instruction_metadata,
                              boost: std::borrow::Cow::Borrowed(&#boost),
                              statuses: #state_body,
                        })))
                };
                wrap_struct(name, emit, instruction_type, &config.structure, state_machine.state_list.len())
            } // InstructionKindDeclaration::Proxy{..} => todo!(),
        }
    }
}
impl MetadataDeclaration {
    fn generate_emit(&self) -> Result<TokenStream2> {
        let mut get_align = Vec::new();
        let mut emit_args = Vec::new();
        let mut emit_generic = Vec::new();
        let mut arg_set = HashSet::new();
        if self.generics.iter().any(|gen| matches!(&gen.kind, &GenericsDeclarationKind::Type)) {
            return Ok(quote! {});
        }
        for gen in &self.generics {
            match &gen.kind {
                GenericsDeclarationKind::Constant { ty: value_type, .. } => {
                    let name = format_ident!("generics_{}", &gen.name);
                    get_align.push(quote! { align=usize::max(align, <#value_type as jvm_core::TypeDeclaration>::LAYOUT.align()); });
                    emit_args.push(quote! {
                      #name : <#value_type as jvm_core::TypeDeclaration>::Impl
                    });
                    emit_generic.push(quote! {
                        {
                            let b=builder.codes().borrow_mut(token);
                            b.align(<#value_type as jvm_core::TypeDeclaration>::LAYOUT.align());
                            <<#value_type as jvm_core::TypeDeclaration>::Impl as jvm_core::MoveIntoObject>::append( #name, b);
                        }
                    });
                }
                GenericsDeclarationKind::BasicBlock => {
                    let name = format_ident!("generics_{}", &gen.name);
                    get_align.push(quote! { align=usize::max(align, 4); });
                    emit_args.push(quote! {
                      #name : &runtime::code::BlockBuilder<'l,S>
                    });
                    emit_generic.push(quote! {
                      builder.codes().borrow_mut(token).align(4);
                      builder.push_block_offset(token,#name);
                    });
                }
                _ => unreachable!(),
            };
        }
        for arg in &self.args {
            let value_type = &arg.ty;
            let name = format_ident!("arg_{}", &arg.name);
            get_align.push(quote! { align=usize::max(align, 2); });
            emit_args.push(quote! {
              #name : &runtime::code::Register<#value_type,A>
            });
            emit_generic.push(quote! {
                builder.codes().borrow_mut(token).align(2);
                builder.emit_register(token,#name);
            });
            arg_set.insert(arg.name.to_string());
        }
        for ret in &self.returns {
            let value_type = &ret.ty;
            if !arg_set.contains(&ret.name.to_string()) {
                let name = format_ident!("ret_{}", &ret.name);
                get_align.push(quote! { align=usize::max(align, 2); });
                emit_args.push(quote! {
                  #name : &runtime::code::Register<#value_type,A>
                });
                emit_generic.push(quote! {
                    builder.codes().borrow_mut(token).align(2);
                    builder.emit_register(token,#name);
                });
                arg_set.insert(ret.name.to_string());
            }
        }
        let alloc = if !self.args.is_empty() || !self.returns.is_empty() {
            Some(quote! {A: runtime::code::RegisterPool,})
        } else {
            None
        };
        let emit = quote! {
          #[allow(dead_code)]
          pub fn emit<'l,S: runtime::instructions::InstructionSet,#alloc>(
            builder: &runtime::code::BlockBuilder<'l,S>,
            token:&mut jvm_core::_ghost_cell::GhostToken<'l>,
            #(#emit_args),*
          )->runtime::_failure::Fallible<()>
          where
              Self: runtime::instructions::InstructionOf<S>,
          {
            unsafe {
              builder.emit_opcode(token,<Self as runtime::instructions::InstructionOf<S>>::OPCODE);
              let mut align=1;
              #(#get_align)*
              builder.codes().borrow_mut(token).align(align);
              #(#emit_generic)*
            }
            Ok(())
          }
        };
        Ok(emit)
    }

    fn generate_matedata(&self, use_state: bool, config: &BuildInstructionConfig) -> Result<TokenStream2> {
        let impl_generics = &config.impl_generics;
        let mut metadata_operands = Vec::new();
        let mut metadata_generic = Vec::new();
        let arg_set: HashSet<String> = HashSet::from_iter(self.args.iter().map(|arg| arg.name.to_string()));
        let ret_set: HashSet<String> = HashSet::from_iter(self.returns.iter().map(|ret| ret.name.to_string()));
        for arg in &self.args {
            let value_type = &arg.ty;
            let name = &arg.name.to_string();
            let output = ret_set.contains(name);
            metadata_operands.push(quote! {
              runtime::instructions::OperandMetadata {
                name: std::borrow::Cow::Borrowed(&#name),
                value_type:<#value_type as jvm_core::TypeDeclaration>::TYPE,
                input:true,
                output:#output,
              }
            });
        }
        for ret in &self.returns {
            let value_type = &ret.ty;
            let name = &ret.name.to_string();
            if !arg_set.contains(name) {
                metadata_operands.push(quote! {
                  runtime::instructions::OperandMetadata {
                    name:std::borrow::Cow::Borrowed(&#name),
                    value_type:<#value_type as jvm_core::TypeDeclaration>::TYPE,
                    input:false,
                    output:true,
                  }
                });
            }
        }
        for gen in &self.generics {
            let name = gen.name.to_string();
            let kind = match &gen.kind {
                GenericsDeclarationKind::Constant { ty, mutable } => {
                    quote! {runtime::instructions::GenericsMetadataKind::Constant{value_type:#ty::TYPE,writable:#mutable}}
                }
                GenericsDeclarationKind::BasicBlock => {
                    quote!(runtime::instructions::GenericsMetadataKind::BasicBlock)
                }
                GenericsDeclarationKind::Type => {
                    quote!(runtime::instructions::GenericsMetadataKind::Type)
                }
            };
            metadata_generic.push(quote! {
              runtime::instructions::GenericsMetadata{
                name: std::borrow::Cow::Borrowed(&#name),
                kind: #kind,
              }
            });
        }
        if use_state {
            metadata_generic.push(quote! {
             runtime::instructions::GenericsMetadata{
               name: std::borrow::Cow::Borrowed("__state"),
               kind: runtime::instructions::GenericsMetadataKind::Constant{value_type:jvm_core::Type::Int(jvm_core::IntKind::U8),writable:true}
             }
            });
        }
        let instruction_metadata = quote! {
          runtime::instructions::InstructionMetadata{
            operands:runtime::_util::CowSlice::Ref(
                       runtime::_util::inline_const!(
                         #impl_generics[&'static [runtime::instructions::OperandMetadata]]
                         &[#(#metadata_operands),*])
                       ),
            generics:runtime::_util::CowSlice::Ref(
              runtime::_util::inline_const!(
                #impl_generics[&'static [runtime::instructions::GenericsMetadata]]
                &[#(#metadata_generic),*])
              ),
          }
        };
        Ok(instruction_metadata)
    }
}
impl FunctionBobyContext {
    fn alloc_variable(&mut self) -> String {
        let id = self.template_variable_count;
        self.template_variable_count += 1;
        id.to_string()
    }
}
impl FunctionBobyDeclartion {
    fn generate(&self, generics: &[GenericsDeclaration], config: &BuildInstructionConfig) -> Result<TokenStream2> {
        let impl_generics = &config.impl_generics;
        let mut basic_blocks = Vec::new();
        let mut context =
            FunctionBobyContext { template_variable_count: 0, _generic_map: HashMap::from_iter(generics.iter().map(|g| (g.name.to_string(), g.clone()))) };
        for basic_block in self.basic_blocks.iter() {
            basic_blocks.push(basic_block.generate(&mut context, config)?);
        }
        Ok(quote! {
          runtime::_util::CowSlice::Ref(
            runtime::_util::inline_const!(
              #impl_generics[&'static [runtime::instructions::BasicBlock]]
              &[#(#basic_blocks),*]
            ))
        })
    }
}
impl BasicBlockDeclartion {
    fn generate(&self, context: &mut FunctionBobyContext, config: &BuildInstructionConfig) -> Result<TokenStream2> {
        let impl_generics = &config.impl_generics;
        let name_string = self.name.to_string();
        let mut phis = Vec::new();
        let mut instructions = Vec::new();
        for instruction in &self.instruction_list {
            if matches!(instruction, StatDeclaration::Phi { .. }) {
                phis.push(instruction.generate(context, config)?);
            } else {
                instructions.push(instruction.generate(context, config)?);
            }
        }
        Ok(quote_spanned! {self._wrap.span=>
          runtime::instructions::BasicBlock{
            id: std::borrow::Cow::Borrowed(&#name_string),
            stat:runtime::_util::CowSlice::Ref(
              runtime::_util::inline_const!(
                #impl_generics[&'static [runtime::instructions::Stat]]
                &[#(#instructions),*]
            )),
            phi:runtime::_util::CowSlice::Ref(
              runtime::_util::inline_const!(
                #impl_generics[&'static [runtime::instructions::Phi]]
                &[ #(#phis),* ]
            )),
          }
        })
    }
}
impl StatDeclaration {
    fn generate(&self, context: &mut FunctionBobyContext, config: &BuildInstructionConfig) -> Result<TokenStream2> {
        let impl_generics = &config.impl_generics;
        match self {
            StatDeclaration::Call(function_call) => {
                let mut instructions = Vec::new();
                function_call.generate_into(&mut instructions, context, &Vec::new(), config)?;
                Ok(quote_spanned! {function_call.function.span()=> #(#instructions),*})
            }
            StatDeclaration::Put { vars, expr, put } => {
                let mut instructions = Vec::new();
                let rets = vars.iter().map(|var| var.name.to_string()).collect();
                match expr {
                    ExprDeclaration::Var(v) => {
                        let src = v.name.to_string();
                        let dst = vars[0].name.to_string();
                        instructions.push(quote_spanned! {src.span()=>
                        runtime::instructions::Stat::Move(std::borrow::Cow::Borrowed(&#dst),std::borrow::Cow::Borrowed(&#src))});
                    }
                    _ => {
                        expr.generate_into(&mut instructions, context, &rets, config)?;
                    }
                };
                Ok(quote_spanned! {put.span()=> #(#instructions),*})
            }
            StatDeclaration::Branch { prefix, target } => {
                let target_block = target.name.to_string();
                Ok(quote_spanned! {prefix.span()=>
                  runtime::instructions::Stat::InstructionCall (
                    runtime::instructions::InstructionCall{
                      rets: runtime::_util::CowSlice::Ref(&[]),
                      args: runtime::_util::CowSlice::Ref(&[]),
                      generics: runtime::_util::CowSlice::Ref(
                        runtime::_util::inline_const!(
                          #impl_generics[&'static [runtime::instructions::GenericArgument]]
                          &[
                              runtime::instructions::GenericArgument::Var(std::borrow::Cow::Borrowed(#target_block)),
                          ]
                      )),
                      instruction: <runtime::instructions::bootstrap::Branch as runtime::instructions::Instruction>::INSTRUCTION_TYPE ,
                    }
                  )

                })
            }
            StatDeclaration::BranchIf { expr, true_target, false_target, prefix, .. } => {
                let mut instructions = Vec::new();
                let expr_ret = expr.generate_into(&mut instructions, context, &Vec::new(), config)?;
                let true_target = &true_target.name.to_string();
                let false_target = &false_target.name.to_string();
                Ok(quote_spanned! {prefix.span()=>
                  #(#instructions,)*
                  runtime::instructions::Stat::InstructionCall (
                    runtime::instructions::InstructionCall{
                      args: runtime::_util::CowSlice::Ref(
                        runtime::_util::inline_const!(
                          #impl_generics[&'static [std::borrow::Cow<'static,str>]]
                          &[std::borrow::Cow::Borrowed(&#expr_ret)]
                      )),
                      generics: runtime::_util::CowSlice::Ref(
                        runtime::_util::inline_const!(
                          #impl_generics[&'static [runtime::instructions::GenericArgument]]
                          &[
                              runtime::instructions::GenericArgument::Var(std::borrow::Cow::Borrowed(#true_target)),
                              runtime::instructions::GenericArgument::Var(std::borrow::Cow::Borrowed(#false_target)),
                          ]
                      )),
                      rets: runtime::_util::CowSlice::Ref(&[]),
                      instruction: <runtime::instructions::bootstrap::BranchIf as runtime::instructions::Instruction>::INSTRUCTION_TYPE ,
                    }
                    )
                })
            }
            StatDeclaration::Return { values, prefix } => {
                let mut instructions = Vec::new();
                let mut args = Vec::new();
                for expr in values {
                    let arg = expr.generate_into(&mut instructions, context, &Vec::new(), config)?;
                    args.push(quote! {std::borrow::Cow::Borrowed(&#arg)});
                }
                Ok(quote_spanned! {prefix.span()=>
                  #(#instructions,)*
                  runtime::instructions::Stat::InstructionCall (
                    runtime::instructions::InstructionCall{
                      generics: runtime::_util::CowSlice::Ref(&[]),
                      args: runtime::_util::CowSlice::Ref(
                        runtime::_util::inline_const!(
                          #impl_generics[&'static[std::borrow::Cow<'static,str>]]
                          &[#(#args),*]
                        )),
                      rets: runtime::_util::CowSlice::Ref(&[]),
                      instruction: <runtime::instructions::bootstrap::Return as runtime::instructions::Instruction>::INSTRUCTION_TYPE ,
                    }
                  )
                })
            }
            StatDeclaration::Phi { variable, _pre_type, ty, _split, _wrap: _content, from } => {
                let mut phi_map = Vec::new();
                for phi_element in from {
                    let block = phi_element.block.name.to_string();
                    let var = phi_element.var.name.to_string();
                    phi_map.push(quote! {
                      (std::borrow::Cow::Borrowed(&#block),std::borrow::Cow::Borrowed(&#var))
                    });
                }
                let var = variable.name.to_string();
                Ok(quote_spanned! {variable.name.span()=>
                  runtime::instructions::Phi{
                    variable: std::borrow::Cow::Borrowed(&#var),
                    ty: <#ty as jvm_core::TypeDeclaration>::TYPE,
                    map: runtime::_util::CowSlice::Ref(
                      runtime::_util::inline_const!(
                        #impl_generics[&'static[(std::borrow::Cow<'static,str>,std::borrow::Cow<'static,str>)]]
                        &[#(#phi_map),*]
                    )),
                  }
                })
            }
        }
    }
}
impl FunctionCall {
    fn generate_into(
        &self,
        target: &mut Vec<TokenStream2>,
        context: &mut FunctionBobyContext,
        rets: &Vec<String>,
        config: &BuildInstructionConfig,
    ) -> Result<()> {
        let impl_generics = &config.impl_generics;
        let mut args = Vec::new();
        for arg in self.call_body.args.iter() {
            let arg_value = arg.generate_into(target, context, &mut Vec::new(), config)?;
            args.push(quote! {std::borrow::Cow::Borrowed(&#arg_value)});
        }
        let mut generics = Vec::new();
        for generic in self.call_body.generics.iter() {
            generics.push(generic.generate(config)?);
        }
        let function = &self.function;
        let span = self.function.span();
        let wraped_rets: Vec<_> = rets.iter().map(|ret| quote! {std::borrow::Cow::Borrowed(&#ret)}).collect();
        target.push(quote_spanned! {span=>
          runtime::instructions::Stat::InstructionCall(
            runtime::instructions::InstructionCall{
              args:runtime::_util::CowSlice::Ref(
                     runtime::_util::inline_const!(#impl_generics[&'static [std::borrow::Cow<'static,str>]]
                       &[#(#args),*])),
              rets:runtime::_util::CowSlice::Ref(
                runtime::_util::inline_const!(
                  #impl_generics[&'static [std::borrow::Cow<'static,str>]]
                  &[#(#wraped_rets),*]
              )),
              generics:runtime::_util::CowSlice::Ref(
                runtime::_util::inline_const!(
                  #impl_generics[&'static [runtime::instructions::GenericArgument]]
                  &[#(#generics),*]
              )),
              instruction: <#function as runtime::instructions::Instruction>::INSTRUCTION_TYPE,
            }
          )
        });
        Ok(())
    }
}
impl ExprDeclaration {
    fn generate_into(
        &self,
        instructions: &mut Vec<TokenStream2>,
        context: &mut FunctionBobyContext,
        rets: &Vec<String>,
        config: &BuildInstructionConfig,
    ) -> Result<String> {
        match self {
            Self::Call(call) => {
                let mut rets = rets.clone();
                if rets.is_empty() {
                    rets.push(context.alloc_variable());
                }
                call.generate_into(instructions, context, &rets, config)?;
                Ok(rets.first().unwrap().clone())
            }
            Self::Var(var) => {
                let name = var.name.to_string();
                Ok(name)
            }
            Self::Lit(lit) => {
                let ret = rets.first().cloned().unwrap_or_else(|| context.alloc_variable());
                let lit = encode_lit(lit)?;
                instructions.push(quote_spanned! {lit.span()=>
                  runtime::instructions::Stat::Lit(std::borrow::Cow::Borrowed(&#ret),#lit)
                });
                Ok(ret)
            }
            Self::Path(path) => {
                let ret = rets.first().cloned().unwrap_or_else(|| context.alloc_variable());
                let lit = encode_path(path, config)?;
                instructions.push(quote_spanned! {lit.span()=>
                  runtime::instructions::Stat::Lit(std::borrow::Cow::Borrowed(&#ret),#lit)
                });
                Ok(ret)
            }
            Self::RustFn(path) => {
                let ret = rets.first().cloned().unwrap_or_else(|| context.alloc_variable());
                instructions.push(quote_spanned! {path.span()=>
                  runtime::instructions::Stat::Lit(std::borrow::Cow::Borrowed(&#ret),Lit::RustFn(#path as *const u8))
                });
                Ok(ret)
            }
        }
    }
}
fn encode_path(path: &Path, config: &BuildInstructionConfig) -> Result<TokenStream2> {
    if let Some(ident) = path.get_ident() {
        if let Some(kind) = config.generic_params.get(&ident.to_string()) {
            Ok(match kind {
                GenericKind::Type => return Err(Error::new(path.span(), "Add `::TYPE` or `#INSTRUCTION_TYPE` ")),
                GenericKind::Int => quote_spanned! {path.span()=>
                  runtime::instructions::Value::I64(#path)
                },
                GenericKind::Float => quote_spanned! {path.span()=>
                  runtime::instructions::Value::Float(#path)
                },
                GenericKind::Str => quote_spanned! {path.span()=>
                  runtime::instructions::Value::Str(#path)
                },
                GenericKind::Bytes => quote_spanned! {path.span()=>
                  runtime::instructions::Value::ByteStr(#path)
                },
                GenericKind::Bool => quote_spanned! {path.span()=>
                  runtime::instructions::Value::Bool(#path)
                },
                GenericKind::Byte => quote_spanned! {path.span()=>
                  runtime::instructions::Value::Byte(#path)
                },
                GenericKind::RustFn => quote_spanned! {path.span()=>
                  runtime::instructions::Value::RustFn(#path as *const u8)
                },
            })
        } else {
            return Err(Error::new(path.span(), format!("The type is unknown. Add `{}={{kind=<...>}}` to the generic parameter.", ident)));
        }
    } else {
        let path_tail = &path.segments.last().ok_or_else(|| Error::new(path.span(), "invalid path"))?.ident;
        let variant = match &*path_tail.to_string() {
            "TYPE" => {
                quote! {Type}
            }
            "INSTRUCTION_TYPE" => {
                quote! {Instruction}
            }
            _ => return Err(Error::new(path_tail.span(), "invalid, except 'TYPE' or 'INSTRUCTION_TYPE' ")),
        };
        Ok(quote! {runtime::instructions::Value::#variant(#path)})
    }
}
fn encode_lit(lit: &Lit) -> Result<TokenStream2> {
    match lit {
        Lit::Str(s) => Ok(quote_spanned! {s.span()=>
          runtime::instructions::Value::Str(std::borrow::Cow::Borrowed(&#s))
        }),
        Lit::ByteStr(s) => Ok(quote_spanned! {s.span()=>
          runtime::instructions::Value::ByteStr(runtime::_util::CowSlice::Ref(#lit))
        }),
        Lit::Float(s) => Ok(quote_spanned! {s.span()=>
          runtime::instructions::Value::F64(#lit)
        }),
        Lit::Int(s) => Ok(quote_spanned! {s.span()=>
          runtime::instructions::Value::I64(#lit)
        }),
        Lit::Byte(s) => Ok(quote_spanned! {s.span()=>
          runtime::instructions::Value::U8(#lit)
        }),
        Lit::Bool(s) => Ok(quote_spanned! {s.span()=>
          runtime::instructions::Value::Bool(#lit)
        }),
        _ => Err(Error::new(lit.span(), "unsupport lit")),
    }
}
impl GenericcArgumentsDeclaration {
    pub fn generate(&self, config: &BuildInstructionConfig) -> Result<TokenStream2> {
        match self {
            Self::Var(v) => {
                let name = v.name.to_string();
                Ok(quote_spanned! {v.name.span()=>
                  runtime::instructions::GenericArgument::Var(std::borrow::Cow::Borrowed(&#name))
                })
            }
            Self::Lit(l) => {
                let lit = encode_lit(l)?;
                Ok(quote_spanned! {l.span()=>
                    runtime::instructions::GenericArgument::Value(#lit)
                })
            }
            Self::Path(path) => {
                let lit = encode_path(path, config)?;
                Ok(quote! {
                    runtime::instructions::GenericArgument::Value(#lit)
                })
            }
        }
    }
}
impl StateMachineDeclaration {
    fn generate(
        &self,
        generics: &[GenericsDeclaration],
        config: &BuildInstructionConfig,
        metadata: &TokenStream2,
        stateful_instruction_name: &str,
    ) -> Result<TokenStream2> {
        let impl_generics = &config.impl_generics;
        let mut states = Vec::new();
        for state in &self.state_list {
            states.push(state.generate(generics, config, metadata, stateful_instruction_name)?);
        }
        Ok(quote_spanned! {self.wrap.span=>
          runtime::_util::CowSlice::Ref(
            runtime::_util::inline_const!(
              #impl_generics[&'static [runtime::instructions::State]]
              &[ #(#states),* ]
          ))
        })
    }
}
impl StateDeclaration {
    fn generate(
        &self,
        generics: &[GenericsDeclaration],
        config: &BuildInstructionConfig,
        metadata: &TokenStream2,
        stateful_instruction_name: &str,
    ) -> Result<TokenStream2> {
        let name = self.name.to_string();
        let function_boby = self.inner.generate(generics, config)?;
        Ok(quote_spanned! {self.name.span()=>
          runtime::instructions::State{
            name: std::borrow::Cow::Borrowed(&#name),
            instruction: runtime::instructions::ComplexInstruction{
                name: std::borrow::Cow::Borrowed(&#stateful_instruction_name),
                metadata: #metadata,
                blocks: #function_boby,
            }
          }
        })
    }
}
