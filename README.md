# 基于元编程的多语言虚拟机项目

项目中提供一套宏系统和框架，可以快速实现一种编程语言的虚拟机，并且实现的多个虚拟机可以互通，包括相互调用和相互引用等特性。

# 特性
- [x] 词法分析器生成宏
- [x] 语法分析器生成宏
- [x] 中间码定义宏
- [x] 类型元数据宏
- [x] 寄存器式解释器生成器
- [x] JIT编译器生成器
- [x] 只需要定义中间码即可生成出解释器和JIT编译器
- [x] 只需要定义中间码即可生成出中间码编码器
- [x] 独立的内存分配器
- [ ] 无停顿并发GC系统
- [ ] 适配typescript类型系统

# 目前已经实现的虚拟机

## lua虚拟机

使用5260行代码实现一个带有解释器和JIT编译的lua虚拟机，支持内联缓存和指针压缩，其解释速度比clua虚拟机快30%左右。

- [ ] 支持全部内置函数
- [x] 支持解释器
- [x] 支持JIT编译器

## wenyan虚拟机

使用500行代码实现wenyan虚拟机，运行速度与lua相当。

- [ ] 支持全部语法
- [ ] 支持全部内置函数
- [x] 支持与lua相互调用，相互引用
- [x] 支持解释器
- [x] 支持JIT编译器

# 计划实现的虚拟机

## java虚拟机
- [ ] 支持内存多重映射
- [ ] 支持与lua户通
## typescript虚拟机
- [ ] 将typescript类型转换为native类型
- [ ] 支持与jvm户通
- [ ] 支持与lua户通

# 如何制作一个新脚本语言虚拟机(以wenyan为例)

实现wenyan脚本语言的所有代码都在`vm-wenyan`模块中,一共500行代码。

1. 定义词法

```rust
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
```
2. 设计特殊词法的解析器
```rust
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
```
3. 设计语法树解析器
```rust
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
```
4. 设计中间码构建器
这里直接生成lua中间码

```rust
pub struct 中间码构建器<'l> {
    raw: LuaContext<'l>,
}
impl<'l> 中间码构建器<'l> {
    // ...
    pub fn 书之(&mut self, value: 表达式<'l>) -> Fallible<()> {
        let function = self.raw.get_value("print_wenyan".to_string())?;
        self.raw.emit_call(
            function,
            LuaExprListBuilder::default().exprs(vec![value]).build().unwrap(),
        )?;
        Ok(())
    }
    // ...
}
```

5. 设计内置函数
```rust
pub extern "C" fn print_wenyan(_state: LuaStateReference, args: &[LuaValueImpl]) -> Pointer<UnsizedArray<LuaValue>> {
    let mut buffer = Vec::new();
    for (arg_index, arg) in args.iter().enumerate() {
        if arg_index != 0 {
            buffer.push(b'\t');
        }
        unsafe {
            let buffer: &mut Vec<u8> = &mut buffer;
            let i = arg.clone();
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
```
6. 制作运行脚本的函数
把这些步骤链接起来
```rust

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
```

7. 封装shell

```rust

fn main() -> Fallible<()> {
    env_logger::init();
    vm_lua::util::set_signal_handler();
    let vm = vm_wenyan::创建虚拟机(&*LUA_INTERPRETER)?;
    vm_wenyan::打招呼();
    loop {
        print!("");
        std::io::stdout().flush().unwrap();
        let mut code = String::new();
        let len = stdin().read_line(&mut code)?;
        if len == 0 || &code == "\n" {
            break;
        }
        if let Err(e) = vm_wenyan::运行代码(vm.clone(), &code) {
            error!("{e}")
        };
    }
    Ok(())
}
```
