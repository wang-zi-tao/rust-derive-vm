use std::{
    collections::{HashMap, HashSet},
    fmt::{Debug, Display},
    hash::Hash,
    ptr,
    rc::Rc,
};

use proc_macro2::Ident;

use super::parse::*;
use syn::{Expr, Result, Type};
extern crate proc_macro2;
use proc_macro2::TokenStream as TokenStream2;

use syn::{spanned::Spanned, Error};
#[derive(Debug, Hash, PartialEq, Eq)]
pub(crate) enum VariantKind {
    None,
    Truple,
}
pub(crate) struct Terminal {
    pub(crate) ident: Ident,
    pub(crate) name: String,
    pub(crate) variant_kind: VariantKind,
}
impl Debug for Terminal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&*self.ident.to_string())
    }
}

impl Ord for Terminal {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for Terminal {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.name.partial_cmp(&other.name)
    }
}

impl Eq for Terminal {}

impl PartialEq for Terminal {
    fn eq(&self, other: &Self) -> bool {
        self.name.eq(&other.name)
    }
}

impl Hash for Terminal {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}
impl Display for Terminal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.ident.to_string())
    }
}
pub(crate) struct NonTerminal {
    pub(crate) ident: Ident,
    pub(crate) name: String,
    pub(crate) output: Option<Type>,
}

impl Ord for NonTerminal {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for NonTerminal {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.name.partial_cmp(&other.name)
    }
}

impl Eq for NonTerminal {}

impl PartialEq for NonTerminal {
    fn eq(&self, other: &Self) -> bool {
        self.name.eq(&other.name)
    }
}

impl Hash for NonTerminal {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}
impl Debug for NonTerminal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&*self.ident.to_string())
    }
}
#[derive(Clone)]
pub(crate) struct Production {
    pub(crate) left_part: Rc<NonTerminal>,
    pub(crate) right_part: Vec<(Symbol, Option<TokenStream2>)>,
    pub(crate) callback: Option<Expr>,
}
impl Debug for Production {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.left_part.fmt(f)?;
        f.write_str("->")?;
        for r in &self.right_part {
            f.write_str(&r.0.get_ident().to_string())?;
            f.write_str(",")?;
        }
        Ok(())
    }
}
impl Hash for Production {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.left_part.hash(state);
        self.right_part.iter().for_each(|(s, _)| {
            s.hash(state);
        })
    }
}

impl PartialEq for Production {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self, other)
    }
}
impl Eq for Production {}
impl PartialOrd for Production {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (self as *const Self as usize).partial_cmp(&(other as *const Self as usize))
    }
}
impl Ord for Production {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self as *const Self as usize).cmp(&(other as *const Self as usize))
    }
}
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub(crate) enum Symbol {
    Terminal(Rc<Terminal>),
    NonTerminal(Rc<NonTerminal>),
}
impl Symbol {
    pub(crate) fn get_ident(&self) -> &Ident {
        match self {
            Self::Terminal(t) => &t.ident,
            Self::NonTerminal(n) => &n.ident,
        }
    }
}
pub(crate) type ProductionMap = HashMap<Rc<NonTerminal>, Vec<Rc<Production>>, ahash::RandomState>;
pub(crate) type SelectSet = HashMap<Rc<Production>, Vec<Option<Rc<Terminal>>>, ahash::RandomState>;
pub(crate) fn select_set(productions: &ProductionMap) -> SelectSet {
    let first_set = first_set(productions);
    let mut maybe_empty = HashSet::with_hasher(ahash::RandomState::with_seed(0));
    maybe_empty.extend(first_set.iter().filter_map(|(production, first)| if first.contains(&None) { Some(&*production.left_part) } else { None }));
    let follow_set = follow_set(productions, &maybe_empty);
    let mut select_set = HashMap::with_hasher(ahash::RandomState::with_seed(0));
    for (production, first_set) in first_set {
        let entry = select_set.entry(production.clone()).or_insert_with(Vec::new);
        entry.extend(first_set.iter().cloned());
        if first_set.contains(&None) {
            if let Some(follow) = follow_set.get(&*production.left_part) {
                entry.extend(follow.iter().map(|f| Some(f.clone())));
            }
        }
    }
    select_set
}
pub(crate) type FollowSet = HashMap<Rc<NonTerminal>, Vec<Rc<Terminal>>, ahash::RandomState>;
pub(crate) fn follow_set<'t>(productions: &'t ProductionMap, maybe_empty: &HashSet<&'t NonTerminal, ahash::RandomState>) -> FollowSet {
    let mut result = HashMap::with_hasher(ahash::RandomState::with_seed(0));
    let mut queue: Vec<_> = productions
        .values()
        .flat_map(|v| v.iter())
        .flat_map(|production| {
            let mut v = Vec::new();
            for i in 1..production.right_part.len() {
                if let Symbol::NonTerminal(n) = &production.right_part[i - 1].0 {
                    for (follow, _) in &production.right_part[i..] {
                        v.push((n.clone(), follow));
                        if let Symbol::NonTerminal(n) = follow {
                            if maybe_empty.contains(&**n) {
                                break;
                            }
                        }
                    }
                }
            }
            v
        })
        .collect();
    while let Some((front, follow)) = queue.pop() {
        match follow {
            Symbol::Terminal(t) => {
                result.entry(front).or_insert_with(Vec::new).push(t.clone());
            }
            Symbol::NonTerminal(t) => {
                for production in productions.get(t).iter().flat_map(|production_list| production_list.iter()) {
                    for i in 1..production.right_part.len() {
                        if let Symbol::NonTerminal(n) = &production.right_part[i - 1].0 {
                            for (follow, _) in &production.right_part[i..] {
                                queue.push((n.clone(), follow));
                                if let Symbol::NonTerminal(n) = follow {
                                    if maybe_empty.contains(&**n) {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    result
}

pub(crate) type FirstSet = HashMap<Rc<Production>, Vec<Option<Rc<Terminal>>>, ahash::RandomState>;
pub(crate) fn first_set(productions: &ProductionMap) -> FirstSet {
    let mut result = HashMap::with_hasher(ahash::RandomState::with_seed(0));
    let mut history = HashSet::with_hasher(ahash::RandomState::with_seed(0));
    let mut queue: Vec<_> = productions
        .values()
        .flat_map(|v| v.iter())
        .map(|production| production.right_part.get(0).map(|(first_symbol, _)| (Some(first_symbol), production.clone())).unwrap_or((None, production.clone())))
        .collect();
    while let Some((symbol, production)) = queue.pop() {
        history.insert((symbol, production.clone()));
        match &symbol {
            Some(Symbol::Terminal(t)) => {
                result.entry(production).or_insert_with(Vec::new).push(Some(t.clone()));
            }
            Some(Symbol::NonTerminal(t)) => {
                for symbol_production in productions.get(t).iter().flat_map(|production_list| production_list.iter()) {
                    let operation = (symbol_production.right_part.first().map(|t| &t.0), production.clone());
                    if !history.contains(&operation) {
                        queue.push(operation);
                    }
                }
            }
            None => {
                result.entry(production).or_insert_with(Vec::new).push(None);
            }
        }
    }
    result
}
pub(crate) struct Syntax {
    pub(crate) productions: ProductionMap,
    pub(crate) start: Rc<NonTerminal>,
    pub(crate) token_type: Type,
}
pub(crate) fn parse_syntax_declaration(syntax: SyntaxDeclaration) -> Result<Syntax> {
    let mut nonterminals = HashMap::with_hasher(ahash::RandomState::with_seed(0));
    for nonterminal_declaration in &syntax.nonterminals {
        let nonterminal = NonTerminal {
            ident: nonterminal_declaration.ident.clone(),
            name: nonterminal_declaration.ident.to_string(),
            output: match &nonterminal_declaration.output {
                OutputDeclaration::Some { output, .. } => Some(*output.clone()),
                OutputDeclaration::None => None,
            },
        };
        nonterminals.insert(nonterminal_declaration.ident.clone(), Rc::new(nonterminal));
    }
    let mut terminals = HashMap::with_hasher(ahash::RandomState::with_seed(0));
    let mut production_map = HashMap::with_hasher(ahash::RandomState::with_seed(0));
    for nonterminal_declaration in &syntax.nonterminals {
        let left_part = nonterminals.get(&nonterminal_declaration.ident).unwrap().clone();
        let mut productions = Vec::new();
        for production_declaration in &nonterminal_declaration.productions {
            let symbols = &production_declaration.symbols;
            let callback_declaration = &production_declaration.callback;
            let mut right_part = Vec::new();
            for symbol_declaration in symbols {
                let value = match &symbol_declaration.value {
                    MatchValueDeclaration::Some { token, .. } => Some(token.clone()),
                    MatchValueDeclaration::None => None,
                };
                let symbol = if symbol_declaration.ident.to_string().chars().next().unwrap().is_uppercase() {
                    let terminal = terminals
                        .entry(symbol_declaration.ident.clone())
                        .or_insert_with(|| {
                            let variant_kind = if let MatchValueDeclaration::Some { .. } = symbol_declaration.value {
                                VariantKind::Truple
                            } else {
                                VariantKind::None
                            };
                            Rc::new(Terminal { ident: symbol_declaration.ident.clone(), name: symbol_declaration.ident.to_string(), variant_kind })
                        })
                        .clone();
                    Symbol::Terminal(terminal)
                } else {
                    Symbol::NonTerminal(
                        nonterminals.get(&symbol_declaration.ident).ok_or_else(|| Error::new(symbol_declaration.ident.span(), "unknown nonterminal"))?.clone(),
                    )
                };
                right_part.push((symbol, value))
            }
            let callback = match callback_declaration {
                CallbackDeclaration::None => None,
                CallbackDeclaration::Some { callback, .. } => Some(callback.clone()),
            };
            productions.push(Rc::new(Production { left_part: left_part.clone(), right_part, callback: callback.map(|c| *c) }));
        }
        production_map.insert(left_part, productions);
    }
    if syntax.nonterminals.is_empty() {
        return Err(Error::new(syntax.brace.span, "should not be empty"));
    }
    let root_declaration = &syntax.nonterminals[0];
    let root = nonterminals.get(&root_declaration.ident).cloned().unwrap();
    match &root.output {
        None => match &syntax.output_type {
            Type::Tuple(t) if t.elems.is_empty() => {}
            _ => {
                return Err(Error::new(syntax.output_type.span(), "except `()`"));
            }
        },
        Some(s) => match &root_declaration.output {
            OutputDeclaration::Some { output, .. } if s == &**output => {}
            _ => return Err(Error::new(s.span(), "the output type of start nonterminal is not equal with the output type of the function")),
        },
    }
    Ok(Syntax { productions: production_map, start: root, token_type: syntax.lexical })
}
