use std::{io::{stdin, Write}, str::Chars, sync::Arc};

use chinese_number::{ChineseNumber, ChineseNumberCountMethod, ChineseVariant};
use failure::Fallible;
use ghost_cell::GhostToken;
use lexical::Lexical;
use lexical_derive::{lexical, Lexical};
use log::{debug, error};
use runtime::code::{BlockBuilder, BuddyRegisterPool, FunctionBuilder, FunctionPack, RegisterPool};
use runtime_extra as e;
use syntax_derive::lalr1_analyser;
use vm_core::{Direct, DynRuntimeTrait, ExecutableResourceTrait, MemoryTrait, ObjectRef, Pointer, ResourceFactory, RuntimeTrait, TypeDeclaration, UnsizedArray};
use vm_lua::{add_global_function, binary_operate_type, builder::{LuaBlockRef, LuaContext, LuaExprList, LuaExprListBuilder, LuaExprRef, LuaFunctionBuilder, LuaScoptRef, ScoptKind}, built_in::empty_return, instruction::{extend_to_buffer, ConstFalse}, ir::{Add, ConstTrue, Div, Equal, FAdd, FDiv, FEqual, FLarge, FLargeOrEqual, FLess, FLessOrEqual, FMul, FNotEqual, FRem, FSub, IAdd, IEqual, ILarge, ILargeOrEqual, ILess, ILessOrEqual, IMul, INotEqual, IRem, ISub, Large, LargeOrEqual, Less, LessOrEqual, LuaInstructionSet, Mul, NotEqual, Rem, Sub}, lua_lexical::LuaNumberLit, mem::{LuaFunctionRustType, LuaStateReference, LuaValue, LuaValueImpl}, new_state};
type Register<T> = runtime::code::Register<T, BuddyRegisterPool>;

pub type 表达式<'l> = LuaExprRef<'l>;
pub type 表达式列表<'l> = Vec<LuaExprRef<'l>>;
pub type 变量名 = String;
pub type 字符串 = String;
pub type 数值 = LuaNumberLit;
pub type 变量名列表 = Vec<String>;
pub type 基本块<'l> = LuaBlockRef<'l>;
pub type 作用域<'l> = LuaScoptRef<'l>;
pub type 虚拟机 = LuaStateReference;
pub type 程序 = Vec<FunctionPack<LuaInstructionSet>>;
pub type 运行时 = Arc<dyn DynRuntimeTrait<FunctionPack<LuaInstructionSet>>>;
#[lexical([],[
"有","名之","以","其","所餘幾何","昔之","者","今","是矣","曰","恆","之","吾有","其物如是","物之","是謂","之物也","矣","批曰","也","注曰","是術曰","欲行是術","必先得","之術也","疏曰",  "數","言","爻","列","物","術",  "陽","陰",  "若","若非","乃止","中有陽乎","中無陰乎","乃歸空無","乃得","為是","遍","凡","中之","云云",  "大於","不大於","小於","不小於","等於","不等於","加","减","乘","除","減","夫","銜","長","其餘","書之",
])]
#[derive(Debug, Clone, PartialEq)]
pub enum 文言词法 {
    #[lexical(string = "。")]
    句号,
    #[lexical(fn = "解析字符串")]
    一言(字符串),
    #[lexical(fn = "解析变量名")]
    变量(变量名),
    #[lexical(fn = "解析数值")]
    数(数值),
}
pub fn 解析变量名(iter: &mut Chars) -> Option<String> {
    if iter.next() != Some('「') {
        return None;
    };
    let mut name = String::new();
    while let Some(c) = iter.next() {
        if c != '」' {
            name.push(c);
        } else {
            return Some(name);
        }
    }
    None
}
pub fn 解析字符串(iter: &mut Chars) -> Option<String> {
    if iter.next() != Some('「') {
        return None;
    };
    if iter.next() != Some('「') {
        return None;
    };
    let mut name = String::new();
    while let Some(c) = iter.next() {
        if c != '」' {
            name.push(c);
        } else {
            let c = iter.next()?;
            if c == '」' {
                return Some(name);
            } else {
                name.push('」');
            }
        }
    }
    None
}
pub fn 解析数值(iter: &mut Chars) -> Option<数值> {
    let mut string = String::new();
    while let Some(c) = iter.clone().next() {
        match c {
            '一' | '二' | '三' | '四' | '五' | '六' | '七' | '八' | '九' | '零' | '负' | '点' | '十' | '百' | '千'
            | '万' | '兆' | '亿' => {
                string.push(c);
                iter.next();
            }
            _ => {
                break;
            }
        }
    }
    if let Ok(int) = chinese_number::parse_chinese_number_to_i64(ChineseNumberCountMethod::TenThousand, &string) {
        Some(LuaNumberLit::Integer(int))
    } else if let Ok(float) =
        chinese_number::parse_chinese_number_to_f64(ChineseNumberCountMethod::TenThousand, &string)
    {
        Some(LuaNumberLit::Float(float))
    } else {
        None
    }
}
pub enum 二元运算 {
    加,
    减,
    乘,
    除以,
    余,
}
pub enum 比较 {
    大于,
    小于,
    不大于,
    不小于,
    等于,
    不等于,
}
pub fn 解析语法(vm: 虚拟机, 源代码: Vec<文言词法>) -> Fallible<程序> {
    GhostToken::new(|token| {
        let mut 代码 = 中间码构建器::创建(vm, token);
        lalr1_analyser! {
            解析文言语法:文言词法->(){
                文言=>()->{[语句列表]=>代码.返回(None);},
                分块=>(基本块,基本块)->{[]=>代码.分块();},
                语句列表=>()->{[]|[语句,语句列表]},
                新作用域=>作用域->{[]=>代码.新作用域();},
                新循环作用域=>作用域->{[]=>代码.新循环作用域();},
                块=>(基本块,基本块)->{[新作用域(_),语句列表]=>代码.作用域结束();},
                循环块=>(基本块,基本块)->{[新循环作用域(_),语句列表]=>代码.作用域结束();},
                循环块尾=>(基本块,基本块)->{[语句列表]=>代码.作用域结束();},
                语句=>()->{
                    [昔之,变量(甲),者,句号,今,变量(乙),是矣,句号]=>代码.复制变量(甲,乙);
                    | [表达式列表(甲),名之,变量名列表(乙),句号]=>代码.赋值(甲,乙);
                    | [表达式(甲),書之,句号]=>代码.书之(甲);
                    | [若,判断(甲),分块(乙),句号,块(丙),也,句号]=>代码.分支(甲,乙,丙,None);
                    | [若,判断(甲),分块(乙),句号,块(丙),若非,句号,块(丁),也,句号]=>代码.分支(甲,乙,丙,Some(丁));
                    | [循环头(甲),循环块尾(丁),云云,句号]=>代码.循环(甲,丁);
                    | [恆,循环准备,句号,分块(乙),循环块(丙),云云,句号]=>代码.恒循环(乙,丙);
                    | [乃歸空無]=>代码.返回(None);
                    | [乃得,操作数(甲)]=>代码.返回操作数(甲);
                    | [表达式(甲),乃得,矣]=>代码.返回操作数(甲);
                    | [乃止,句号]=>代码.跳出();
                    | [批曰,句号,一言(_),句号]=>Ok(()); | [注曰,句号,一言(_),句号]=>Ok(()); | [疏曰,句号,一言(_),句号]=>Ok(());
                },
                循环头->{[循环准备,操作数(甲),遍,句号,分块(乙),分块(丙),新循环作用域]=>代码.循环头(甲,乙,丙);},
                循环准备=>()->{ [為是]=>代码.循环准备(); },
                表达式列表=>表达式列表->{
                    [夫,操作数(甲),句号]=>Ok(vec![甲]);
                    | [表达式(甲)]=>Ok(vec![甲]);
                    | [表达式列表(mut 乙),夫,操作数(甲),句号]=>Ok({乙.push(甲);乙});
                },
                表达式=>表达式->{
                    [有,类型(_),常量(甲),句号]=>Ok(甲);
                    | [吾有,数(甲),类型(乙),句号,常量列表(丙)]=>Ok(丙[0].clone());
                    | [表达式(甲),运算(丙),其,以,操作数(乙),句号]=>代码.二元运算(甲,乙,丙);
                    | [表达式(甲),除,其,以,操作数(乙),句号]=>代码.二元运算(甲,乙,二元运算::除以);
                    | [表达式(甲),除,其,以,操作数(乙),句号,所餘幾何,句号]=>代码.二元运算(甲,乙,二元运算::余);
                    | [运算(丙),操作数(甲),以,操作数(乙),句号]=>代码.二元运算(甲,乙,丙);
                    | [除,操作数(甲),以,操作数(乙),句号]=>代码.二元运算(甲,乙,二元运算::除以);
                    | [除,操作数(甲),以,操作数(乙),所餘幾何,句号]=>代码.二元运算(甲,乙,二元运算::余);
                    | [夫,操作数(甲),分块(丙),操作数(乙),分块(丁),中有陽乎,句号]=>代码.与(甲,丙,乙,丁);
                    | [夫,操作数(甲),分块(丙),操作数(乙),分块(丁),中無陰乎,句号]=>代码.或(甲,丙,乙,丁);
                },
                判断=>表达式->{
                    [操作数(甲)]=>Ok(甲);
                    | [操作数(甲),比较(丙),操作数(乙),者]=>代码.比较(甲,乙,丙);
                },
                常量=>表达式->{
                    [数(甲)]=>代码.常量数字(甲);
                    | [一言(甲)]=>代码.常量字符串(甲);
                    | [陽]=>代码.常量布尔值(true);
                    | [陰]=>代码.常量布尔值(false);
                },
                运算=>二元运算->{ [加]=>Ok(二元运算::加); |[减]=>Ok(二元运算::减); |[乘]=>Ok(二元运算::乘); },
                比较=>比较->{[大於]=>Ok(比较::大于);|[不大於]=>Ok(比较::不大于);|[小於]=>Ok(比较::小于);|[不小於]=>Ok(比较::不小于);|[等於]=>Ok(比较::等于);|[不等於]=>Ok(比较::不等于);},
                类型=>()->{ [數] |[言] |[爻] |[列] |[物] |[術] },
                操作数=>表达式->{
                    [变量(甲)]=>代码.获取变量(甲);
                    | [常量(甲)]=>Ok(甲);
                },
                变量名列表=>变量名列表->{ [曰,变量(甲)]=>Ok(vec![甲]); |[变量名列表(mut 乙),曰,变量(甲)]=>Ok({乙.push(甲);乙}); },
                常量列表=>表达式列表->{ [曰,常量(甲),句号]=>Ok(vec![甲]); |[常量列表(mut 乙),曰,常量(甲),句号]=>Ok({乙.push(甲);乙}); },
            }
        }
        解析文言语法(源代码)?;
        代码.打包()
    })
}
pub struct 中间码构建器<'l> {
    raw: LuaContext<'l>,
}
impl<'l> 中间码构建器<'l> {
    pub(crate) fn 创建(vm: 虚拟机, token: GhostToken<'l>) -> Self {
        Self {
            raw: LuaContext::new(token, vm),
        }
    }
    pub fn 新作用域(&mut self) -> Fallible<作用域<'l>> { self.raw.new_scopt(vm_lua::builder::ScoptKind::Other) }
    pub fn 新循环作用域(&mut self) -> Fallible<作用域<'l>> {
        self.raw.new_scopt(ScoptKind::Loop {
            break_block: self.raw.current_block.clone(),
        })
    }
    pub fn 作用域结束(&mut self) -> Fallible<(基本块<'l>, 基本块<'l>)> {
        let s = self.raw.split_block()?;
        self.raw.finish_scopt(s)
    }
    pub fn 获取变量(&mut self, name: String) -> Fallible<表达式<'l>> { self.raw.get_value(name) }
    pub fn 复制变量(&mut self, from: String, to: String) -> Fallible<()> {
        let var = self.raw.get_value(from)?;
        self.raw.put_value(to, var)?;
        Ok(())
    }
    pub fn 赋值(&mut self, from: 表达式列表<'l>, to: 变量名列表) -> Fallible<()> {
        let len = from.len().min(to.len());
        for (expr, var) in from.into_iter().take(len).zip(to.into_iter().take(len)) {
            self.raw.add_local(var, Default::default(), expr)?;
        }
        Ok(())
    }
    pub fn 常量数字(&mut self, lit: 数值) -> Fallible<表达式<'l>> { self.raw.const_number(lit) }
    pub fn 常量字符串(&mut self, string: String) -> Fallible<表达式<'l>> { self.raw.const_string(string) }
    pub fn 常量布尔值(&mut self, b: bool) -> Fallible<表达式<'l>> {
        self.raw
            .emit_const_value(if b { ConstTrue::emit } else { ConstFalse::emit })
    }
    pub fn 与(
        &mut self,
        a: 表达式<'l>,
        b1: (基本块<'l>, 基本块<'l>),
        b: 表达式<'l>,
        b2: (基本块<'l>, 基本块<'l>),
    ) -> Fallible<表达式<'l>> {
        self.raw.and(a, b1, b, b2)
    }
    pub fn 或(
        &mut self,
        a: 表达式<'l>,
        b1: (基本块<'l>, 基本块<'l>),
        b: 表达式<'l>,
        b2: (基本块<'l>, 基本块<'l>),
    ) -> Fallible<表达式<'l>> {
        self.raw.or(a, b1, b, b2)
    }
    pub fn 二元运算(&mut self, a: 表达式<'l>, b: 表达式<'l>, o: 二元运算) -> Fallible<表达式<'l>> {
        let (emit_int, emit_float, emit_value): (
            Option<binary_operate_type!(e::I64)>,
            binary_operate_type!(e::F64),
            binary_operate_type!(LuaValue),
        ) = match o {
            二元运算::加 => (Some(IAdd::emit), FAdd::emit, Add::emit),
            二元运算::减 => (Some(ISub::emit), FSub::emit, Sub::emit),
            二元运算::乘 => (Some(IMul::emit), FMul::emit, Mul::emit),
            二元运算::除以 => (None, FDiv::emit, Div::emit),
            二元运算::余 => (Some(IRem::emit), FRem::emit, Rem::emit),
        };
        self.raw.binary_operate(a, b, emit_int, Some(emit_float), emit_value)
    }
    pub fn 比较(&mut self, a: 表达式<'l>, b: 表达式<'l>, o: 比较) -> Fallible<表达式<'l>> {
        let (emit_int, emit_float, emit_value): (
            binary_operate_type!(e::I64, LuaValue),
            binary_operate_type!(e::F64, LuaValue),
            binary_operate_type!(LuaValue),
        ) = match o {
            比较::大于 => (ILarge::emit, FLarge::emit, Large::emit),
            比较::小于 => (ILess::emit, FLess::emit, Less::emit),
            比较::不大于 => (ILessOrEqual::emit, FLessOrEqual::emit, LessOrEqual::emit),
            比较::不小于 => (ILargeOrEqual::emit, FLargeOrEqual::emit, LargeOrEqual::emit),
            比较::等于 => (IEqual::emit, FEqual::emit, Equal::emit),
            比较::不等于 => (INotEqual::emit, FNotEqual::emit, NotEqual::emit),
        };
        self.raw
            .binary_to_bool_operate(a, b, Some(emit_int), Some(emit_float), emit_value)
    }
    pub fn 分支(
        &mut self,
        a: 表达式<'l>,
        (predicate_block_end, then_block_begin): (基本块<'l>, 基本块<'l>),
        (then_block_end, post_then_block_begin): (基本块<'l>, 基本块<'l>),
        else_branch: Option<(基本块<'l>, 基本块<'l>)>,
    ) -> Fallible<()> {
        self.raw
            .branch_if(a, &predicate_block_end, &then_block_begin, &post_then_block_begin)?;
        if let Some((else_block_end, post_block_begin)) = else_branch {
            self.raw.branch(&then_block_end, &post_block_begin)?;
            self.raw.branch(&else_block_end, &post_block_begin)?;
        } else {
            self.raw.branch(&then_block_end, &post_then_block_begin)?;
        };
        Ok(())
    }
    pub fn 循环准备(&mut self) -> Fallible<()> { self.raw.loop_head() }
    pub fn 循环头(
        &mut self,
        times: 表达式<'l>,
        (init_block_end, predicate_block_begin): (基本块<'l>, 基本块<'l>),
        (predicate_block_end, loop_block_begin): (基本块<'l>, 基本块<'l>),
    ) -> Fallible<(
        表达式<'l>,
        表达式<'l>,
        (基本块<'l>, 基本块<'l>),
        (基本块<'l>, 基本块<'l>),
        表达式<'l>,
    )> {
        let pre_block_end = &init_block_end.borrow(self.raw.token()).builder().clone();
        self.raw.current_builder = pre_block_end.clone();
        let start = self.raw.const_number(LuaNumberLit::Integer(0))?;
        let start = self.raw.to_value(start)?;
        let state = self.raw.const_number(LuaNumberLit::Integer(0))?;
        let state = self.raw.to_value(state)?;
        let times = self.raw.to_value(times)?;
        self.raw.current_builder = loop_block_begin.borrow(self.raw.token()).builder().clone();
        Ok((
            start,
            times,
            (init_block_end, predicate_block_begin),
            (predicate_block_end, loop_block_begin),
            state,
        ))
    }
    pub fn 循环(
        &mut self,
        h: (
            表达式<'l>,
            表达式<'l>,
            (基本块<'l>, 基本块<'l>),
            (基本块<'l>, 基本块<'l>),
            表达式<'l>,
        ),
        l: (基本块<'l>, 基本块<'l>),
    ) -> Fallible<()> {
        self.raw.for_(h, l)
    }
    pub fn 恒循环(
        &mut self,
        (init_block_end, loop_block_begin): (基本块<'l>, 基本块<'l>),
        (loop_block_end, _post_block_begin): (基本块<'l>, 基本块<'l>),
    ) -> Fallible<()> {
        self.raw.branch(&init_block_end, &loop_block_begin)?;
        self.raw.branch(&loop_block_end, &loop_block_begin)?;
        Ok(())
    }
    pub fn 跳出(&mut self) -> Fallible<()> { self.raw.break_() }
    pub fn 分块(&mut self) -> Fallible<(基本块<'l>, 基本块<'l>)> { self.raw.split_block() }
    pub fn 返回(&mut self, values: Option<表达式列表<'l>>) -> Fallible<()> {
        self.raw
            .emit_return(values.map(|values| LuaExprListBuilder::default().exprs(values).build().unwrap()))
    }
    pub fn 返回操作数(&mut self, value: 表达式<'l>) -> Fallible<()> {
        self.raw.emit_return(Some(LuaExprList::from(value)))
    }
    pub fn 书之(&mut self, value: 表达式<'l>) -> Fallible<()> {
        let function = self.raw.get_value("print_wenyan".to_string())?;
        self.raw.emit_call(
            function,
            LuaExprListBuilder::default().exprs(vec![value]).build().unwrap(),
        )?;
        Ok(())
    }
    pub fn 打包(self) -> Fallible<程序> { self.raw.pack() }
}
pub extern "C" fn print_wenyan(state: LuaStateReference, args: &[LuaValueImpl]) -> Pointer<UnsizedArray<LuaValue>> {
    let mut buffer = Vec::new();
    for (arg_index, arg) in args.iter().enumerate() {
        if arg_index != 0 {
            buffer.push(b'\t');
        }
        unsafe {
            let buffer: &mut Vec<u8> = &mut buffer;
            let mut i = arg.clone();
            if let Some(v) = i.read_integer() {
                let v = (v.0) >> 4;
                buffer.extend(v.to_lowercase_ten_thousand(ChineseVariant::Traditional).as_bytes());
            } else if let Some(v) = i.read_big_int() {
                let v = v.as_ref().get_value().0;
                buffer.extend(v.to_lowercase_ten_thousand(ChineseVariant::Traditional).as_bytes());
            } else if let Some(v) = i.read_float() {
                let v = v.0;
                let v = f64::from_le_bytes(i64::to_le_bytes(v));
                buffer.extend(v.to_lowercase_ten_thousand(ChineseVariant::Traditional).as_bytes());
            } else if let Some(v) = i.read_big_float() {
                let v = v.as_ref().get_value().0;
                buffer.extend(v.to_lowercase_ten_thousand(ChineseVariant::Traditional).as_bytes());
            } else if let Some(v) = i.read_string() {
                buffer.extend(v.as_ref().ref_data().as_slice().iter().map(|d| d.0));
            } else if let Some(_v) = i.read_nil() {
                buffer.extend_from_slice("空".as_bytes());
            } else if let Some(v) = i.read_boolean() {
                let v = v.0 != 0;
                buffer.extend(v.to_string().as_bytes());
            } else if let Some(v) = i.read_table() {
                buffer.extend(format!("表({:p})", v.as_ptr()).bytes());
            } else if let Some(v) = i.read_function() {
                buffer.extend(format!("函数({:p})", v.as_ptr()).bytes());
            } else if let Some(v) = i.read_closure() {
                buffer.extend(format!("函数({:p})", v.as_ptr()).bytes());
            } else {
                error!("错误的值: {:X?}", &i.0);
            }
        }
    }
    println!("{}", String::from_utf8_lossy(&buffer));
    empty_return()
}
#[cfg(feature = "runtime")]
pub extern "C" fn exec_wenyan(state: LuaStateReference, args: &[LuaValueImpl]) -> Pointer<UnsizedArray<LuaValue>> {
    if let Some(v) = args[0].read_string() {
        let code = unsafe {
            let mut buffer = Vec::new();
            buffer.extend(v.as_ref().ref_data().as_slice().iter().map(|d| d.0));
            String::from_utf8_lossy(&buffer).to_string()
        };
        运行代码(state, code.as_ref()).unwrap();
    } else {
        panic!("代码值不是字符串");
    }
    empty_return()
}
pub fn 创建虚拟机(runtime: 运行时) -> Fallible<虚拟机> {
    let vm = new_state(runtime)?;
    Ok(vm)
}
pub fn 加入虚拟机(vm: 虚拟机) -> Fallible<虚拟机> {
    add_global_function(vm.clone(), "print_wenyan", &(print_wenyan as LuaFunctionRustType))?;
    #[cfg(feature = "runtime")]
    {
        add_global_function(vm.clone(), "exec_wenyan", &(exec_wenyan as LuaFunctionRustType))?;
    }
    Ok(vm)
}
pub fn 打招呼() {
    println!("問天地好在。『zitao 文言 虚拟机 v{} 』", &env!("CARGO_PKG_VERSION"));
}
pub fn 加载代码(vm: 虚拟机, code: &str) -> Fallible<ObjectRef> {
    debug!("code: {:?}", code);
    let lexical = 文言词法::parse(code)?;
    debug!("lexical: {:?}", &lexical);
    let mut pack = 解析语法(vm.clone(), lexical)?;
    let root_function = pack.pop().unwrap();
    let vm_pointer = vm.as_pointer();
    let runtime = unsafe { vm_pointer.as_ref().ref_runtime() };
    let resource = runtime.create_dyn(root_function)?;
    for closure in pack {
        runtime.create_dyn(closure)?;
    }
    let object = resource.get_object()?;
    Ok(object)
}
pub fn 运行代码(vm: 虚拟机, code: &str) -> Fallible<()> {
    let resource = 加载代码(vm.clone(), code)?;
    unsafe {
        let function: LuaFunctionRustType = std::mem::transmute(resource.lock().unwrap().get_export_ptr(0));
        let args = &[];
        function(vm, args);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{创建虚拟机, 运行代码};
    use failure::Fallible;
    use llvm_runtime::{Interpreter, JITCompiler};
    use memory_mmmu::MemoryMMMU;
    use vm_lua::LuaInstructionSet;
    pub type LuaInterpreter = Interpreter<LuaInstructionSet, MemoryMMMU>;
    pub type LuaJIT = JITCompiler<LuaInstructionSet, MemoryMMMU>;
    #[test]
    fn run_wenyan_script() -> Fallible<()> {
        env_logger::init();
        vm_lua::util::set_signal_handler();
        let vm = 创建虚拟机(Arc::new(LuaInterpreter::new()?))?;
        let code = "為是十遍。吾有一言。曰「「問天地好在。」」。書之。云云。";
        运行代码(vm.clone(), code)?;
        Ok(())
    }
}
