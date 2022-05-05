use super::builder::*;
use super::ir::LuaInstructionSet;
use super::lua_lexical::LuaLexical;
use crate::mem::LuaState;
use failure::Fallible;
use ghost_cell::GhostToken;
use lexical_derive::token as t;
use runtime::code::FunctionPack;
use std::{collections::HashMap, rc::Rc};
use syntax_derive::{lalr1_analyser, lr1_analyser, recursive_predictive_analysis};
pub fn parse(source: Vec<LuaLexical>) -> Fallible<Vec<FunctionPack<LuaInstructionSet>>> {
    use super::{builder::*, ir::*};
    GhostToken::new(|token| {
        let mut ctx = new_ctx(token);
        macro_rules! const_value {
            ($Instruction:ident) => {
                ctx.emit_const_value($Instruction::emit)
            };
        }
        macro_rules! unique_integer_operate {
            ($IntInstruction:ident, $expr1:ident) => {
                concat_idents::concat_idents!(
                    i_instruction = I,$IntInstruction {
                        ctx.unique_operate( $expr1, Some(i_instruction::emit), None, $IntInstruction::emit,)})
            };
        }
        macro_rules! binary_integer_operate {
            ($IntInstruction:ident, $expr1:ident, $expr2:ident) => {
                concat_idents::concat_idents!(
                    i_instruction = I,$IntInstruction {
                        ctx.binary_operate( $expr1, $expr2, Some(i_instruction::emit), None, $IntInstruction::emit,)})
            };
        }
        macro_rules! binary_float_operate {
            ($IntInstruction:ident, $expr1:ident, $expr2:ident) => {
                concat_idents::concat_idents!(
                    f_instruction = F,$IntInstruction {
                        ctx.binary_operate( $expr1, $expr2, None, Some(f_instruction::emit), $IntInstruction::emit,)})
            };
        }
        macro_rules! unique_value_operate {
            ($Instruction:ident, $expr1:ident) => {
                ctx.unique_operate($expr1,None,None, $Instruction::emit)
            };
        }
        macro_rules! unique_operate {
            ($Instruction:ident, $expr1:ident) => {
                ctx.unique_operate( $expr1, Some(<concat_idents!(I, $Instruction)>::emit), Some(<concat_idents!(F, $Instruction)>::emit), $Instruction::emit,)
            };
        }
        macro_rules! binary_operate {
            ($Instruction:ident, $expr1:ident, $expr2:ident) => {
                ctx.binary_operate( $expr1, $expr2, Some(<concat_idents!(I, $Instruction)>::emit), Some(<concat_idents!(F, $Instruction)>::emit), $Instruction::emit,)
            };
        }
        macro_rules! binary_to_bool_operate {
            ($Instruction:ident, $expr1:ident, $expr2:ident) => {
                ctx.binary_to_bool_operate( $expr1, $expr2, Some(<concat_idents!(I, $Instruction)>::emit), Some(<concat_idents!(F, $Instruction)>::emit), $Instruction::emit,)
            };
        }
        lalr1_analyser! {
          lua_parser:LuaLexical->(){
            chunk=>()->{
              [stat_list,return_expr(r)]=>ctx.emit_return(r);
                | [stat_list]=>ctx.emit_return(None);
            },
            block=>(LuaBlockRef<'_>,LuaBlockRef<'_>)->{ [scopt_begin(b),block_inner(i)]=>Ok(i); },
            loop_block=>(LuaBlockRef<'_>,LuaBlockRef<'_>)->{ [loop_scopt_begin(b),block_inner(i)]=>Ok(i); },
            block_inner=>(LuaBlockRef<'_>,LuaBlockRef<'_>)->{
              [stat_list,return_expr(r),block_split(s)]=>ctx.return_(r,s);
                | [stat_list,block_split(s)]=>Ok(s);
            },
            stat_list=>()->{ []|[stat,stat_list] },
            block_split=>(LuaBlockRef<'_>,LuaBlockRef<'_>)->{ []=>Ok((ctx.current_block.clone(),ctx.new_block().clone())); },
            current_block=>LuaBlockRef<'_>->{ []=>Ok(ctx.current_block.clone()); },
            scopt_begin=>LuaScoptRef<'_>->{ []=>Ok(ctx.new_scopt(ScoptKind::Other).clone()); },
            loop_scopt_begin=>LuaScoptRef<'_>->{ []=>Ok(ctx.new_scopt(ScoptKind::Loop{break_block:ctx.current_block.clone()}).clone()); },
            stat->{
                [t!(;)]=>Ok(());
              | [stat_var_list(v),t!(=),expr_list(exprs)]=>ctx.put_values(v,exprs);
              | [stat_prefix_expr(e),args(a)]=>Ok({ctx.stat_call(e,a)?;});
              | [stat_prefix_expr(e),t!(:),Name(n),args(a)]=>Ok({ctx.stat_call_self(e,n,a)?;});
              | [t!(::),Name(n),t!(::)]=>ctx.define_label(n);
              | [t!(break)]=>ctx.break_();
              | [t!(goto),Name(n)]=>ctx.goto(n);
              | [if_prefix(p),block_split(b),t!(else),block(a),t!(end)]=>ctx.else_(p,b,a);
              | [if_prefix(p),t!(end),block_split(b)]=>ctx.end_if(p,b);
              | [t!(function),Name(n),function_boby(f)]=>ctx.set_function(n,f);
              | [t!(local),t!(function),Name(n),function_boby(f)]=>ctx.local_function(n,f);
              | [t!(local),att_name_list(a)]=>ctx.local_variable(a);
              | [t!(local),att_name_list(a),t!(=),expr_list(e)]=>ctx.local_variable_with_values(a,e);
              | [t!(do),loop_block(b),t!(end)]=>ctx.finish_block(b);
              | [t!(while),expr_wraped(e),block_split(s),t!(do),loop_block(b),t!(end)]=>ctx.while_(e,s,b);
              | [t!(repeat),block_split(s),loop_block(b),t!(until),expr_wraped(e)]=>ctx.repeat(s,b,e);
              | [t!(for),for_head(h),block_inner(b),t!(end)]=>ctx.for_(h,b);
              | [t!(for),for_each_head(h),block_inner(b),t!(end)]=>ctx.for_step(h,b);
              | [t!(for),for_in_head(h),block_inner(b),t!(end)]=>ctx.for_in(h,b);
            },
            for_head->{ [Name(n),t!(=),expr(e),t!(,),expr(e1),t!(do),block_split(p),block_split(p1),loop_scopt_begin]=>ctx.for_head(n,e,e1,p,p1); },
            for_each_head->{ [Name(n),t!(=),expr(e),t!(,),expr(e1),t!(,),expr(e2),t!(do),block_split(p),block_split(p1),loop_scopt_begin]=>ctx.for_step_head(n,e,e1,e2,p,p1); },
            for_in_head->{ [name_list(n),t!(in),expr_list(e),t!(do),block_split(p),block_split(p1),loop_scopt_begin]=>ctx.for_in_head(n,e,p,p1); },
            expr_wraped=>((LuaBlockRef<'_>,LuaBlockRef<'_>),LuaExprRef)->{ [block_split(b),expr(e)]=>Ok((b,e)); },
            if_prefix=>(LuaBlockRef<'_>,LuaBlockRef<'_>)->{
              [t!(if),expr(e),block_split(b),t!(then),block(c)]=>ctx.if_(e,b,c);
                | [if_prefix(p),t!(elseif),expr(e),block_split(c),t!(then),block(b),block_split(n)]=>ctx.elseif(p,e,c,b,n);
            },
            att_name_list=>Vec<(std::string::String,VarAttribute)>->{
              [Name(n),attrib(a)]=>Ok(vec![(n,a)]);
              | [att_name_list(mut l),t!(,),Name(n),attrib(a)]=>Ok({l.push((n,a));l});
            },
            attrib=>VarAttribute->{
              []=>Ok(Default::default());
                | [t!(<),Name(n),t!(>)]=>VarAttribute::new(n);
            },
            return_expr=>Option<LuaExprList<'_>>->{
              [t!(return)]=>Ok(None);
                | [t!(return),t!(;)]=>Ok(None);
                | [t!(return),expr_list(e)]=>Ok(Some(e));
                | [t!(return),expr_list(e),t!(;)]=>Ok(Some(e));
            },
            function_name=>(Vec<std::string::String>,Option<std::string::String>)->{
              [name_path(n)]=>Ok((n,None));
                | [name_path(p),t!(:),Name(n)]=>Ok((p,Some(n)));
            },
            name_path=>Vec<std::string::String>->{
              [Name(n)]=>Ok(vec![n]);
                | [name_path(mut l),t!(.),Name(n)]=>Ok({l.push(n);l});
            },
            name_list=>Vec<String>->{
              [Name(n)]=>Ok(vec![n]);
                | [name_list(mut l),t!(,),Name(n)]=>Ok({l.push(n);l});
            },
            expr_list=>LuaExprList<'_>->{
              [expr(e)]=>Ok(LuaExprList::from(e));
              | [expr_list(l),t!(,),expr(e)]=>ctx.extend_expr_list(l,e);
            },
            expr=>LuaExprRef->{
              [expr1(e)]=>Ok(e); |[expr2(e)]=>Ok(e); |[expr3(e)]=>Ok(e);
              |[expr4(e)]=>Ok(e); |[expr5(e)]=>Ok(e); |[expr6(e)]=>Ok(e);
              |[expr7(e)]=>Ok(e); |[expr8(e)]=>Ok(e); |[expr9(e)]=>Ok(e);
              |[expr10(e)]=>Ok(e); |[expr11(e)]=>Ok(e);
              |[expr12(e)]=>Ok(e);
              |[expr_highest(e)]=>Ok(e);
            },
            expr_high_1=>LuaExprRef->{
              [expr2(e)]=>Ok(e); |[expr3(e)]=>Ok(e); |[expr4(e)]=>Ok(e);
                  |[expr5(e)]=>Ok(e); |[expr6(e)]=>Ok(e); |[expr7(e)]=>Ok(e);
                  |[expr8(e)]=>Ok(e); |[expr9(e)]=>Ok(e); |[expr10(e)]=>Ok(e);
                  |[expr11(e)]=>Ok(e); |[expr12(e)]=>Ok(e); |[expr_highest(e)]=>Ok(e);
            },
            expr_high_2=>LuaExprRef->{
              [expr3(e)]=>Ok(e); |[expr4(e)]=>Ok(e); |[expr5(e)]=>Ok(e);
                  |[expr6(e)]=>Ok(e); |[expr7(e)]=>Ok(e); |[expr8(e)]=>Ok(e);
                  |[expr9(e)]=>Ok(e); |[expr10(e)]=>Ok(e); |[expr11(e)]=>Ok(e);
                  |[expr12(e)]=>Ok(e); |[expr_highest(e)]=>Ok(e);
            },
            expr_high_3=>LuaExprRef->{
              [expr4(e)]=>Ok(e); |[expr5(e)]=>Ok(e); |[expr6(e)]=>Ok(e);
                  |[expr7(e)]=>Ok(e); |[expr8(e)]=>Ok(e); |[expr9(e)]=>Ok(e);
                  |[expr10(e)]=>Ok(e); |[expr11(e)]=>Ok(e); | [expr12(e)]=>Ok(e);
                  |[expr_highest(e)]=>Ok(e);
            },
            expr_high_4=>LuaExprRef->{
              [expr5(e)]=>Ok(e); |[expr6(e)]=>Ok(e); |[expr7(e)]=>Ok(e);
                  |[expr8(e)]=>Ok(e); |[expr9(e)]=>Ok(e); |[expr10(e)]=>Ok(e);
                  |[expr11(e)]=>Ok(e); | [expr12(e)]=>Ok(e); |[expr_highest(e)]=>Ok(e);
            },
            expr_high_5=>LuaExprRef->{
              [expr6(e)]=>Ok(e); |[expr7(e)]=>Ok(e); |[expr8(e)]=>Ok(e);
                  |[expr9(e)]=>Ok(e); |[expr10(e)]=>Ok(e); |[expr11(e)]=>Ok(e);
              | [expr12(e)]=>Ok(e); |[expr_highest(e)]=>Ok(e);
            },
            expr_high_6=>LuaExprRef->{
              [expr7(e)]=>Ok(e); |[expr8(e)]=>Ok(e); |[expr9(e)]=>Ok(e);
                  |[expr10(e)]=>Ok(e); |[expr11(e)]=>Ok(e); | [expr12(e)]=>Ok(e);
                  |[expr_highest(e)]=>Ok(e);
            },
            expr_high_7=>LuaExprRef->{
              [expr8(e)]=>Ok(e); |[expr9(e)]=>Ok(e); |[expr10(e)]=>Ok(e);
                  |[expr11(e)]=>Ok(e); | [expr12(e)]=>Ok(e); |[expr_highest(e)]=>Ok(e);
            },
            expr_high_8=>LuaExprRef->{
              [expr9(e)]=>Ok(e); |[expr10(e)]=>Ok(e); |[expr11(e)]=>Ok(e);
              | [expr12(e)]=>Ok(e); |[expr_highest(e)]=>Ok(e);
            },
            expr_high_9=>LuaExprRef->{
              [expr10(e)]=>Ok(e); |[expr11(e)]=>Ok(e); | [expr12(e)]=>Ok(e);
                  |[expr_highest(e)]=>Ok(e);
            },
            expr_high_10=>LuaExprRef->{
              [expr11(e)]=>Ok(e); | [expr12(e)]=>Ok(e); |[expr_highest(e)]=>Ok(e);
            },
            expr_high_11=>LuaExprRef->{ [expr12(e)]=>Ok(e); |[expr_highest(e)]=>Ok(e); },
            expr1=>LuaExprRef->{ [expr(v),t!(or),block_split(b1),expr_high_1(v1),block_split(b2)]=>ctx.or(v,b1,v1,b2); },
            expr2=>LuaExprRef->{ [expr_high_1(v),t!(and),block_split(b1),expr_high_2(v1),block_split(b2)]=>ctx.and(v,b1,v1,b2); },
            expr3=>LuaExprRef->{
              [expr_high_2(v),t!(<),expr_high_3(v1)]=>binary_to_bool_operate!(Less,v,v1);
              |[expr_high_2(v),t!(>),expr_high_3(v1)]=>binary_to_bool_operate!(Large,v,v1);
              |[expr_high_2(v),t!(<=),expr_high_3(v1)]=>binary_to_bool_operate!(LessOrEqual,v,v1);
              |[expr_high_2(v),t!(>=),expr_high_3(v1)]=>binary_to_bool_operate!(LargeOrEqual,v,v1);
              |[expr_high_2(v),NotEqual,expr_high_3(v1)]=>binary_to_bool_operate!(NotEqual,v,v1);
              |[expr_high_2(v),t!(==),expr_high_3(v1)]=>binary_to_bool_operate!(Equal,v,v1);
            },
            expr4=>LuaExprRef->{ [expr_high_3(v),t!(|),expr_high_4(v1)]=>binary_integer_operate!(BitOr,v,v1); },
            expr5=>LuaExprRef->{ [expr_high_4(v),t!(~),expr_high_5(v1)]=>binary_integer_operate!(BitXor,v,v1); },
            expr6=>LuaExprRef->{ [expr_high_5(v),t!(&),expr_high_6(v1)]=>binary_integer_operate!(BitAnd,v,v1); },
            expr7=>LuaExprRef->{
              [expr_high_6(v),t!(<<),expr_high_7(v1)]=>binary_integer_operate!(LeftShift,v,v1);
              | [expr_high_6(v),t!(>>),expr_high_7(v1)]=>binary_integer_operate!(RightShift,v,v1);
            },
            expr8=>LuaExprRef->{ [expr_high_7(v),t!(..),expr_high_8(v1)]=>ctx.concat(v,v1); },
            expr9=>LuaExprRef->{
              [expr_high_8(v),t!(+),expr_high_9(v1)]=>binary_operate!(Add,v,v1);
              | [expr_high_8(v),t!(-),expr_high_9(v1)]=>binary_operate!(Sub,v,v1);
            },
            expr10=>LuaExprRef->{
              [expr_high_9(v),t!(*),expr_high_10(v1)]=>binary_operate!(Mul,v,v1);
              |[expr_high_9(v),t!(/),expr_high_10(v1)]=>binary_float_operate!(Div,v,v1);
              |[expr_high_9(v),DoubleSlash,expr_high_10(v1)]=>binary_operate!(DivFloor,v,v1);
              |[expr_high_9(v),t!(%),expr_high_10(v1)]=>binary_operate!(Rem,v,v1);
            },
            expr11=>LuaExprRef->{
              [t!(not),expr_high_11(v)]=>unique_integer_operate!(BitNot,v);
              |[t!(#),expr_high_11(v)]=>unique_value_operate!(Length,v);
              |[t!(-),expr_high_11(v)]=>unique_operate!(Neg,v);
              |[t!(~),expr_high_11(v)]=>unique_integer_operate!(BitNot,v);
            },
            expr12=>LuaExprRef->{
              [expr_high_11(v),t!(^),expr_highest(v1)]=>binary_float_operate!(Pow,v,v1);
            },
            expr_highest=>LuaExprRef->{
              [t!(nil)]=>const_value!(ConstNil);
                | [t!(false)]=>const_value!(ConstFalse);
                  | [t!(true)]=>const_value!(ConstTrue);
                  | [Number(n)]=>ctx.const_number(n);
                  | [String(s)]=>ctx.const_string(s);
                  | [t!(...)]=>ctx.const_va_arg0();
                  | [function_def(f)]=>ctx.const_function(f);
                  | [prefix_expr(p)]=>Ok(p);
                  | [table_constructor(t)]=>ctx.const_table(t);
            },
            prefix_expr=>LuaExprRef->{
              [var(v)]=>ctx.load_var(v);
                | [function_call(c)]=>ctx.get_from_slice(0,c);
                  | [LeftParen,expr(e),RightParen]=>Ok(e);
            },
            stat_prefix_expr=>LuaExprRef->{
              [stat_var(v)]=>ctx.load_var(v);
                | [stat_prefix_expr(e),args(a)]=>ctx.stat_call(e,a);
                | [stat_prefix_expr(e),t!(:),Name(n),args(a)]=>ctx.stat_call_self(e,n,a);
                | [t!(;),LeftParen,expr(e),RightParen]=>Ok(e);
            },
            stat_var_list=>Vec<LuaVar>->{
              [stat_var(v)]=>Ok(vec![v]);
                | [stat_var_list(mut l),t!(,),var(v)]=>Ok({l.push(v);l});
            },
            stat_var=>LuaVar->{
              [Name(n)]=>Ok(LuaVar::Variable(n));
                | [stat_prefix_expr(e),LeftBracket,expr(i),RightBracket]=>Ok(LuaVar::Element(e,i));
                  | [stat_prefix_expr(e),t!(.),Name(n)]=>Ok(LuaVar::Field(e,n));
            },
            var_list=>Vec<LuaVar>->{
              [var(v)]=>Ok(vec![v]);
                | [var_list(mut l),t!(,),var(v)]=>Ok({l.push(v);l});
            },
            var=>LuaVar->{
              [Name(n)]=>Ok(LuaVar::Variable(n));
                | [prefix_expr(e),LeftBracket,expr(i),RightBracket]=>Ok(LuaVar::Element(e,i));
                  | [prefix_expr(e),t!(.),Name(n)]=>Ok(LuaVar::Field(e,n));
            },
            function_call=>LuaExprList->{
              [prefix_expr(e),args(a)]=>ctx.call(e,a);
                | [prefix_expr(e),t!(:),Name(n),args(a)]=>ctx.call_self(e,n,a);
            },
            args=>LuaExprList<'_>->{
              [LeftParen,RightParen]=>Ok(LuaExprList::new());
              | [LeftParen,expr_list(e),RightParen]=>Ok(e);
                | [table_constructor(t)]=>ctx.const_table(t).map(|t|t.into());
                  | [String(s)]=>ctx.const_string(s).map(|t|t.into());
            },
            function_def=>LuaFunctionBuilderRef->{[t!(function),function_boby(f)]=>Ok(f);},
            function_boby=>LuaFunctionBuilderRef->{[LeftParen,param_list(p),RightParen,block(b),t!(end)]=>ctx.finish_function(b);},
            param_list=>LuaFunctionBuilderRef<'_>->{
              [t!(...)]=>ctx.define_parameters(vec![],false);
                | [name_list(n)]=>ctx.define_parameters(n,false);
                  | [name_list(n),t!(,),t!(...)]=>ctx.define_parameters(n,true);
            },
            table_constructor=>Vec<(LuaTableKey,LuaExprRef)>->{
                [LeftBrace,RightBrace]=>Ok(vec![]);
                | [LeftBrace,field_list(f),RightBrace]=>Ok(f);
                | [LeftBrace,field_list(f),field_sep,RightBrace]=>Ok(f);
            },
            field_list=>Vec<(LuaTableKey,LuaExprRef)>->{
              [field(f)]=>Ok(vec![f]); | [field_list(mut l),field_sep,field(f)]=>Ok({l.push(f);l});
            },
            field_sep=>()->{[t!(,)]|[t!(;)]},
            field=>(LuaTableKey,LuaExprRef)->{
              [LeftBracket,expr(e),RightBracket,t!(=),expr(v)]=>Ok((LuaTableKey::Expr(e),v));
                | [Name(n),t!(=),expr(v)]=>Ok((LuaTableKey::String(n),v));
                  | [expr(e)]=>Ok((LuaTableKey::None,e));
            }
          }
        }
        lua_parser(source)?;
        ctx.pack()
    })
}
