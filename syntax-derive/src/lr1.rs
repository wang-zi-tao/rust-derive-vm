use std::{
    cell::RefCell,
    cmp::Ordering,
    collections::{hash_map::Entry, BTreeMap, BTreeSet, HashMap, HashSet},
    fmt::Debug,
    hash::Hash,
    iter::FromIterator,
    rc::Rc,
};

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};

use syn::{Error, Result, Type};
extern crate proc_macro2;

use super::{parse::*, production::*};
use syn::spanned::Spanned;
type ItemCluster = BTreeMap<LR0Item, BTreeSet<Option<Rc<Terminal>>>>;
#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct LR0Item {
    position: usize,
    production: Rc<Production>,
}

impl Debug for LR0Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Item({}@{:?})", self.position, &self.production))
    }
}
impl LR0Item {
    pub fn add_position(self) -> Self {
        Self { position: self.position + 1, ..self }
    }

    pub fn next_symbol(&self) -> Option<Symbol> {
        self.production.right_part.get(self.position).map(|(symbol, _value_ident)| symbol.clone())
    }
}
#[derive(Clone, PartialEq, Eq)]
enum Action {
    Shift(Rc<RefCell<Node>>),
    Reduce(Rc<Production>),
    Accept(Rc<Production>),
}

impl PartialOrd for Action {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Action {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Action::Shift(s), Action::Shift(s1)) => s.borrow().id.cmp(&s1.borrow().id),
            (Action::Reduce(p), Action::Reduce(p1)) => p.cmp(p1),
            (Action::Accept(p), Action::Accept(p1)) => p.cmp(p1),
            (Action::Shift(_), Action::Reduce(_)) | (Action::Shift(_), Action::Accept(_)) | (Action::Reduce(_), Action::Accept(_)) => Ordering::Less,
            (Action::Reduce(_), Action::Shift(_)) | (Action::Accept(_), Action::Shift(_)) | (Action::Accept(_), Action::Reduce(_)) => Ordering::Greater,
        }
    }
}

impl Hash for Action {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Action::Shift(n) => n.borrow().id.hash(state),
            Action::Reduce(p) => p.hash(state),
            Action::Accept(p) => p.hash(state),
        }
    }
}

impl Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Shift(arg0) => f.debug_tuple("Shift").field(&arg0.try_borrow().unwrap().id).finish(),
            Self::Reduce(arg0) => f.debug_tuple("Reduce").field(arg0).finish(),
            Self::Accept(arg0) => f.debug_tuple("Accept").field(arg0).finish(),
        }
    }
}
struct Node {
    source_items: ItemCluster,
    items: ItemCluster,
    action_map: HashMap<Option<Rc<Terminal>>, Action, ahash::RandomState>,
    goto_map: HashMap<Rc<NonTerminal>, Rc<RefCell<Node>>, ahash::RandomState>,
    id: usize,
}

impl Hash for Node {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Node {}
impl Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("id", &self.id)
            .field("source_items", &self.source_items)
            .field("items", &self.items)
            .field("action_map", &self.action_map)
            .field("goto_map", &BTreeMap::from_iter(self.goto_map.iter().map(|(t, g)| (t, g.borrow().id))))
            .finish()
    }
}
struct SyntaxLR1 {
    syntax: Syntax,
    nodes: Vec<Rc<RefCell<Node>>>,
}
impl SyntaxLR1 {
    pub fn generate(&self, name: Ident) -> Result<TokenStream2> {
        let Self { syntax, nodes } = self;
        let Syntax { productions: _productions, start, token_type } = syntax;
        let mut stack_map = HashMap::<Symbol, Option<Type>, ahash::RandomState>::with_hasher(ahash::RandomState::with_seed(0));
        for (nonterminal, _productions) in &syntax.productions {
            stack_map.insert(Symbol::NonTerminal(nonterminal.clone()), nonterminal.output.clone()).ok_or(()).expect_err(&format!("{}:{}", file!(), line!()));
        }
        for (_nonterminal, productions) in &syntax.productions {
            for production in productions {
                for (symbol, value_ident) in &production.right_part {
                    if value_ident.is_some() {
                        stack_map.entry(symbol.clone()).or_insert_with(|| None);
                    }
                }
            }
        }
        let mut goto_case_map = HashMap::<Ident, Vec<TokenStream2>, ahash::RandomState>::with_hasher(ahash::RandomState::with_seed(0));
        for (node_id, node) in nodes.iter().enumerate() {
            let node = node.try_borrow().unwrap();
            for (nonterminal, goto_node) in &node.goto_map {
                let goto_node_id = goto_node.try_borrow().unwrap().id;
                goto_case_map.entry(nonterminal.ident.clone()).or_default().push(quote! {
                  #node_id => #goto_node_id,
                });
            }
        }
        let generate_action = |terminal: &Option<Rc<Terminal>>, operation: &Action| -> Result<TokenStream2> {
            let case_body = match operation {
                Action::Shift(next) => {
                    let next_state_id = next.try_borrow().unwrap().id;
                    let push_value = terminal
                        .as_ref()
                        .and_then(|t| if stack_map.contains_key(&Symbol::Terminal(t.clone())) { Some(&t.ident) } else { None })
                        .map(|stack| {
                            let stack_ident = Ident::new(&format!("stack_{}", stack.to_string().to_lowercase()), stack.span());
                            let push_next = terminal.as_ref().map_or_else(
                                || quote! {None},
                                |terminal_unwrap| {
                                    let ident = &terminal_unwrap.ident;
                                    match terminal_unwrap.variant_kind {
                                        VariantKind::None => quote! {Some(#token_type::#ident)=>{#stack_ident.push(());}},
                                        VariantKind::Truple => quote! {Some(#token_type::#ident(value))=>{#stack_ident.push(value);}},
                                    }
                                },
                            );
                            quote! {
                                match iter.next(){
                                    #push_next
                                    _ => std::unreachable!(),
                                }
                            }
                        })
                        .unwrap_or_else(|| {
                            quote! {let _ = iter.next().unwrap();}
                        });
                    let push_state = quote! {
                      state_stack.push(#next_state_id);
                    };
                    quote! {
                      #push_state
                      #push_value
                    }
                }
                Action::Reduce(production) => {
                    let span = production.callback.as_ref().map(|e| e.span()).unwrap_or_else(|| production.left_part.ident.span());
                    let mut pop_values = Vec::new();
                    for (symbol, value_ident) in production.right_part.iter().rev() {
                        if stack_map.contains_key(symbol) {
                            let token_stream_unwrap = value_ident.clone().unwrap_or_else(|| quote! {_});
                            let stack = symbol.get_ident();
                            let stack_ident = Ident::new(&format!("stack_{}", stack.to_string().to_lowercase()), stack.span());
                            pop_values.push(quote_spanned! {span=>
                              let #token_stream_unwrap=#stack_ident.pop().unwrap();
                            });
                        }
                    }
                    let production_parser_callback =
                        production.callback.as_ref().map(|c| quote_spanned! {span=>#c}).unwrap_or(quote_spanned! {span=>Ok(::syntax::_Default::default())});

                    let pop_count = production.right_part.len();
                    let emit = if stack_map.contains_key(&Symbol::NonTerminal(production.left_part.clone())) {
                        let stack_ident =
                            Ident::new(&format!("stack_{}", production.left_part.ident.to_string().to_lowercase()), production.left_part.ident.span());
                        quote_spanned! {span=>
                          let result=#production_parser_callback;
                          match result{
                            Ok(r)=>#stack_ident.push(r),
                            Err(e)=>return Err(e),
                          }
                        }
                    } else {
                        quote_spanned! {span=>
                          let result=#production_parser_callback;
                          match result{
                            Ok(r)=>{},
                            Err(e)=>return Err(e),
                          }
                        }
                    };
                    let goto_function_ident = Ident::new(&format!("goto_{}", production.left_part.ident), production.left_part.ident.span());
                    quote_spanned! {span=>
                      #(#pop_values)*
                      #emit
                      #goto_function_ident(&mut state_stack,#pop_count)?;
                    }
                }
                Action::Accept(production) => {
                    let mut pop_values = Vec::new();
                    for (symbol, token_stream) in production.right_part.iter() {
                        if stack_map.contains_key(symbol) {
                            let value_ident_unwrap = token_stream.clone().unwrap_or_else(|| quote! {_});
                            let stack = symbol.get_ident();
                            let stack_ident = Ident::new(&format!("stack_{}", stack.to_string().to_lowercase()), stack.span());
                            pop_values.push(quote! {
                              let #value_ident_unwrap=#stack_ident.pop().unwrap();
                            });
                        }
                    }
                    let production_parser_callback = production.callback.as_ref().map(|c| quote! {#c}).unwrap_or(quote! {Ok(())});

                    let pop_count = production.right_part.len();
                    let goto_function_ident = Ident::new(&format!("goto_{}", production.left_part.ident), production.left_part.ident.span());
                    let reduce = if goto_case_map.contains_key(&production.left_part.ident) {
                        let emit = if stack_map.contains_key(&Symbol::NonTerminal(production.left_part.clone())) {
                            let stack_ident =
                                Ident::new(&format!("stack_{}", production.left_part.ident.to_string().to_lowercase()), production.left_part.ident.span());
                            quote! {
                              match result{
                                Ok(r)=>#stack_ident.push(r),
                                Err(e)=>return Err(e),
                              }
                            }
                        } else {
                            quote! {
                              match result{
                                Ok(r)=>{},
                                Err(e)=>return Err(e),
                              }
                            }
                        };
                        quote! {
                            #emit
                            #goto_function_ident(&mut state_stack,#pop_count)?;
                        }
                    } else {
                        quote! {
                          return Err(::syntax::_format_err!("state stack is not empty,pop_count:{},state stack:{:?}",#pop_count,&state_stack));
                        }
                    };
                    quote! {
                      #(#pop_values)*
                      let result=#production_parser_callback;
                      if state_stack.len()!=#pop_count+1{
                        #reduce
                      }else{
                        if iter.next().is_some(){
                          return Err(::syntax::_format_err!("except EOF"));
                        }
                        return result;
                      }
                    }
                }
            };
            Ok(case_body)
        };
        let mut action_map = BTreeMap::new();
        for (_node_id, node_cell) in nodes.iter().enumerate() {
            let node = node_cell.try_borrow().unwrap();
            let mut excepts = Vec::new();
            for (terminal, operation) in &node.action_map {
                excepts.push(terminal.as_ref().map(|t| t.ident.to_string()).unwrap_or_else(|| "no thing".to_string()));
                let terminal_key = match operation {
                    Action::Shift(_) => terminal.clone(),
                    _ => None,
                };
                match action_map.entry((terminal_key, operation.clone())) {
                    std::collections::btree_map::Entry::Vacant(v) => {
                        v.insert((generate_action(terminal, operation)?, Vec::new())).1.push((terminal.clone(), node_cell.clone()));
                    }
                    std::collections::btree_map::Entry::Occupied(mut o) => o.get_mut().1.push((terminal.clone(), node_cell.clone())),
                };
            }
        }
        let mut state_transition = action_map
            .into_iter()
            .map(|((_terminal, _action), (tokens, match_keys))| {
                let match_list: Vec<_> = match_keys
                    .iter()
                    .map(|(terminal, node)| {
                        let terminal_patten = terminal.as_ref().map_or_else(
                            || quote! {None},
                            |terminal_unwrap| {
                                let ident = &terminal_unwrap.ident;
                                let match_patten = match terminal_unwrap.variant_kind {
                                    VariantKind::None => quote! {#ident},
                                    VariantKind::Truple => quote! {#ident(_)},
                                };
                                quote! {
                                  Some(#token_type::#match_patten)
                                }
                            },
                        );
                        let id = node.borrow().id;
                        quote! {(#id,#terminal_patten)}
                    })
                    .collect();
                let patten = quote! {#(#match_list)|*};
                quote! {
                  #patten=>{
                    #tokens
                  },
                }
            })
            .collect::<Vec<_>>();
        for (node_id, node_cell) in nodes.iter().enumerate() {
            let node = node_cell.try_borrow().unwrap();
            let mut excepts = Vec::new();
            for (terminal, _operation) in &node.action_map {
                excepts.push(terminal.as_ref().map(|t| t.ident.to_string()).unwrap_or_else(|| "no thing".to_string()));
            }
            let except_string = excepts.join("/");
            let node_string = format!("{:?}", &node);
            state_transition.push(quote! {(#node_id,input)=>{
                return Err(::syntax::_format_err!(
                        "except {} ,got {:?},loop:{},states:{:?},node:{}",
                        #except_string,
                        input,
                        loop_count-1,
                        &*state_stack,
                        #node_string,
                    ))
            }});
        }
        let goto_function_list: Vec<_> = goto_case_map
            .iter()
            .map(|(nonterminal_ident, goto_cases)| {
                let goto_function_ident = Ident::new(&format!("goto_{}", nonterminal_ident), nonterminal_ident.span());
                quote! {
                  #[allow(non_snake_case)]
                  #[allow(unused_mut)]
                  #[allow(unused_variables)]
                  #[allow(dead_code)]
                  #[allow(unreachable_code)]
                  fn #goto_function_ident(mut state_stack:&mut Vec<usize>,pop_count:usize)->::syntax::_Fallible<()>{
                    for _ in 0..pop_count{
                      state_stack.pop();
                    }
                    let last=state_stack.last_mut().ok_or_else(||::syntax::_format_err!("wrone state"))?;
                    let new_state = match *last{
                      #(#goto_cases)*
                      _=> { ::syntax::_unreachable!() },
                    };
                    state_stack.push(new_state);
                    Ok(())
                  }
                }
            })
            .collect();
        let mut stacks_init = Vec::new();
        for (symbol, output) in stack_map {
            let ident = &symbol.get_ident();
            let stack_ident = Ident::new(&format!("stack_{}", ident.to_string().to_lowercase()), ident.span());
            let span = ident.span();
            let constructor = output.map_or_else(|| quote! {std::vec::Vec::new()}, |o| quote! {std::vec::Vec::< #o >::new()});
            stacks_init.push(quote_spanned! {span=>
              #[allow(non_snake_case)]
              let mut #stack_ident = #constructor;
            });
        }
        let root_output = &start.output;
        let root_output_unwraped: TokenStream2 = root_output.as_ref().map(|output| quote! {#output}).unwrap_or(quote! {()});
        Ok(quote! {
          #[allow(non_snake_case)]
          #[allow(unused_mut)]
          #[allow(unused_variables)]
          #[allow(dead_code)]
          #[allow(unreachable_code)]
          let mut #name=|tokens:Vec<#token_type>|->::syntax::_Fallible<#root_output_unwraped>{
            #(#goto_function_list)*
            #(#stacks_init)*
            let mut iter=tokens.into_iter().peekable();
            let mut state_stack=Vec::new();
            state_stack.push(0);
            let mut loop_count=0;
            loop{
              loop_count+=1;
              let state=state_stack.last().ok_or_else(||::syntax::_format_err!("wrone state"))?;
              let input=iter.peek();
              match (*state,input){
                #(#state_transition)*
                (o,i)=>{
                        if let Some(input)=i {
                            return Err(::syntax::_format_err!("wrone syntax,loop:{},states:{:?},input:{:?}",loop_count-1,&*state_stack,input))
                        }else{
                            return Err(::syntax::_format_err!("wrone syntax,got none,loop:{},states:{:?}",loop_count-1,&*state_stack))
                        }
                    }
                };
            }
            ::syntax::_unreachable!();
          };
        })
    }
}
struct StateMachineBuilder<'t> {
    productions: &'t ProductionMap,
    start: &'t Rc<NonTerminal>,
    first_set: FirstSet,
    nodes: Vec<Rc<RefCell<Node>>>,
    process_stack: Vec<Rc<RefCell<Node>>>,
    items_map: HashMap<ItemCluster, Rc<RefCell<Node>>, ahash::RandomState>,
    span: Span,
    is_lalr: bool,
}
impl<'t> StateMachineBuilder<'t> {
    fn closure(&self, mut items: ItemCluster) -> ItemCluster {
        let mut tasks = Vec::from_iter(items.iter().map(|(item, outlook)| (item.clone(), outlook.clone())));
        while let Some((item, outlooks)) = tasks.pop() {
            if let Some(symbol) = item.next_symbol() {
                let mut new_items: ItemCluster = BTreeMap::new();
                match symbol {
                    Symbol::NonTerminal(nonterminal) => {
                        let follow_symbols: Vec<_> =
                            item.production.right_part.split_at(item.position + 1).1.iter().map(|(symbol, _value_ident)| symbol.clone()).collect();
                        let mut add_item = |outlook: Option<Rc<Terminal>>| {
                            for production in self.productions.get(&nonterminal).iter().flat_map(|v| v.iter()) {
                                let new_item = LR0Item { production: production.clone(), position: 0 };
                                if items.get_mut(&new_item).map(|i| i.contains(&outlook)) != Some(true) {
                                    new_items.entry(new_item).or_default().insert(outlook.clone());
                                }
                            }
                        };
                        let mut has_end = true;
                        for follow in follow_symbols {
                            let mut has_empty_symbol = false;
                            match follow {
                                Symbol::Terminal(terminal) => {
                                    add_item(Some(terminal));
                                }
                                Symbol::NonTerminal(nonterminal) => {
                                    for option_first in
                                        self.productions.get(&nonterminal).unwrap().iter().flat_map(|production| self.first_set.get(production).unwrap().iter())
                                    {
                                        if let Some(first) = option_first {
                                            add_item(Some(first.clone()));
                                        } else {
                                            has_empty_symbol = true;
                                        }
                                    }
                                }
                            }
                            if !has_empty_symbol {
                                has_end = false;
                                break;
                            }
                        }
                        if has_end {
                            outlooks.iter().for_each(|o| add_item(o.clone()));
                            add_item(None);
                        }
                    }
                    _ => {}
                }
                for (new_item, new_outlooks) in new_items.iter() {
                    let outlooks = items.entry(new_item.clone()).or_default();
                    outlooks.extend(new_outlooks.iter().cloned());
                }
                tasks.extend(new_items.into_iter());
            }
        }
        items
    }

    fn add_node(&mut self, items: ItemCluster, source_items: ItemCluster) -> Result<Rc<RefCell<Node>>> {
        if !self.is_lalr {
            let node = match self.items_map.entry(items) {
                Entry::Vacant(v) => {
                    let items = v.key().clone();
                    let new_node = Node {
                        items,
                        source_items,
                        action_map: HashMap::with_hasher(ahash::RandomState::with_seed(0)),
                        goto_map: HashMap::with_hasher(ahash::RandomState::with_seed(0)),
                        id: self.nodes.len(),
                    };
                    let new_node_wraped = Rc::new(RefCell::new(new_node));
                    v.insert(new_node_wraped.clone());
                    self.process_stack.push(new_node_wraped.clone());
                    self.nodes.push(new_node_wraped.clone());
                    new_node_wraped
                }
                Entry::Occupied(o) => o.get().clone(),
            };
            Ok(node)
        } else {
            let lr_items = items.keys().map(|lr0item| (lr0item.clone(), Default::default())).collect();
            let (node, items) = match self.items_map.entry(lr_items) {
                Entry::Vacant(v) => {
                    let new_node = Node {
                        items,
                        source_items,
                        action_map: HashMap::with_hasher(ahash::RandomState::with_seed(0)),
                        goto_map: HashMap::with_hasher(ahash::RandomState::with_seed(0)),
                        id: self.nodes.len(),
                    };
                    let new_node_wraped = Rc::new(RefCell::new(new_node));
                    v.insert(new_node_wraped.clone());
                    self.process_stack.push(new_node_wraped.clone());
                    self.nodes.push(new_node_wraped.clone());
                    (new_node_wraped, None)
                }
                Entry::Occupied(mut o) => {
                    let node_cell = o.get_mut();
                    (node_cell.clone(), Some(items))
                }
            };
            if let Some(items) = items {
                self.add_outlooks(items, node.clone())?;
            }
            Ok(node)
        }
    }

    fn add_outlooks(&self, new_items: ItemCluster, node_cell: Rc<RefCell<Node>>) -> Result<()> {
        let mut tasks = vec![(new_items, node_cell)];
        while let Some((mut new_items, node_cell)) = tasks.pop() {
            {
                let node = node_cell.borrow();
                new_items.retain(|item, outlooks| {
                    let exist_outlooks = node.items.get(item).unwrap_or_else(|| panic!("None error:{}:{}", file!(), line!()));
                    outlooks.retain(|outlook| !exist_outlooks.contains(outlook));
                    !outlooks.is_empty()
                });
            }
            if new_items.is_empty() {
                continue;
            }
            {
                {
                    let mut node = node_cell.borrow_mut();
                    for (item, outlooks) in new_items.iter() {
                        node.items.get_mut(item).unwrap_or_else(|| panic!("None error:{}:{}", file!(), line!())).extend(outlooks.iter().cloned());
                    }
                }
                let mut new_actions = Vec::new();
                for (item, outlooks) in &node_cell.borrow().items {
                    if item.next_symbol().is_none() {
                        for outlook in outlooks {
                            if item.position == item.production.right_part.len() && &item.production.left_part == self.start {
                                new_actions.push((None, Action::Accept(item.production.clone())));
                            } else {
                                new_actions.push((outlook.clone(), Action::Reduce(item.production.clone())));
                            }
                        }
                    }
                }
                for (terminal, action) in new_actions {
                    self.add_action(terminal, action, node_cell.clone())?;
                }
                for (terminal, action) in node_cell.borrow().action_map.iter() {
                    match action {
                        Action::Shift(shift_node_cell) => {
                            let outlooks: BTreeMap<_, _> = {
                                new_items
                                    .iter()
                                    .filter_map(|(item, outlooks)| {
                                        if matches!(item.clone().next_symbol().as_ref(), Some(Symbol::Terminal(next_terminal)) if Some(next_terminal)==terminal.as_ref()) {
                                            if outlooks.is_empty() {
                                                None
                                            } else {
                                                Some((item.clone().add_position(), outlooks.clone()))
                                            }
                                        } else {
                                            None
                                        }
                                    })
                                    .collect()
                            };
                            tasks.push((self.closure(outlooks), shift_node_cell.clone()));
                        }
                        Action::Reduce(_production) => {}
                        Action::Accept(_) => {}
                    }
                }
                {
                    let node = node_cell.borrow();
                    for (non_terminal, goto_node_cell) in node.goto_map.iter() {
                        let goto_new_items = node
                            .items
                            .iter()
                            .filter_map(|(item, outlooks)| {
                                if matches!(item.next_symbol(),Some(Symbol::NonTerminal(n))if &n==non_terminal) {
                                    Some((item.clone().add_position(), outlooks.clone()))
                                } else {
                                    None
                                }
                            })
                            .collect();
                        tasks.push((self.closure(goto_new_items), goto_node_cell.clone()));
                    }
                }
            }
        }
        Ok(())
    }

    fn add_action(&self, terminal: Option<Rc<Terminal>>, action: Action, node_cell: Rc<RefCell<Node>>) -> Result<()> {
        let mut node = node_cell.borrow_mut();
        if let Some(conflict_action) = node.action_map.insert(terminal.clone(), action.clone()) {
            if action != conflict_action {
                return Err(Error::new(
                    terminal.as_ref().map(|t| t.ident.span()).unwrap_or_else(|| self.span),
                    format!("action conflict,terminal:{:?},action:{:?},conflict action:{:?},node:{:#?}", &terminal, &action, conflict_action, &node),
                ));
            }
        }
        Ok(())
    }

    fn add_goto(&self, nonterminal: Rc<NonTerminal>, goto: Rc<RefCell<Node>>, node_cell: Rc<RefCell<Node>>) -> Result<()> {
        let mut node = node_cell.borrow_mut();
        if let Some(conflict_goto) = node.goto_map.insert(nonterminal.clone(), goto.clone()) {
            if goto != conflict_goto {
                return Err(Error::new(
                    nonterminal.ident.span(),
                    format!("`goto` conflict,nonterminal:{:?},action:{:?},conflict goto:{:?},node:{:?}", &nonterminal, &goto, conflict_goto, &node),
                ));
            }
        }
        Ok(())
    }

    fn process_loop(&mut self) -> Result<()> {
        while let Some(node_cell) = self.process_stack.pop() {
            let mut next_symbols: HashSet<Option<Symbol>, ahash::RandomState> = HashSet::with_hasher(ahash::RandomState::with_seed(0));
            next_symbols.extend(node_cell.borrow().items.keys().map(|item| item.next_symbol()));
            for next_symbol in next_symbols {
                match next_symbol {
                    Some(Symbol::Terminal(terminal)) => {
                        let source_items = BTreeMap::from_iter(
                            node_cell
                                .borrow()
                                .items
                                .iter()
                                .filter(|(item, _outlooks)| item.next_symbol() == Some(Symbol::Terminal(terminal.clone())))
                                .map(|(item, outlooks)| (item.clone().add_position(), outlooks.clone())),
                        );
                        let items = self.closure(source_items.clone());
                        let new_node = self.add_node(items, source_items)?;
                        self.add_action(Some(terminal.clone()), Action::Shift(new_node.clone()), node_cell.clone())?;
                    }
                    Some(Symbol::NonTerminal(nonterminal)) => {
                        let source_items = BTreeMap::from_iter(
                            node_cell
                                .borrow()
                                .items
                                .iter()
                                .filter(|(item, _outlooks)| item.next_symbol() == Some(Symbol::NonTerminal(nonterminal.clone())))
                                .map(|(item, outlooks)| (item.clone().add_position(), outlooks.clone())),
                        );
                        let items = self.closure(source_items.clone());
                        let new_node = self.add_node(items, source_items)?;
                        self.add_goto(nonterminal.clone(), new_node, node_cell.clone())?;
                    }
                    None => {}
                }
            }
            {
                let mut new_actions = Vec::new();
                for (item, outlooks) in &node_cell.borrow().items {
                    if item.next_symbol().is_none() {
                        for outlook in outlooks {
                            if item.position == item.production.right_part.len() && &item.production.left_part == self.start {
                                new_actions.push((None, Action::Accept(item.production.clone())));
                            } else {
                                new_actions.push((outlook.clone(), Action::Reduce(item.production.clone())));
                            }
                        }
                    }
                }
                for (terminal, action) in new_actions {
                    self.add_action(terminal, action, node_cell.clone())?;
                }
            }
        }
        Ok(())
    }

    fn build_state_machine(syntax: &'t Syntax, span: Span, is_lalr: bool) -> Result<Vec<Rc<RefCell<Node>>>> {
        let Syntax { productions, start, token_type: _token_type } = syntax;
        let first_set = first_set(productions);
        let process_stack = Vec::new();
        let nodes = Vec::new();
        let items_map = HashMap::with_hasher(ahash::RandomState::with_seed(0));
        let mut this = Self { productions, start, first_set, nodes, process_stack, items_map, is_lalr, span };
        let source_items = ItemCluster::from_iter(
            productions.get(start).unwrap().iter().map(|production| (LR0Item { production: production.clone(), position: 0 }, BTreeSet::from_iter([None]))),
        );
        let items = this.closure(source_items.clone());
        let start_node = Node {
            items,
            source_items,
            action_map: HashMap::with_hasher(ahash::RandomState::with_seed(0)),
            goto_map: HashMap::with_hasher(ahash::RandomState::with_seed(0)),
            id: 0,
        };
        let start_node_wrap = Rc::new(RefCell::new(start_node));
        this.nodes.push(start_node_wrap.clone());
        this.process_stack.push(start_node_wrap);
        this.process_loop()?;
        Ok(this.nodes)
    }
}
pub(crate) fn do_generate_parser(syntax_declaration: SyntaxDeclaration, is_lalr: bool) -> Result<TokenStream2> {
    let name = syntax_declaration.ident.clone();
    let span = name.span();
    let syntax = parse_syntax_declaration(syntax_declaration)?;
    let nodes = StateMachineBuilder::build_state_machine(&syntax, span, is_lalr)?;
    let syntax_lr1 = SyntaxLR1 { syntax, nodes };
    syntax_lr1.generate(name)
}
