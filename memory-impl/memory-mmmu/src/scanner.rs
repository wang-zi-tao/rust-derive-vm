use std::{cell::RefCell, ptr::NonNull, rc::Rc, sync::Arc, thread::JoinHandle};

use crossbeam_deque::{Injector, Worker};
use failure::{format_err, Error, Fail, Fallible};
use jvm_core::{
    EnumTagLayout, ExecutableResourceTrait, FunctionTypeBuilder, Native, ObjectRef, Pointer, Resource, RuntimeTrait, SmallElementLayout, Type, TypeDeclaration,
    TypeLayout, TypeResource, UnsafeSymbolRef, _ghost_cell::GhostToken,
};
use runtime::code::{BlockBuilder, FunctionBuilder, FunctionPack, LinearRegisterPool, Register, RegisterPool};
use runtime_derive::make_instruction_set;
use runtime_extra as e;
use runtime_extra::{Usize, I8};
use util::CowArc;

use crate::{gc::GCPlan, mark::GlobalMarkSet, metadata::TypeMetadata, MemoryMMMU, RegistedType};

use self::i::{Block, BlockImpl, LOCAL_STACK_MAX_SIZE};
type RegisterPoolImpl = LinearRegisterPool<{ Block::LAYOUT.size() }>;

mod i {
    use super::GCWorker;
    use std::num::NonZeroU64;

    use jvm_core::{Array, Native, Pointer, TypeDeclaration};
    use runtime::instructions::bootstrap::*;
    use runtime_derive::{make_instruction, make_native_function, Instruction, TypeDeclaration};
    use runtime_extra::*;

    #[derive(TypeDeclaration)]
    #[make_type(make_instruction)]
    pub struct Block {
        size: Usize,
        value: Array<Usize, 63>,
    }

    impl Block {
        pub fn len(this: &BlockImpl) -> usize {
            this.get_size().0
        }

        pub fn full(this: &BlockImpl) -> bool {
            this.get_size().0 == this.get_value().len()
        }

        pub fn push(this: &mut BlockImpl, value: Usize) {
            this.get_value()[Self::len(this)] = value;
        }

        pub fn as_slice(this: &BlockImpl) -> &[Usize] {
            &this.ref_value()[0..Self::len(this)]
        }

        pub fn as_slice_mut(this: &mut BlockImpl) -> &mut [Usize] {
            let len = Self::len(this);
            &mut this.ref_value_mut()[0..len]
        }
    }
    impl std::clone::Clone for BlockImpl {
        fn clone(&self) -> Self {
            Self(self.0)
        }
    }
    use block::*;
    pub(crate) type BlockRef = Pointer<Block>;
    make_instruction! {
        ReadBlockElement->fn(block:BlockRef,index:Usize)->(elem:Usize){ entry:{
            %elem = Read<Usize::TYPE>(LocateElement<Usize::TYPE>(LocateValue(%block),%index));
        }}
    }
    make_instruction! {
        BlockPush->fn(block:BlockRef,elem:Usize){ entry:{
            %elem = Write<Usize::TYPE>(
                LocateElement<Usize::TYPE>(
                    LocateValue(%block),
                    ReadSize(%block)),
                %elem);
            WriteSize(%block,UsizeAdd(1,ReadSize(%block)));
        }}
    }
    make_instruction! {
        WriteBlockElement->fn(block:BlockRef,index:Usize,elem:Usize){ entry:{
            %elem = Write<Usize::TYPE>(LocateElement<Usize::TYPE>(LocateValue(%block),%index),%elem);
        }}
    }
    make_instruction! {
        WriteBlockElement1->fn<block then>(block:BlockRef,index:Usize,elem:Usize){ entry:{
            %elem = Write<Usize::TYPE>(LocateElement<Usize::TYPE>(LocateValue(%block),%index),%elem);
        }}
    }
    macro_rules! repeat {
        (map $name:ident->fn$(<$(const $generate:ident:$generates_ty:ty),*>)?($($inputs:ident:$input_type:ty),*){$($implement:tt)*}) => {
            make_instruction! {
                 $name->fn$(<$(const $generate:$generates_ty),*>)?(block_in:BlockRef,block_out:BlockRef,$($inputs:$input_type),*){
                     entry:{
                         %i=0;
                         WriteSize(%block_out,ReadSize(%block_in));
                         if UsizeGt(ReadSize(%input),%i) %loop %end;
                     },
                     loop:{
                         phi %i:Usize={%entry=>%i,%loop=>%i1};
                         %in = ReadBlockElement(%block_in,%i);
                         $($implement)*
                         WriteBlockElement(%block_out,%i,%out);
                         %i1 = UsizeAdd(%i,1);
                         if UsizeGt(ReadSize(%input),%i) %loop %end;
                     },
                     end:{}
                 }
            }
        };
        (filter $name:ident->fn$(<$(const $generates:ident:$generates_ty:ty),*>)?($($inputs:ident:$input_type:ty),*){$($implement:tt)*}) => {
            make_instruction! {
                 $name->fn<block not_empty,block empty,$($(const $generates:$generates_ty),*)?>(block_in:BlockRef,block_out:BlockRef,$($inputs:$input_type),*){
                     entry:{
                         %i=0;
                         WriteSize(%block_out,0);
                         if UsizeGt(ReadSize(%input),%i) %loop %end;
                     },
                     loop:{
                         phi %i:Usize={%entry=>%i,%loop=>%i1};
                         %in = ReadBlockElement(%block_in,%i);
                         $($implement)*
                         %i1 = UsizeAdd(%i,1);
                         if %out %write %no_write;
                     },
                     write:{
                         BlockPush(%block_out,%in);
                         branch %no_write;
                     },
                     no_write:{
                         if UsizeGt(ReadSize(%input),%i) %loop %end;
                     },
                     end:{
                         if UsizeGt(ReadSize(%block_out),0) %not_empty %empty;
                     }
                 }
            }
        };
    }
    repeat! {map LocateField->fn<const offset:Usize>(){
        %out = UsizeAdd(%in,%offset);
    }}
    repeat! {map ReadComposedField->fn<const mask:Usize,const bit_offset:I8>(){
        %value = Read<Usize::TYPE>(CastUnchecked<Pointer::<Usize>::TYPE,Usize::TYPE>(%in));
        %out = UsizeShr(UsizeAnd(%value,%mask),IntExtend<0,8>(%bit_offset));
    }}
    repeat! {map ReadMaskField->fn<const mask:Usize>(){
        %value = Read<Usize::TYPE>(CastUnchecked<Pointer::<Usize>::TYPE,Usize::TYPE>(%in));
        %out = UsizeAnd(%value,%mask);
    }}
    repeat! {map GetComposedField->fn<const mask:Usize,const bit_offset:I8>(){
        %out = UsizeShr(UsizeAnd(%in,%mask),IntExtend<0,8>(%bit_offset));
    }}
    repeat! {map GetMaskField->fn<const mask:Usize>(){
        %out = UsizeAnd(%in,%mask);
    }}
    repeat! {map GetNicheTag->fn<const start:Usize>(){
        %out = UsizeSub(%in,%start);
    }}
    repeat! {map GetU8Tag->fn<const offset:Usize>(){
        %out = Read<U8::TYPE>(CastUnchecked<Pointer::<U8>::TYPE,Usize::TYPE>(UsizeAdd(%in,%offset)));
    }}
    repeat! {map GetU16Tag->fn<const offset:Usize>(){
        %out = Read<U16::TYPE>(CastUnchecked<Pointer::<U16>::TYPE,Usize::TYPE>(UsizeAdd(%in,%offset)));
    }}
    repeat! {map GetU32Tag->fn<const offset:Usize>(){
        %out = Read<U32::TYPE>(CastUnchecked<Pointer::<U32>::TYPE,Usize::TYPE>(UsizeAdd(%in,%offset)));
    }}
    repeat! {filter FilterByNicheTag->fn<const start:Usize,const count:Usize>(){
        %out = U8Ge(GetTag(%in,%start),%count);
    }}
    repeat! {filter ReadNicheTagAndFilter->fn<const start:Usize,const count:Usize>(){
        %value = Read<Usize::TYPE>(CastUnchecked<Pointer::<Usize>::TYPE,Usize::TYPE>(%in));
        %out = U8Ge(GetTag(%value,%start),%count);
    }}
    repeat! {filter FilterByU8Tag->fn<const tag:Usize,const offset:Usize>(){
        %value = Read<Usize::TYPE>(CastUnchecked<Pointer::<U8>::TYPE,Usize::TYPE>(UsizeAdd(%in,%offset)));
        %out = U8Eq(%tag,%value);
    }}
    repeat! {filter FilterByU16Tag->fn<const tag:Usize,const offset:Usize>(){
        %value = Read<Usize::TYPE>(CastUnchecked<Pointer::<U16>::TYPE,Usize::TYPE>(UsizeAdd(%in,%offset)));
        %out = U16Eq(%tag,%value);
    }}
    repeat! {filter FilterByU32Tag->fn<const tag:Usize,const offset:Usize>(){
        %value = Read<Usize::TYPE>(CastUnchecked<Pointer::<U32>::TYPE,Usize::TYPE>(UsizeAdd(%in,%offset)));
        %out = U32Eq(%tag,%value);
    }}
    repeat! {filter GetComposedTagAndFilter->fn<const mask:Usize,const bit_offset:I8,const except_tag:Usize>(){
        %tag = UsizeShr(UsizeAnd(%in,%mask),IntExtend<0,8>(%bit_offset));
        %out = UsizeEq(%tag,%except_tag);
    }}
    repeat! {filter ReadComposedTagAndFilter->fn<const mask:Usize,const bit_offset:I8,const except_tag:Usize>(){
        %value = Read<Usize::TYPE>(CastUnchecked<Pointer::<Usize>::TYPE,Usize::TYPE>(%in));
        %tag = UsizeShr(UsizeAnd(%value,%mask),IntExtend<0,8>(%bit_offset));
        %out = UsizeEq(%tag,%except_tag);
    }}
    repeat! {filter Eq->fn<const tag:Usize>(){
        %out = UsizeEq(%tag,%in);
    }}
    repeat! {filter Ge->fn<const tag:Usize>(){
        %out = UsizeGe(%in,%tag);
    }}
    pub(crate) struct GCMetadata {
        gc_type_index: Option<NonZeroU64>,
    }
    pub(crate) const LOCAL_STACK_MAX_SIZE: usize = 16 * 1024;
    #[make_native_function(RawPushReference)]
    unsafe extern "C" fn __memory_impl_scanner_push(this: Pointer<Native<GCWorker>>, reference: Usize) {
        GCWorker::push(this.as_non_null().as_mut(), reference.0);
    }
    #[make_native_function(RawPushReferenceBlock)]
    unsafe extern "C" fn __memory_impl_scanner_push_block(this: Pointer<Native<GCWorker>>, block: BlockRef) {
        GCWorker::push_block(this.as_non_null().as_mut(), block.as_non_null().as_mut());
    }
    make_instruction! {PushReference->fn<block then>(gc:Pointer<Native<GCWorker>>,ptr:Usize){ entry:{
        RawPushReference(%gc,%ptr);
        branch %then;
    }}}
    make_instruction! {PushReferenceBlock->fn<block then>(gc:Pointer<Native<GCWorker>>,block:BlockRef){ entry:{
        RawPushReferenceBlock(%gc,%block);
        branch %then;
    }}}
    make_instruction! {
        PushUnsizedArray->fn<block then,const elem_size:Usize>(gc:Pointer<Native<GCWorker>>,block:BlockRef){
             entry:{
                 %i=0;
                 WriteSize(%block_out,ReadSize(%block_in));
                 if UsizeGe(ReadSize(%input),%i) %loop %end;
             },
             loop:{
                 phi %i:Usize={%entry=>%i,%loop=>%i1};
                 %array = ReadBlockElement(%block_in,%i);
                 %j=0;
                 %array_len = Read<Usize::TYPE>(CastUnchecked<Pointer::<Usize>::TYPE,Usize::TYPE>(%array));
                 if UsizeGt(%array_len,%i) %elem_loop %loop_end;
             },
             elem_loop:{
                 phi %j:Usize={%loop=>%j,%elem_loop=>%j1};
                 %elem = UsizeAdd(%array,UsizeMul(%j,%elem_size));
                 %j1 = UsizeAdd(%j,1);
                 if UsizeGt(%array_len,%j) %elem_loop %loop_end;
             },
             loop_end:{
                 WriteBlockElement(%block_out,%i,%out);
                 if UsizeGt(ReadSize(%input),%i) %loop %end;
             },
             end:{}
        }
    }
    make_instruction! {
        PushSizedArray->fn<block then,const elem_size:Usize,const array_len:Usize>(gc:Pointer<Native<GCWorker>>,block:BlockRef){
             entry:{
                 %i=0;
                 WriteSize(%block_out,ReadSize(%block_in));
                 if UsizeGe(ReadSize(%input),%i) %loop %end;
             },
             loop:{
                 phi %i:Usize={%entry=>%i,%loop=>%i1};
                 %array = ReadBlockElement(%block_in,%i);
                 %j=0;
                 if UsizeGt(%array_len,%i) %elem_loop %loop_end;
             },
             elem_loop:{
                 phi %j:Usize={%loop=>%j,%elem_loop=>%j1};
                 %elem = UsizeAdd(%array,UsizeMul(%j,%elem_size));
                 %j1 = UsizeAdd(%j,1);
                 if UsizeGt(%array_len,%j) %elem_loop %loop_end;
             },
             loop_end:{
                 WriteBlockElement(%block_out,%i,%out);
                 if UsizeGt(ReadSize(%input),%i) %loop %end;
             },
             end:{}
        }
    }
}

make_instruction_set! {
    GCInstructionSet=[
        LocateField->i::LocateField,
        ReadComposedField->i::ReadComposedField,
        ReadMaskField->i::ReadMaskField,
        GetComposedField->i::GetComposedField,
        GetMaskField->i::GetMaskField,
        GetNicheTag->i::GetNicheTag,
        GetU8Tag->i::GetU8Tag,
        GetU16Tag->i::GetU16Tag,
        GetU32Tag->i::GetU32Tag,
        FilterByNicheTag->i::FilterByNicheTag,
        ReadNicheTagAndFilter->i::ReadNicheTagAndFilter,
        FilterByU8Tag->i::FilterByU8Tag,
        FilterByU16Tag->i::FilterByU16Tag,
        FilterByU32Tag->i::FilterByU32Tag,
        GetComposedTagAndFilter->i::GetComposedTagAndFilter,
        ReadComposedTagAndFilter->i::ReadComposedTagAndFilter,
        PushReferenceBlock->i::PushReferenceBlock,
        PushSizedArray->i::PushSizedArray,
        PushUnsizedArray->i::PushUnsizedArray,
        Eq->i::Eq,
        Ge->i::Ge,
        Goto->e::Goto,
    ]
}
#[derive(Clone, Copy)]
enum BlockKind {
    Pointer,
    Value,
}
struct ScanContext<'l> {
    function: FunctionBuilder<'l, GCInstructionSet>,
    register_pool: Rc<RefCell<RegisterPoolImpl>>,
    gc_thread_local: Register<Pointer<Native<GCWorker>>, RegisterPoolImpl>,
}
enum ScanPath {
    Reference,
    Embed,
    Pointer { sub_path: Box<Self> },
    Enum { tag: EnumTagLayout, sub_paths: Vec<(usize, Self)> },
    Tuple { sub_paths: Vec<(usize, usize, Self)> },
    ComposedTuple { sub_paths: Vec<(SmallElementLayout, Self)> },
    Array { size: Option<usize>, element_size: usize },
}
impl ScanPath {
    pub fn build(&self) -> Fallible<FunctionPack<GCInstructionSet>> {
        GhostToken::new(|mut token| {
            let mut context =
                ScanContext { register_pool: RegisterPoolImpl::reserve_range(0..2), gc_thread_local: Register::new_const(0), function: Default::default() };
            let mut block = context.function.new_block();
            let mut exit = context.function.new_block();
            let input = Register::new_const(1);
            self.do_build(&mut token, &input, BlockKind::Pointer, &mut block, &mut exit, &mut context)?;
            let reg_count = context.register_pool.borrow().max_allocated();
            context.function.pack(
                &mut token,
                FunctionTypeBuilder::default().args(vec![Pointer::<Native<GCWorker>>::TYPE, Pointer::<Block>::TYPE].into()).build().unwrap(),
                reg_count,
            )
        })
    }

    pub fn do_build<'l>(
        &self,
        token: &mut GhostToken<'l>,
        input: &Register<Pointer<i::Block>, RegisterPoolImpl>,
        input_kind: BlockKind,
        block: &mut BlockBuilder<'l, GCInstructionSet>,
        exit: &mut BlockBuilder<'l, GCInstructionSet>,
        context: &mut ScanContext<'l>,
    ) -> Fallible<()> {
        match self {
            ScanPath::Pointer { sub_path } => {
                sub_path.do_build(token, input, input_kind, block, exit, context)?;
            }
            ScanPath::Enum { tag, sub_paths } => {
                let mut current_block = block.clone();
                for (index, (variant_index, sub_path)) in sub_paths.iter().enumerate() {
                    let process_block = context.function.new_block();
                    let output = RegisterPoolImpl::alloc(context.register_pool.clone()).ok_or_else(|| format_err!("can not alloc register"))?;
                    let tags = RegisterPoolImpl::alloc(context.register_pool.clone()).ok_or_else(|| format_err!("can not alloc register"))?;
                    match sub_paths.len() {
                        0 => unreachable!(),
                        1 => match (tag, input_kind) {
                            (EnumTagLayout::UndefinedValue { end, start }, BlockKind::Pointer) => {
                                FilterByNicheTag::emit(&mut current_block, token, &process_block, exit, Usize(*start), Usize(*end - *start), input, &output)?;
                            }
                            (EnumTagLayout::UndefinedValue { end, start }, BlockKind::Value) => {
                                ReadNicheTagAndFilter::emit(
                                    &mut current_block,
                                    token,
                                    &process_block,
                                    exit,
                                    Usize(*start),
                                    Usize(*end - *start),
                                    input,
                                    &output,
                                )?;
                            }
                            (EnumTagLayout::UnusedBytes { offset, size }, BlockKind::Pointer) => {
                                GetComposedTagAndFilter::emit(
                                    &mut current_block,
                                    token,
                                    &process_block,
                                    exit,
                                    Usize((usize::MAX >> (64 - offset * 8 - (*size as usize))) & (usize::MAX << (offset * 8))),
                                    I8(*offset as i8 * 8),
                                    Usize(*variant_index),
                                    input,
                                    &output,
                                )?;
                            }
                            (EnumTagLayout::UnusedBytes { offset, size }, BlockKind::Value) => {
                                let emit = match size {
                                    8 => FilterByU8Tag::emit,
                                    16 => FilterByU16Tag::emit,
                                    32 => FilterByU32Tag::emit,
                                    _ => unreachable!(),
                                };
                                emit(&mut current_block, token, &process_block, exit, Usize(*variant_index), Usize(*offset), input, &output)?;
                            }
                            (EnumTagLayout::SmallField(tag_layout), BlockKind::Pointer) => {
                                ReadComposedTagAndFilter::emit(
                                    &mut current_block,
                                    token,
                                    &process_block,
                                    exit,
                                    Usize(tag_layout.mask()),
                                    I8(tag_layout.bit_offset()),
                                    Usize(*variant_index),
                                    input,
                                    &output,
                                )?;
                            }
                            (EnumTagLayout::SmallField(tag_layout), BlockKind::Value) => {
                                GetComposedTagAndFilter::emit(
                                    &mut current_block,
                                    token,
                                    &process_block,
                                    exit,
                                    Usize(tag_layout.mask()),
                                    I8(tag_layout.bit_offset()),
                                    Usize(*variant_index),
                                    input,
                                    &output,
                                )?;
                            }
                            (EnumTagLayout::AppendTag { offset, size }, BlockKind::Pointer) => {
                                let emit = match size {
                                    8 => FilterByU8Tag::emit,
                                    16 => FilterByU16Tag::emit,
                                    32 => FilterByU32Tag::emit,
                                    _ => unreachable!(),
                                };
                                emit(&mut current_block, token, &process_block, exit, Usize(*variant_index), Usize(*offset), input, &output)?;
                            }
                            (EnumTagLayout::AppendTag { offset, size }, BlockKind::Value) => {
                                GetComposedTagAndFilter::emit(
                                    &mut current_block,
                                    token,
                                    &process_block,
                                    exit,
                                    Usize((usize::MAX >> (64 - offset * 8 - (*size as usize))) & (usize::MAX << (offset * 8))),
                                    I8(*offset as i8 * 8),
                                    Usize(*variant_index),
                                    input,
                                    &output,
                                )?;
                            }
                        },
                        _ => {
                            if index == 0 {
                                match (tag, input_kind) {
                                    (EnumTagLayout::UndefinedValue { .. }, BlockKind::Pointer) => {
                                        unreachable!()
                                    }
                                    (EnumTagLayout::UndefinedValue { .. }, BlockKind::Value) => {
                                        unreachable!()
                                    }
                                    (EnumTagLayout::UnusedBytes { offset, size }, BlockKind::Pointer) => {
                                        GetComposedField::emit(
                                            &mut current_block,
                                            token,
                                            Usize((usize::MAX >> (64 - offset * 8 - (*size as usize))) & (usize::MAX << (offset * 8))),
                                            I8(*offset as i8 * 8),
                                            input,
                                            &tags,
                                        )?;
                                    }
                                    (EnumTagLayout::UnusedBytes { offset, size }, BlockKind::Value) => {
                                        let emit = match size {
                                            8 => GetU8Tag::emit,
                                            16 => GetU16Tag::emit,
                                            32 => GetU32Tag::emit,
                                            _ => unreachable!(),
                                        };
                                        emit(&mut current_block, token, Usize(*offset), input, &tags)?;
                                    }
                                    (EnumTagLayout::SmallField(tag_layout), BlockKind::Pointer) => {
                                        GetComposedField::emit(&mut current_block, token, Usize(tag_layout.mask()), I8(tag_layout.bit_offset()), input, &tags)?;
                                    }
                                    (EnumTagLayout::SmallField(tag_layout), BlockKind::Value) => {
                                        GetComposedField::emit(&mut current_block, token, Usize(tag_layout.mask()), I8(tag_layout.bit_offset()), input, &tags)?;
                                    }
                                    (EnumTagLayout::AppendTag { offset, size }, BlockKind::Pointer) => {
                                        let emit = match size {
                                            8 => GetU8Tag::emit,
                                            16 => GetU16Tag::emit,
                                            32 => GetU32Tag::emit,
                                            _ => unreachable!(),
                                        };
                                        emit(&mut current_block, token, Usize(*offset), input, &tags)?;
                                    }
                                    (EnumTagLayout::AppendTag { offset, size }, BlockKind::Value) => {
                                        GetComposedField::emit(
                                            &mut current_block,
                                            token,
                                            Usize((usize::MAX >> (64 - offset * 8 - (*size as usize))) & (usize::MAX << (offset * 8))),
                                            I8(*offset as i8 * 8),
                                            input,
                                            &tags,
                                        )?;
                                    }
                                }
                            }
                            match tag {
                                EnumTagLayout::UndefinedValue { end, start } => {
                                    Ge::emit(&mut current_block, token, &process_block, exit, Usize(*end - *start), &tags, &output)?;
                                }
                                _ => {
                                    Eq::emit(&mut current_block, token, &process_block, exit, Usize(*variant_index), &tags, &output)?;
                                }
                            }
                        }
                    }
                    sub_path.do_build(token, &output, input_kind, block, exit, context)?;
                    current_block = process_block;
                }
                Goto::emit(&mut current_block, token, exit)?;
            }
            ScanPath::Tuple { sub_paths } => {
                let mut current_block = block.clone();
                for (index, (offset, size, sub_path)) in sub_paths.iter().enumerate() {
                    let mut exit = if index == sub_paths.len() { exit.clone() } else { context.function.new_block() };
                    let output = RegisterPoolImpl::alloc(context.register_pool.clone()).ok_or_else(|| format_err!("can not alloc register"))?;
                    match input_kind {
                        BlockKind::Pointer => {
                            LocateField::emit(&mut current_block, token, Usize(*offset), input, &output)?;
                        }
                        BlockKind::Value => {
                            ReadComposedField::emit(
                                &mut current_block,
                                token,
                                Usize((usize::MAX >> (64 - offset * 8 - size)) & (usize::MAX << (offset * 8))),
                                I8(*offset as i8 * 8),
                                input,
                                &output,
                            )?;
                        }
                    }
                    sub_path.do_build(token, &output, input_kind, block, &mut exit, context)?;
                    current_block = exit;
                }
            }
            ScanPath::ComposedTuple { sub_paths } => {
                let mut current_block = block.clone();
                for (index, (layout, sub_path)) in sub_paths.iter().enumerate() {
                    let mut exit = if index == sub_paths.len() { exit.clone() } else { context.function.new_block() };
                    let output = RegisterPoolImpl::alloc(context.register_pool.clone()).ok_or_else(|| format_err!("can not alloc register"))?;
                    match input_kind {
                        BlockKind::Pointer => {
                            if layout.bit_offset() == 0 {
                                ReadMaskField::emit(&mut current_block, token, Usize(layout.mask()), input, &output)?;
                            } else {
                                ReadComposedField::emit(&mut current_block, token, Usize(layout.mask()), I8(layout.bit_offset()), input, &output)?;
                            }
                        }
                        BlockKind::Value => {
                            if layout.bit_offset() == 0 {
                                GetMaskField::emit(&mut current_block, token, Usize(layout.mask()), input, &output)?;
                            } else {
                                GetComposedField::emit(&mut current_block, token, Usize(layout.mask()), I8(layout.bit_offset()), input, &output)?;
                            }
                        }
                    }
                    sub_path.do_build(token, &output, input_kind, block, &mut exit, context)?;
                    current_block = exit;
                }
            }
            ScanPath::Array { size, element_size } => {
                if let Some(size) = size {
                    PushSizedArray::emit(block, token, exit, Usize(*element_size), Usize(*size), &context.gc_thread_local, input)?;
                } else {
                    PushUnsizedArray::emit(block, token, exit, Usize(*element_size), &context.gc_thread_local, input)?;
                }
            }
            ScanPath::Reference => {
                PushReferenceBlock::emit(block, token, exit, &context.gc_thread_local, input)?;
            }
            ScanPath::Embed => {
                PushReferenceBlock::emit(block, token, exit, &context.gc_thread_local, input)?;
            }
        }
        Ok(())
    }

    pub fn scan(plan: &GCPlan, ty: &Type) -> Fallible<Option<Self>> {
        Ok(match ty {
            jvm_core::Type::Tuple(t) => match t {
                jvm_core::Tuple::Normal(fields) => {
                    let mut sub_paths = Vec::new();
                    let mut layout_builder = TypeLayout::new().builder();
                    for (_index, field) in fields.iter().enumerate() {
                        if let Some(sub_path) = Self::scan(plan, field)? {
                            let layout = field.get_layout()?;
                            layout_builder = layout_builder.extend(layout);
                            sub_paths.push((layout_builder.size(), layout.size(), sub_path));
                        }
                    }
                    if sub_paths.is_empty() {
                        None
                    } else {
                        Some(Self::Tuple { sub_paths })
                    }
                }
                jvm_core::Tuple::Compose(fields) => {
                    let mut sub_paths = Vec::new();
                    for (_index, (field, layout)) in fields.iter().enumerate() {
                        if let Some(sub_path) = Self::scan(plan, field)? {
                            sub_paths.push((*layout, sub_path));
                        }
                    }
                    if sub_paths.is_empty() {
                        None
                    } else {
                        Some(Self::ComposedTuple { sub_paths })
                    }
                }
            },
            jvm_core::Type::Enum(e) => {
                let mut sub_paths = Vec::new();
                for (index, variant) in e.variants.iter().enumerate() {
                    if let Some(sub_path) = Self::scan(plan, variant)? {
                        sub_paths.push((index, sub_path));
                    }
                }
                if sub_paths.is_empty() {
                    None
                } else {
                    Some(Self::Enum { sub_paths, tag: e.tag_layout })
                }
            }
            jvm_core::Type::Pointer(p) => Self::scan(plan, p)?.map(|sub_path| Self::Pointer { sub_path: Box::new(sub_path) }),
            jvm_core::Type::Array(inner, size) => {
                let layout = inner.get_layout()?;
                Some(Self::Array { size: *size, element_size: usize::max(layout.size(), layout.align()) })
            }
            jvm_core::Type::Reference(obj) => obj.try_map(|obj| {
                Ok(if plan.clean_types().contains(RegistedType::try_downcast(&**obj)?) {
                    Some(Self::Reference)
                } else {
                    None
                })
            })?,
            jvm_core::Type::Embed(inner) => inner.try_map(|obj| {
                Ok(if plan.scan_types().contains(RegistedType::try_downcast(&**obj)?) {
                    Some(Self::Embed)
                } else {
                    None
                })
            })?,
            _ => None,
        })
    }
}
pub(crate) struct GCHeapScanner {
    pub(crate) scan_types: Vec<(CowArc<'static, RegistedType>, ObjectRef)>,
    pub(crate) category_functions: Box<[UnsafeSymbolRef]>,
    pub(crate) global_stack: Injector<BlockImpl>,
    pub(crate) plan: GCPlan,
    pub(crate) markset: GlobalMarkSet,
}

pub struct GCWorker {
    pub(crate) send_buffer: BlockImpl,
    pub(crate) category_buffer: Vec<BlockImpl>,
    pub(crate) local_stack: Worker<BlockImpl>,
    pub(crate) tasks: Vec<CowArc<'static, RegistedType>>,
    pub(crate) global_scanner: Arc<GCHeapScanner>,
    pub(crate) markset: GlobalMarkSet,
}
impl GCWorker {
    pub(crate) fn scan(&mut self) -> Fallible<()> {
        for (_index, ty) in self.tasks.clone().iter().enumerate() {
            let small_heaps = ty.memory_pool.small_heaps.lock().unwrap();
            let scanner = |this: &mut Self, value: NonNull<u8>| {
                unsafe {
                    if let Some(index) = GCWorker::category(value.as_ptr()) {
                        if !this.markset.is_marked(value) {
                            let category = &mut this.category_buffer[index];
                            Block::push(category, Usize(value.as_ptr() as usize));
                            let category = &this.category_buffer[index];
                            if Block::full(category) {
                                let category = &mut this.category_buffer[index] as *mut BlockImpl;
                                let func: fn(&mut GCWorker, *mut BlockImpl) = std::mem::transmute(&this.global_scanner.category_functions[index].as_ptr());
                                func(this, category);
                            };
                        }
                    }
                }
                Ok(())
            };
            let layout = ty.get_layout()?;
            let len_offset = ty.get_len_offset()?;
            for small_heap in small_heaps.full.iter().chain(small_heaps.allocable.iter()) {
                unsafe {
                    small_heap.scan(layout, |v| scanner(self, v), len_offset);
                    self.pop();
                }
            }
            let large_heaps = ty.memory_pool.large_heaps.lock().unwrap();
            for large_heap in large_heaps.full.iter().chain(large_heaps.allocable.iter()) {
                unsafe {
                    large_heap.clone().scan(layout, |v| scanner(self, v), len_offset);
                    self.pop();
                }
            }
            unsafe {
                self.pop();
            }
        }
        Ok(())
    }

    #[inline(always)]
    pub(crate) unsafe fn push_block(this: &mut GCWorker, block: &mut BlockImpl) {
        for reference in Block::as_slice(block) {
            Self::push(this, reference.0);
        }
    }

    pub(crate) fn push(this: &mut GCWorker, reference: usize) {
        if Block::full(&this.send_buffer) {
            let buffer = std::mem::replace(&mut this.send_buffer, BlockImpl(Default::default()));
            if this.local_stack.len() >= LOCAL_STACK_MAX_SIZE {
                this.global_scanner.global_stack.push(buffer);
            } else {
                this.local_stack.push(buffer);
            }
        }
        Block::push(&mut this.send_buffer, Usize(reference));
    }

    #[inline(always)]
    pub(crate) unsafe fn category(ptr: *mut u8) -> Option<usize> {
        let meta_ptr = NonNull::new_unchecked((((ptr as usize) >> 3) as *mut *mut u8).read());
        let gc_meta = &TypeMetadata::from_raw(meta_ptr).as_ref().gc;
        let index = gc_meta.index.load();
        if index != usize::MAX {
            Some(index)
        } else {
            None
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn pop(&mut self) {
        loop {
            let block = self.local_stack.pop().or_else(|| -> Option<BlockImpl> {
                loop {
                    match self.global_scanner.global_stack.steal_batch_and_pop(&self.local_stack) {
                        crossbeam_deque::Steal::Empty => return None,
                        crossbeam_deque::Steal::Success(v) => return Some(v),
                        crossbeam_deque::Steal::Retry => {}
                    }
                }
            });
            if let Some(mut block) = block {
                for value in Block::as_slice(&mut block) {
                    let ptr = NonNull::new_unchecked(value.0 as *mut u8);
                    let index = Self::category(value.0 as *mut u8);
                    if let Some(index) = index {
                        if !self.markset.is_marked(ptr) {
                            let category = &mut self.category_buffer[index];
                            Block::push(category, *value);
                            let category = &self.category_buffer[index];
                            if Block::full(category) {
                                let category = &mut self.category_buffer[index] as *mut BlockImpl;
                                let func: fn(&mut GCWorker, *mut BlockImpl) = std::mem::transmute(&self.global_scanner.category_functions[index].as_ptr());
                                func(self, category);
                                self.markset.mark(ptr);
                            };
                        }
                    }
                }
            } else {
                break;
            };
        }
        loop {
            let mut finish = true;
            for i in 0..self.category_buffer.len() {
                let block = self.category_buffer[i].clone();
                if !Block::full(&block) {
                    let func: fn(&mut GCWorker, &BlockImpl) = std::mem::transmute(self.global_scanner.category_functions[i].as_ptr());
                    func(self, &block);
                    finish = false;
                }
            }
            if finish {
                break;
            }
        }
    }
}
impl GCHeapScanner {
    pub fn new<T>(plan: GCPlan, runtime: T) -> Fallible<Self>
    where
        T: RuntimeTrait<FunctionPack<GCInstructionSet>>,
        T::ResourceImpl: ExecutableResourceTrait<FunctionPack<GCInstructionSet>>,
    {
        let _clean_type = plan.clean_types();
        let mut scan_types = Vec::new();
        for scan_type in plan.scan_types() {
            if let Some(scan_path) = ScanPath::scan(&plan, scan_type.get_type()?)? {
                let function = runtime.define()?;
                let function_pack = scan_path.build()?;
                function.upload(function_pack)?;
                let object = function.get_object()?;
                scan_types.push((scan_type.clone(), object));
            }
        }
        let mut symbols = Vec::new();
        for (index, (scan_type, object)) in scan_types.iter().enumerate() {
            let metas = scan_type.metas().metas.read().unwrap();
            for meta in metas.iter() {
                let type_meta = unsafe { TypeMetadata::from_raw(meta.as_raw()).as_ref() };
                type_meta.gc.index.store(index);
            }
            symbols.push(unsafe { UnsafeSymbolRef::new_uninited(object.clone(), 0) })
        }
        let mut symbols: Box<[UnsafeSymbolRef]> = symbols.into();
        for ((_scan_type, _object), symbol) in scan_types.iter().zip(symbols.iter_mut()) {
            unsafe {
                symbol.init();
            }
        }
        Ok(Self { category_functions: symbols, global_stack: Injector::new(), plan, scan_types, markset: MemoryMMMU::get_instance().markset.clone() })
    }

    pub fn after_finish(&mut self) -> Fallible<()> {
        for (_index, (scan_type, _object)) in self.scan_types.iter().enumerate() {
            let metas = scan_type.metas().metas.read().unwrap();
            for meta in metas.iter() {
                let type_meta = unsafe { TypeMetadata::from_raw(meta.as_raw()).as_ref() };
                type_meta.gc.index.store(usize::MAX);
            }
        }
        Ok(())
    }

    pub fn new_worker(self: Arc<Self>, tasks: Vec<CowArc<'static, RegistedType>>) -> GCWorker {
        GCWorker {
            send_buffer: BlockImpl(Default::default()),
            category_buffer: self.category_functions.iter().map(|_| BlockImpl(Default::default())).collect(),
            local_stack: Worker::new_lifo(),
            tasks,
            markset: self.markset.clone(),
            global_scanner: self,
        }
    }

    pub fn new_workers(self: Arc<Self>, worker_count: usize) -> Fallible<Vec<JoinHandle<Fallible<()>>>> {
        let mut workers = Vec::new();
        for (index, tasks) in self.scan_types.chunks(self.scan_types.len().div_ceil(worker_count)).enumerate() {
            let tasks = tasks.iter().map(|(ty, _)| ty.clone()).collect();
            let mut worker_context = self.clone().new_worker(tasks);
            workers.push(std::thread::Builder::new().name(format!("gc_scanner_{}", index)).spawn(move || worker_context.scan())?);
        }
        Ok(workers)
    }

    pub fn work(self: Arc<Self>) -> Fallible<()> {
        let workers = self.new_workers(num_cpus::get())?;
        let mut errors = Vec::new();
        for worker in workers {
            match worker.join() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => errors.push(GCThreadError::Other(e)),
                Err(_e) => {
                    errors.push(GCThreadError::Panic());
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(GCScannerError::GCThreadError(errors).into())
        }
    }
}
#[derive(Fail, Debug)]
enum GCThreadError {
    #[fail(display = "gc thread panic")]
    Panic(),
    #[fail(display = "other error:{}", _0)]
    Other(#[cause] Error),
}
#[derive(Fail, Debug)]
enum GCScannerError {
    #[fail(display = "gc thread error:{:#?}", _0)]
    GCThreadError(Vec<GCThreadError>),
}
