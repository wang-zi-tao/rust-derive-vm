use super::mem::*;
use log::debug;
use log::error;
use runtime::instructions::{bootstrap::{self as b, CallState, GetLength, MakeSlice, Read, SetState, Write}, Instruction};
use runtime_extra::{self as e, instructions::*, ty::*};
use std::{cell::UnsafeCell, marker::PhantomData, mem::MaybeUninit};
use vm_core::{Direct, MoveIntoObject, ObjectBuilder, Pointer, Slice, TypeDeclaration, UnsizedArray};

make_instruction! { ConstValue->fn<const v:LuaValue>()->(o:LuaValue){ entry:{ %o=b::Move<LuaValue::TYPE>(%v); }} }
make_instruction! { ConstNil->fn()->(o:LuaValue){ entry:{ %o= lua_value::EncodeNil(b::UninitedStruct<Unit::TYPE>()); }} }
make_instruction! { EncodeBoolean->fn(i:Bool)->(o:LuaValue){ entry:{ %o=lua_value::EncodeBoolean(I64Shl(b::IntTruncate<7,0>(%i),7)); }} }
make_instruction! { ConstTrue->fn()->(o:LuaValue){ entry:{ %o= EncodeBoolean(true); }} }
make_instruction! { ConstFalse->fn()->(o:LuaValue){ entry:{ %o= EncodeBoolean(false); }} }
make_instruction! { ConstZero->fn()->(o:I64){ entry:{ %o=0; }} }
make_instruction! { ConstOne->fn()->(o:I64){ entry:{ %o=1; }} }
make_instruction! { ConstM1->fn()->(o:I64){ entry:{ %o=-1; }} }
type WriteLuaUpValueRefArray = e::WriteElement<LuaUpValueReference, UnsizedArray<LuaUpValueReference>>;
make_instruction! { ConstClosure0->fn<mut function:LuaClosureFunctionReference>(state:LuaStateReference,up_value:LuaUpValueReference)->(v:LuaValue){ entry:{
    %closure=b::AllocUnsized<LuaClosureReference::TYPE>(b::IntTruncate<12,7>(1));
    %closure_ptr=b::Deref<LuaClosureReference::TYPE>(%closure);
    b::SetLength<UnsizedArray::<LuaUpValueReference>::TYPE>(lua_closure::LocateUpValues(%closure_ptr),b::IntTruncate<12,7>(1));
    lua_closure::WriteState(%closure_ptr,b::Clone<LuaStateReference::TYPE>(%state));
    lua_closure::WriteFunction(%closure_ptr,%function);
    WriteLuaUpValueRefArray(lua_closure::LocateUpValues(%closure_ptr),b::IntTruncate<12,7>(0),b::Clone<LuaUpValueReference::TYPE>(%up_value));
    %v=lua_value::EncodeClosure(%closure);
}} }
type LuaUpValueRefSliceCopy = SliceCopy<LuaUpValueReference>;
type LuaUpValueRefSubSlice = SubSlice<LuaUpValueReference>;
type UnsizedLuaUpValueRefArrayToSlice = UnsizedArrayToSlice<LuaUpValueReference>;
make_instruction! { ConstClosure->fn<mut function:LuaClosureFunctionReference>(state:LuaStateReference,up_value:LuaUpValueReference,parent_closure:LuaClosureReference)->(v:LuaValue){ entry:{
    %parent_closure_ptr=b::Deref<LuaClosureReference::TYPE>(%parent_closure);
    %parent_len=b::GetLength<UnsizedArray::<LuaUpValueReference>::TYPE>(lua_closure::LocateUpValues(%parent_closure_ptr));
    %closure=b::AllocUnsized<LuaClosureReference::TYPE>(UsizeAdd(%parent_len,b::IntTruncate<12,7>(1)));
    %closure_ptr=b::Deref<LuaClosureReference::TYPE>(%closure);
    b::SetLength<UnsizedArray::<LuaUpValueReference>::TYPE>(lua_closure::LocateUpValues(%closure_ptr),b::IntTruncate<12,7>(1));
    lua_closure::WriteState(%closure_ptr,b::Clone<LuaStateReference::TYPE>(%state));
    lua_closure::WriteFunction(%closure_ptr,%function);
    WriteLuaUpValueRefArray(lua_closure::LocateUpValues(%closure_ptr),%parent_len,b::Clone<LuaUpValueReference::TYPE>(%up_value));
    LuaUpValueRefSliceCopy(LuaUpValueRefSubSlice(UnsizedLuaUpValueRefArrayToSlice(lua_closure::LocateUpValues(%closure_ptr)),b::IntTruncate<12,7>(0),%parent_len),UnsizedLuaUpValueRefArrayToSlice(lua_closure::LocateUpValues(%parent_closure_ptr)));
    %v=lua_value::EncodeClosure(%closure);
}} }
make_instruction! { MakeTable->fn<const shape:LuaShapeReference,const fast_len:Usize>(fields:Slice<LuaValue>)->(table_value:LuaValue){
    entry:{
        %table=b::AllocUnsized<LuaTableReference::TYPE>(%fast_len);
        %table_deref=b::Deref<LuaTableReference::TYPE>(%table);
        b::SetLength<UnsizedArray::<LuaValue>::TYPE>(lua_table::LocateFastFields(%table_deref),%fast_len);
        lua_table::WriteShape(%table_deref,%shape);
        lua_table::WriteSlowFields(%table_deref,NullableLuaValueArrayEncodeNone(b::UninitedStruct<Unit::TYPE>()));
        %fast_fields = lua_table::LocateFastFields(%table_deref);
        %copy_len=LuaValueSliceLen(%fields);
        %i=b::IntTruncate<12,7>(0);
        LuaValueSliceCopy(LuaValueSubSlice(UnsizedLuaValueArrayToSlice(%fast_fields),%i,%copy_len),%fields);
        if UsizeLe(%copy_len,%fast_len) %fill %fill_complete; },
    fill:{
       phi %i:Usize={%entry=>%i,%fill=>%i1};
       WriteLuaValueArray(%fast_fields,%i,ConstNil());
       %i1=UsizeAdd(%i,b::IntTruncate<12,7>(1));
       if UsizeLe(%i1,%fast_len) %fill %fill_complete; },
    fill_complete:{ %table_value = lua_value::EncodeTable(%table); },
}}
make_instruction! { MakeTable0->fn(state:LuaStateReference)->(table_value:LuaValue){
    entry:{
        %fast_len=b::IntTruncate<12,7>(5);
        %table=b::AllocUnsized<LuaTableReference::TYPE>(%fast_len);
        %table_deref=b::Deref<LuaTableReference::TYPE>(%table);
        b::SetLength<UnsizedArray::<LuaValue>::TYPE>(lua_table::LocateFastFields(%table_deref),%fast_len);
        lua_table::WriteShape(%table_deref,b::Clone<LuaShapeReference::TYPE>(lua_state::ReadTableShape(b::Deref<LuaStateReference::TYPE>(%state))));
        lua_table::WriteSlowFields(%table_deref,NullableLuaValueArrayEncodeNone(b::UninitedStruct<Unit::TYPE>()));
        %fast_fields = lua_table::LocateFastFields(%table_deref);
        %i1=b::IntTruncate<12,7>(0);
        if UsizeLe(%i1,%fast_len) %fill %fill_complete; },
    fill:{
       phi %i1:Usize={%entry=>%i1,%fill=>%i2};
       WriteLuaValueArray(%fast_fields,%i1,ConstNil());
       %i2=UsizeAdd(%i1,b::IntTruncate<12,7>(1));
       if UsizeLe(%i2,%fast_len) %fill %fill_complete; },
    fill_complete:{ %table_value = lua_value::EncodeTable(%table); },
}}
make_instruction! { ToBool->fn(v:LuaValue)->(o:Bool){
    entry:{ if lua_value::IsBoolean(%v) %is_bool %not_bool; },
    not_bool:{ if lua_value::IsNil(%v) %is_nil %other; },
    is_nil:{ %o=false; },
    is_bool:{ %o=I64Ne(lua_value::DecodeBooleanUnchecked(%v),0); },
    other:{ %o=true; }
} }
make_instruction! { LogicalOr->fn(lhs:LuaValue,rhs:LuaValue)->(o:LuaValue){
    entry:{ if ToBool(%lhs) %true %false; },
    true:{ %o =b::Move<LuaValue::TYPE>(%lhs); },
    false:{ %o =b::Move<LuaValue::TYPE>(%rhs); },
} }
make_instruction! { LogicalNot->fn(v:LuaValue)->(o:LuaValue){
    entry:{ %o = EncodeBoolean(BoolNot(ToBool(%v))); },
} }
type GetMetaValueCall = GetMetaValue<lua_meta_functions::ReadCall>;
type LuaValueSliceGet = SliceGet<LuaValue>;
type LuaValueArrayGet = ReadElement<LuaValue, UnsizedArray<LuaValue>>;
type LuaValueArraySet = WriteElement<LuaValue, UnsizedArray<LuaValue>>;
type LuaValueSliceSet = SliceSet<LuaValue>;
type LuaValueSliceCopy = SliceCopy<LuaValue>;
type LuaValueSubSlice = SubSlice<LuaValue>;
type LuaValueSliceLen = SliceLen<LuaValue>;
type UnsizedLuaValueArrayToSlice = UnsizedArrayToSlice<LuaValue>;
make_instruction! {
    CallFunction->fn(callable:LuaValue,args:Slice<LuaValue>)->(o:Pointer<UnsizedArray<LuaValue>>){
        entry:{ if lua_value::IsFunction(%callable) %is_function %not_function; },
        is_function:{
            %function=b::Deref<LuaFunctionReference::TYPE>(lua_value::DecodeFunctionUnchecked(%callable));
            %function_ptr=b::Read<LuaFunctionType::TYPE>(lua_function::ReadFunction(%function));
            %o=b::Call<LuaFunctionType::TYPE>(%function_ptr,lua_function::ReadState(%function),%args);
        },
        not_function:{ if lua_value::IsClosure(%callable) %is_closure %not_closure; },
        is_closure:{
            %closure=b::Deref<LuaClosureReference::TYPE>(lua_value::DecodeClosureUnchecked(%callable));
            %function_ptr=b::Read<LuaClosureFunctionType::TYPE>(lua_closure::ReadFunction(%closure));
            %o=b::Call<LuaClosureFunctionType::TYPE>(%function_ptr,lua_closure::ReadState(%closure),lua_value::DecodeClosureUnchecked(%callable),%args);
        },
        not_closure:{ if lua_value::IsFunction(%callable) %is_object %other; },
        is_object:{
            %meta=GetMetaValueCall(%callable);
            if lua_value::IsClosure(%meta) %meta_closure %other;
        },
        meta_closure:{
            %new_slice=UnsizedLuaValueArrayToSlice(b::StackAllocUnsized<UnsizedArray::<LuaValue>::TYPE>(UsizeAdd(b::IntTruncate<12,7>(1),LuaValueSliceLen(%args))));
            LuaValueSliceCopy(LuaValueSubSlice(%new_slice,b::IntTruncate<12,7>(1),LuaValueSliceLen(%args)),%args);
            LuaValueSliceSet(%new_slice,b::IntTruncate<12,7>(0),%callable);
            %closure=b::Deref<LuaClosureReference::TYPE>(lua_value::DecodeClosureUnchecked(%callable));
            %function_ptr=b::Read<LuaClosureFunctionType::TYPE>(lua_closure::ReadFunction(%closure));
            %o=b::Call<LuaClosureFunctionType::TYPE>(%function_ptr,lua_closure::ReadState(%closure),lua_value::DecodeClosureUnchecked(%callable),%new_slice);
        },
        other:{
            %array=b::StackAllocUnsized<UnsizedArray::<LuaValue>::TYPE>(b::IntTruncate<12,7>(0));
            b::SetLength<UnsizedArray::<LuaValue>::TYPE>(%array,b::IntTruncate<12,7>(0));
            %o=b::Move<Pointer::<UnsizedArray<LuaValue>>::TYPE>(%array);
            ThrowError();},
    }
}
make_instruction! {
    CallFunctionVA->fn(callable:LuaValue,args:Slice<LuaValue>,va_args:Pointer<UnsizedArray<LuaValue>>)->(o:Pointer<UnsizedArray<LuaValue>>){entry:{
        %o=CallFunctionVaSlice(%callable,%args,UnsizedLuaValueArrayToSlice(%va_args));
}}}
make_instruction! {
    CallFunctionVaSlice->fn(callable:LuaValue,args:Slice<LuaValue>,va_args:Slice<LuaValue>)->(o:Pointer<UnsizedArray<LuaValue>>){
        entry:{ if lua_value::IsFunction(%callable) %is_function %not_function; },
        is_function:{
            %new_slice=UnsizedLuaValueArrayToSlice(b::StackAllocUnsized<UnsizedArray::<LuaValue>::TYPE>(UsizeAdd(LuaValueSliceLen(%args),LuaValueSliceLen(%va_args))));
            LuaValueSliceCopy(LuaValueSubSlice(%new_slice,LuaValueSliceLen(%args),LuaValueSliceLen(%va_args)),%va_args);
            LuaValueSliceCopy(LuaValueSubSlice(%new_slice,b::IntTruncate<12,7>(0),LuaValueSliceLen(%args)),%args);
            %function=b::Deref<LuaFunctionReference::TYPE>(lua_value::DecodeFunctionUnchecked(%callable));
            %function_ptr=b::Read<LuaFunctionType::TYPE>(lua_function::ReadFunction(%function));
            %o=b::Call<LuaFunctionType::TYPE>(%function_ptr,lua_function::ReadState(%function),%new_slice);
        },
        not_function:{ if lua_value::IsClosure(%callable) %is_closure %not_closure; },
        is_closure:{
            %new_slice=UnsizedLuaValueArrayToSlice(b::StackAllocUnsized<UnsizedArray::<LuaValue>::TYPE>(UsizeAdd(LuaValueSliceLen(%args),LuaValueSliceLen(%va_args))));
            LuaValueSliceCopy(LuaValueSubSlice(%new_slice,LuaValueSliceLen(%args),LuaValueSliceLen(%args)),%va_args);
            LuaValueSliceCopy(LuaValueSubSlice(%new_slice,b::IntTruncate<12,7>(0),LuaValueSliceLen(%args)),%args);
            %closure=b::Deref<LuaClosureReference::TYPE>(lua_value::DecodeClosureUnchecked(%callable));
            %function_ptr=b::Read<LuaClosureFunctionType::TYPE>(lua_closure::ReadFunction(%closure));
            %o=b::Call<LuaClosureFunctionType::TYPE>(%function_ptr,lua_closure::ReadState(%closure),lua_value::DecodeClosureUnchecked(%callable),%new_slice);
        },
        not_closure:{ if lua_value::IsFunction(%callable) %is_object %other; },
        is_object:{
            %meta=GetMetaValueCall(%callable);
            if lua_value::IsClosure(%meta) %meta_closure %other;
        },
        meta_closure:{
            %new_slice=UnsizedLuaValueArrayToSlice(b::StackAllocUnsized<UnsizedArray::<LuaValue>::TYPE>(UsizeAdd(b::IntTruncate<12,7>(1),UsizeAdd(LuaValueSliceLen(%args),LuaValueSliceLen(%va_args)))));
            LuaValueSliceSet(%new_slice,b::IntTruncate<12,7>(0),%callable);
            LuaValueSliceCopy(LuaValueSubSlice(%new_slice,UsizeAdd(b::IntTruncate<12,7>(1),LuaValueSliceLen(%args)),LuaValueSliceLen(%args)),%va_args);
            LuaValueSliceCopy(LuaValueSubSlice(%new_slice,b::IntTruncate<12,7>(1),LuaValueSliceLen(%args)),%args);
            %closure=b::Deref<LuaClosureReference::TYPE>(lua_value::DecodeClosureUnchecked(%callable));
            %function_ptr=b::Read<LuaClosureFunctionType::TYPE>(lua_closure::ReadFunction(%closure));
            %o=b::Call<LuaClosureFunctionType::TYPE>(%function_ptr,lua_closure::ReadState(%closure),lua_value::DecodeClosureUnchecked(%callable),%new_slice);
        },
        other:{
            %array=b::StackAllocUnsized<UnsizedArray::<LuaValue>::TYPE>(b::IntTruncate<12,7>(0));
            b::SetLength<UnsizedArray::<LuaValue>::TYPE>(%array,b::IntTruncate<12,7>(0));
            %o=b::Move<Pointer::<UnsizedArray<LuaValue>>::TYPE>(%array);
            ThrowError();},
    }
}
make_instruction! {
    CallFunctionRet1->fn(callable:LuaValue,args:Slice<LuaValue>)->(o:LuaValue){ entry:{
        %o=GetRet0(CallFunction(%callable,%args));
}}}
make_instruction! {
    CallFunctionVaSliceRet1->fn(callable:LuaValue,args:Slice<LuaValue>,va_args:Slice<LuaValue>)->(o:LuaValue){ entry:{
        %o=GetRet0(CallFunctionVaSlice(%callable,%args,%va_args));
}}}
make_instruction! {
    CallFunctionVaRet1->fn(callable:LuaValue,args:Slice<LuaValue>,va_args:Pointer<UnsizedArray<LuaValue>>)->(o:LuaValue){ entry:{
        %o=GetRet0(CallFunctionVA(%callable,%args,%va_args));
}}}
make_instruction! { CallFunction0->fn(callable:LuaValue)->(r:Pointer<UnsizedArray<LuaValue>>){ entry:{
        %r=CallFunction(%callable,MakeSlice<LuaValue::TYPE,0>());
}}}
make_instruction! { CallFunction1->fn(callable:LuaValue,arg1:LuaValue)->(r:Pointer<UnsizedArray<LuaValue>>){ entry:{
        %r=CallFunction(%callable,MakeSlice<LuaValue::TYPE,1>(%arg1));
}}}
make_instruction! { CallFunction2->fn(callable:LuaValue,arg1:LuaValue,arg2:LuaValue)->(r:Pointer<UnsizedArray<LuaValue>>){ entry:{
        %r=CallFunction(%callable,MakeSlice<LuaValue::TYPE,2>(%arg1,%arg2));
}}}
make_instruction! { CallFunction3->fn(callable:LuaValue,arg1:LuaValue,arg2:LuaValue,arg3:LuaValue)->(r:Pointer<UnsizedArray<LuaValue>>){ entry:{
        %r=CallFunction(%callable,MakeSlice<LuaValue::TYPE,3>(%arg1,%arg2,%arg3));
}}}
make_instruction! { CallFunction0VaSlice->fn(callable:LuaValue,va_args:Slice<LuaValue>)->(r:Pointer<UnsizedArray<LuaValue>>){ entry:{
        %r=CallFunctionVaSlice(%callable,MakeSlice<LuaValue::TYPE,0>(),%va_args);
}}}
make_instruction! { CallFunction1VaSlice->fn(callable:LuaValue,arg1:LuaValue,va_args:Slice<LuaValue>)->(r:Pointer<UnsizedArray<LuaValue>>){ entry:{
        %r=CallFunctionVaSlice(%callable,MakeSlice<LuaValue::TYPE,1>(%arg1),%va_args);
}}}
make_instruction! { CallFunction2VaSlice->fn(callable:LuaValue,arg1:LuaValue,arg2:LuaValue,va_args:Slice<LuaValue>)->(r:Pointer<UnsizedArray<LuaValue>>){ entry:{
        %r=CallFunctionVaSlice(%callable,MakeSlice<LuaValue::TYPE,2>(%arg1,%arg2),%va_args);
}}}
make_instruction! { CallFunction3VaSlice->fn(callable:LuaValue,arg1:LuaValue,arg2:LuaValue,arg3:LuaValue,va_args:Slice<LuaValue>)->(r:Pointer<UnsizedArray<LuaValue>>){ entry:{
        %r=CallFunctionVaSlice(%callable,MakeSlice<LuaValue::TYPE,3>(%arg1,%arg2,%arg3),%va_args);
}}}
make_instruction! { CallFunction0VA->fn(callable:LuaValue,va_args:Pointer<UnsizedArray<LuaValue>>)->(r:Pointer<UnsizedArray<LuaValue>>){ entry:{
        %r=CallFunctionVA(%callable,MakeSlice<LuaValue::TYPE,0>(),%va_args);
}}}
make_instruction! { CallFunction1VA->fn(callable:LuaValue,arg1:LuaValue,va_args:Pointer<UnsizedArray<LuaValue>>)->(r:Pointer<UnsizedArray<LuaValue>>){ entry:{
        %r=CallFunctionVA(%callable,MakeSlice<LuaValue::TYPE,1>(%arg1),%va_args);
}}}
make_instruction! { CallFunction2VA->fn(callable:LuaValue,arg1:LuaValue,arg2:LuaValue,va_args:Pointer<UnsizedArray<LuaValue>>)->(r:Pointer<UnsizedArray<LuaValue>>){ entry:{
        %r=CallFunctionVA(%callable,MakeSlice<LuaValue::TYPE,2>(%arg1,%arg2),%va_args);
}}}
make_instruction! { CallFunction3VA->fn(callable:LuaValue,arg1:LuaValue,arg2:LuaValue,arg3:LuaValue,va_args:Pointer<UnsizedArray<LuaValue>>)->(r:Pointer<UnsizedArray<LuaValue>>){ entry:{
        %r=CallFunctionVA(%callable,MakeSlice<LuaValue::TYPE,3>(%arg1,%arg2,%arg3),%va_args);
}}}
make_instruction! { CallFunction0Ret1->fn(callable:LuaValue)->(r:LuaValue){ entry:{
        %r=GetRet0(CallFunction0(%callable));
}}}
make_instruction! { CallFunction1Ret1->fn(callable:LuaValue,arg1:LuaValue)->(r:LuaValue){ entry:{
        %r=GetRet0(CallFunction1(%callable,%arg1));
}}}
make_instruction! { CallFunction2Ret1->fn(callable:LuaValue,arg1:LuaValue,arg2:LuaValue)->(r:LuaValue){ entry:{
        %r=GetRet0(CallFunction2(%callable,%arg1,%arg2));
}}}
make_instruction! { CallFunction3Ret1->fn(callable:LuaValue,arg1:LuaValue,arg2:LuaValue,arg3:LuaValue)->(r:LuaValue){ entry:{
        %r=GetRet0(CallFunction3(%callable,%arg1,%arg2,%arg3));
}}}
make_instruction! { CallFunction0VaSliceRet1->fn(callable:LuaValue,va_args:Slice<LuaValue>)->(r: LuaValue){ entry:{
        %r=GetRet0(CallFunctionVaSlice(%callable,MakeSlice<LuaValue::TYPE,0>(),%va_args));
}}}
make_instruction! { CallFunction1VaSliceRet1->fn(callable:LuaValue,arg1:LuaValue,va_args:Slice<LuaValue>)->(r: LuaValue){ entry:{
        %r=GetRet0(CallFunctionVaSlice(%callable,MakeSlice<LuaValue::TYPE,1>(%arg1),%va_args));
}}}
make_instruction! { CallFunction2VaSliceRet1->fn(callable:LuaValue,arg1:LuaValue,arg2:LuaValue,va_args:Slice<LuaValue>)->(r: LuaValue){ entry:{
        %r=GetRet0(CallFunctionVaSlice(%callable,MakeSlice<LuaValue::TYPE,2>(%arg1,%arg2),%va_args));
}}}
make_instruction! { CallFunction3VaSliceRet1->fn(callable:LuaValue,arg1:LuaValue,arg2:LuaValue,arg3:LuaValue,va_args:Slice<LuaValue>)->(r: LuaValue){ entry:{
        %r=GetRet0(CallFunctionVaSlice(%callable,MakeSlice<LuaValue::TYPE,3>(%arg1,%arg2,%arg3),%va_args));
}}}
make_instruction! { CallFunction0VaRet1->fn(callable:LuaValue,va_args:Pointer<UnsizedArray<LuaValue>>)->(r: LuaValue){ entry:{
        %r=GetRet0(CallFunctionVA(%callable,MakeSlice<LuaValue::TYPE,0>(),%va_args));
}}}
make_instruction! { CallFunction1VaRet1->fn(callable:LuaValue,arg1:LuaValue,va_args:Pointer<UnsizedArray<LuaValue>>)->(r: LuaValue){ entry:{
        %r=GetRet0(CallFunctionVA(%callable,MakeSlice<LuaValue::TYPE,1>(%arg1),%va_args));
}}}
make_instruction! { CallFunction2VaRet1->fn(callable:LuaValue,arg1:LuaValue,arg2:LuaValue,va_args:Pointer<UnsizedArray<LuaValue>>)->(r: LuaValue){ entry:{
        %r=GetRet0(CallFunctionVA(%callable,MakeSlice<LuaValue::TYPE,2>(%arg1,%arg2),%va_args));
}}}
make_instruction! { CallFunction3VaRet1->fn(callable:LuaValue,arg1:LuaValue,arg2:LuaValue,arg3:LuaValue,va_args:Pointer<UnsizedArray<LuaValue>>)->(r: LuaValue){ entry:{
        %r=GetRet0(CallFunctionVA(%callable,MakeSlice<LuaValue::TYPE,3>(%arg1,%arg2,%arg3),%va_args));
}}}
type NullableLuaValuePointerDecodeSome = nullable_option::DecodeSomeUnchecked<Pointer<LuaValue>>;
type ReadUpRefs = e::ReadElement<NullablePointer<LuaValue>, UnsizedArray<NullablePointer<LuaValue>>>;
type WriteUpRefs = e::WriteElement<NullablePointer<LuaValue>, UnsizedArray<NullablePointer<LuaValue>>>;
make_instruction! {LocateUpVariable->fn<const tire:Usize,const index:Usize>(closure:LuaClosureReference)->(o:Pointer<LuaValue>){entry:{
    %o=NullableLuaValuePointerDecodeSome(ReadUpRefs(lua_up_value::LocatePointers(b::Deref<LuaUpValueReference::TYPE>(b::Read<LuaUpValueReference::TYPE>(b::LocateElement<UnsizedArray::<LuaUpValueReference>::TYPE>(lua_closure::LocateUpValues(b::Deref<LuaClosureReference::TYPE>(%closure)),%tire)))),%index));
}}}
make_instruction! {GetUpVariable->fn<const tire:Usize,const index:Usize>(closure:LuaClosureReference)->(o:LuaValue){entry:{
    %o=Read<LuaValue::TYPE>(LocateUpVariable<%tire,%index>(%closure));
}}}
make_instruction! {SetUpVariable->fn<const tire:Usize,const index:Usize>(closure:LuaClosureReference,value:LuaValue){entry:{
    Write<LuaValue::TYPE>(LocateUpVariable<%tire,%index>(%closure),%value);
}}}
type NullableLuaValuePointerEncodeSome = nullable_pointer::EncodeSome<LuaValue>;
make_instruction! {SetUpValue->fn<const index:Usize>(up_value:LuaUpValueReference,value:LuaValue){entry:{
    %up_value_ptr=b::Deref<LuaUpValueReference::TYPE>(%up_value);
    %owned=b::LocateElement<UnsizedArray::<LuaValue>::TYPE>(NullableLuaValueArrayDecodeSome(lua_up_value::ReadOwned(%up_value_ptr)),%index);
    Write<LuaValue::TYPE>(%owned,%value);
    WriteUpRefs(lua_up_value::LocatePointers(%up_value_ptr),%index,NullableLuaValuePointerEncodeSome(%owned));
}}}
make_instruction! {SetUpRef->fn<const index:Usize>(up_value:LuaUpValueReference,value:LuaValue){entry:{
    %up_value_ptr=b::Deref<LuaUpValueReference::TYPE>(%up_value);
    WriteUpRefs(lua_up_value::LocatePointers(%up_value_ptr),%index,NullableLuaValuePointerEncodeSome(b::GetPointer<LuaValue::TYPE>(%value)));
}}}
type NullableLuaValueArrayEncodeNone = nullable_pointer::EncodeNone<UnsizedArray<LuaValue>>;
type NullableLuaValueEncodeNone = nullable_pointer::EncodeNone<LuaValue>;
make_instruction! {
    NewUpValue->fn<const len:Usize>()->(v:LuaUpValueReference){
        entry:{
            %upvalue=b::AllocUnsized<LuaUpValueReference::TYPE>(%len);
            %upvalue_deref=b::Deref<LuaUpValueReference::TYPE>(%upvalue);
            lua_up_value::WriteOwned(%upvalue_deref,NullableLuaValueArrayEncodeNone(b::UninitedStruct<Unit::TYPE>()));
            %i=b::IntTruncate<12,7>(0);
            if UsizeLarge(%len,%i) %loop %end;
        },
        loop:{
            phi %i:Usize={%entry=>%i,%loop=>%i1};
            %i1=UsizeAdd(%i,b::IntTruncate<12,7>(1));
            Write<NullablePointer::<LuaValue>::TYPE>(b::LocateElement<UnsizedArray::<NullablePointer<LuaValue>>::TYPE>(lua_up_value::LocatePointers(%upvalue_deref),%i),NullableLuaValueEncodeNone(b::UninitedStruct<Unit::TYPE>()));
            if UsizeLarge(%len,%i1) %loop %end;
        },
        end:{
            %v=b::Move<LuaUpValueReference::TYPE>(%upvalue);
        },
    }
}
make_instruction! {
    NewClosure->fn(closure:LuaClosureReference,up_value:LuaUpValueReference)->(v:LuaValue){
        entry:{
            e::TODO();
        },
    }
}
#[make_native_function(ThrowError)]
pub extern "C" fn __vm_lua_lib_throw_error() {
    panic!("lua throw error");
}
#[make_native_function(IllegalInstruction)]
pub extern "C" fn __vm_lua_lib_illegal_instruction() {
    panic!("illegal instruction 0");
}
#[derive(Instruction)]
#[instruction(GetMetaValue->fn(v:LuaValue)->(o:LuaValue){
        entry:{ if lua_value::IsTable(%v) %table %not_table; },
        table:{ %o= ReadMetaFunction(b::Deref<LuaMetaFunctionsReference::TYPE>(lua_shape::ReadMetaFunctions(b::Deref<LuaShapeReference::TYPE>(lua_table::ReadShape(b::Deref<LuaTableReference::TYPE>(lua_value::DecodeTableUnchecked(%v))))))); },
        not_table:{ if lua_value::IsString(%v) %string %not_string; },
        string:{ %o= ReadMetaFunction(b::Deref<LuaMetaFunctionsReference::TYPE>(lua_state::ReadStringMetaFunctions(b::Deref<LuaStateReference::TYPE>(lua_string::ReadLuaState(b::Deref<LuaStringReference::TYPE>(lua_value::DecodeStringUnchecked(%v))))))); },
        not_string:{ %o= ConstNil(); },
})]
pub struct GetMetaValue<ReadMetaFunction: Instruction>(PhantomData<ReadMetaFunction>);
make_instruction! { I64ToValue->fn(i:I64)->(v:LuaValue){
  entry:{
    if I64Eq(%i,I64Shr(I64Shl(%i,4),4)) %small %big;
  },
  small:{ %v=lua_value::EncodeInteger(I64Shl(%i,4)); },
  big:{
    %heap_object=b::AllocSized<LuaI64Reference::TYPE>();
    lua_i64::WriteValue(b::Deref<LuaI64Reference::TYPE>(%heap_object),%i);
    %v=lua_value::EncodeBigInt(%heap_object);
  },
} }
make_instruction! { GetIntegerValue->fn(value:LuaValue)->(int:I64){
    entry:{ if lua_value::IsInteger(%value) %small %large; },
    small:{ %int = I64Shr(lua_value::DecodeIntegerUnchecked(%value),4); },
    large:{ %int = lua_i64::ReadValue(b::Deref<LuaI64Reference::TYPE>(lua_value::DecodeBigIntUnchecked(%value))); }
} }
make_instruction! {
    F64ToValue->fn(f:F64)->(v:LuaValue){
      entry:{
        %i = e::F64AsI64(%f);
        %c=I64And(%i,15);
        if I64Eq(%c,0) %small %big;
      },
      small:{
        %v=lua_value::EncodeFloat(%i);
      },
      big:{
        %heap_object=b::AllocSized<LuaF64Reference::TYPE>();
        lua_f64::WriteValue(b::Deref<LuaF64Reference::TYPE>(%heap_object),%f);
        %v=lua_value::EncodeBigFloat(%heap_object);
      },
    }
}
make_instruction! {IsInteger->fn(i:LuaValue)->(o:Bool){entry:{
    %o=UsizeLt(lua_value::GetTag(%i),b::IntTruncate<12,7>(2));
}}}
make_instruction! {IsFloat->fn(i:LuaValue)->(o:Bool){entry:{
    %o=UsizeLt(UsizeSub(lua_value::GetTag(%i),b::IntTruncate<12,7>(2)),b::IntTruncate<12,7>(2));
}}}
make_instruction! { GetFloatValue->fn(value:LuaValue)->(float:F64){
    entry:{ if lua_value::IsFloat(%value) %small %large; },
    small:{
        %i = lua_value::DecodeFloatUnchecked(%value);
        %float = I64AsF64(%i); },
    large:{ %float = lua_f64::ReadValue(b::Deref<LuaF64Reference::TYPE>(lua_value::DecodeBigFloatUnchecked(%value))); }
} }
make_instruction! {ToFloat->fn(i:LuaValue)->(o:F64){
    entry:{ if IsFloat(%i) %float %not_float; },
      float:{%o=GetFloatValue(%i);},
      not_float:{if IsInteger(%i) %integer %not_integer},
        integer:{%o=I64ToF64(GetIntegerValue(%i));},
        not_integer:{%o=0.0;ThrowError();},
}}
make_instruction! {ValueToFloatValue->fn(v:LuaValue)->(v:LuaValue){entry:{
    %v=F64ToValue(ToFloat(%v));
}}}
#[make_native_function(RawFPow)]
pub extern "C" fn __vm_lua_lib_f64_pow(arg1: F64, arg2: F64) -> F64 { F64(f64::powf(arg1.0, arg2.0)) }
make_instruction! {FPow->fn(f1:F64,f2:F64)->(f2:F64){entry:{
    %f2=RawFPow(%f1,%f2);
}}}
#[make_native_function(RawFDivFloor)]
pub extern "C" fn __vm_lua_lib_f64_div_floor(arg1: F64, arg2: F64) -> F64 { F64((arg1.0 / arg2.0).floor()) }
make_instruction! {FDivFloor->fn(f1:F64,f2:F64)->(f2:F64){entry:{
    %f2=RawFDivFloor(%f1,%f2);
}}}

type NoneLuaValueArray = e::nullable_pointer::EncodeNone<UnsizedArray<LuaValue>>;
make_instruction! { BuildTable->fn<const shape:LuaShapeReference,const slots:Usize>()->(o:LuaValue){ entry:{
    %new_table=b::AllocUnsized<LuaTableReference::TYPE>(%slots);
    %new_table_ptr=b::Deref<LuaTableReference::TYPE>(%shape);
    lua_table::WriteSlowFields(%new_table_ptr,NoneLuaValueArray(b::UninitedStruct<Unit::TYPE>()));
    lua_table::WriteShape(%new_table_ptr,%shape);
    %o=lua_value::EncodeTable(%new_table);
}} }
pub unsafe fn extend_to_buffer(buffer: &mut Vec<u8>, mut i: Direct<LuaValue>) -> bool {
    if let Some(v) = i.read_integer() {
        let v = (v.0) >> 4;
        buffer.extend(v.to_string().as_bytes());
    } else if let Some(v) = i.read_big_int() {
        let v = v.as_ref().get_value().0;
        buffer.extend(v.to_string().as_bytes());
    } else if let Some(v) = i.read_float() {
        let v = v.0;
        let v = f64::from_le_bytes(i64::to_le_bytes(v));
        buffer.extend(v.to_string().as_bytes());
    } else if let Some(v) = i.read_big_float() {
        let v = v.as_ref().get_value().0;
        buffer.extend(v.to_string().as_bytes());
    } else if let Some(v) = i.read_string() {
        buffer.extend(v.as_ref().ref_data().as_slice().iter().map(|d| d.0));
    } else if let Some(_v) = i.read_nil() {
        buffer.extend(b"nil");
    } else if let Some(v) = i.read_boolean() {
        let v = v.0 != 0;
        buffer.extend(v.to_string().as_bytes());
    } else if let Some(v) = i.read_table() {
        buffer.extend(format!("table: {:p}", v.as_ptr()).bytes());
    } else if let Some(v) = i.read_function() {
        buffer.extend(format!("function: {:p}", v.as_ptr()).bytes());
    } else if let Some(v) = i.read_closure() {
        buffer.extend(format!("function: {:p}", v.as_ptr()).bytes());
    } else {
        error!("invalid lua value: {:X?}", &i.0 .0);
        return false;
    }
    true
}
#[make_native_function(RawConcat)]
pub unsafe extern "C" fn __vm_lua_lib_raw_concat(
    state: Direct<LuaStateReference>,
    i1: Direct<LuaValue>,
    i2: Direct<LuaValue>,
) -> Direct<LuaValue> {
    let mut buffer = Vec::new();
    if !extend_to_buffer(&mut buffer, i1) || !extend_to_buffer(&mut buffer, i2) {
        return Direct(LuaValueImpl::encode_nil(()));
    }
    Direct(crate::new_string(state.0, &*buffer).unwrap())
}
type GetMetaValueConcat = GetMetaValue<lua_meta_functions::ReadConcat>;
make_instruction! {
    Concat->fn(state:LuaStateReference,i1:LuaValue,i2:LuaValue)->(i2:LuaValue){
        entry:{ if BoolOr(lua_value::IsTable(%i1),lua_value::IsTable(%i2)) %use_metatable %dont_use_meratable; },
          use_metatable:{
              %i1_meta_function = GetMetaValueConcat(%i1);
              if lua_value::IsNil(%i1_meta_function) %i1_has_no_meta_function %i1_has_meta_function; },
            i1_has_meta_function:{ %o=CallFunction2Ret1(%i1_meta_function,%i1,%i2); },
            i1_has_no_meta_function:{
                %i2_meta_function = GetMetaValueConcat(%i2);
                if lua_value::IsNil(%i2_meta_function) %i2_has_no_meta_function %i2_has_meta_function; },
            i2_has_meta_function:{ %i2=CallFunction2Ret1(%i2_meta_function,%i1,%i2); },
            i2_has_no_meta_function:{%i2=ConstNil();ThrowError();},
          dont_use_meratable:{ %i2=RawConcat(%state,%i1,%i2); },
    }
}
make_instruction! {
    Length->fn(i1:LuaValue)->(i1:LuaValue){
        entry:{ if lua_value::IsString(%i1) %string %not_string; },
        string:{ %v=I64ToValue(b::IntTruncate<7,12>(GetLength<UnsizedArray::<U8>::TYPE>(lua_string::LocateData(b::Deref<LuaStringReference::TYPE>(lua_value::DecodeStringUnchecked(%i1)))))); },
        not_string:{ if lua_value::IsTable(%i1) %table %error; },
        table:{
            %meta_function=lua_meta_functions::ReadLen(b::Deref<LuaMetaFunctionsReference::TYPE>(lua_shape::ReadMetaFunctions(b::Deref<LuaShapeReference::TYPE>(lua_table::ReadShape(b::Deref<LuaTableReference::TYPE>(lua_value::DecodeTableUnchecked(%i1)))))));
            if lua_value::IsNil(%meta_function) %use_int_keys %use_meta_function;
        },
        use_meta_function:{%i1=CallFunction1Ret1(%meta_function,%i1);},
        use_int_keys:{%i1=I64ToValue(b::IntTruncate<7,8>(lua_shape::ReadMaxIntIndex(b::Deref<LuaShapeReference::TYPE>(lua_table::ReadShape(b::Deref<LuaTableReference::TYPE>(lua_value::DecodeTableUnchecked(%i1)))))));},
        error:{%i2=ConstNil();ThrowError();}
    }
}
make_instruction! {
    BranchIf->fn<block then,block else>(i:LuaValue){ entry:{
        b::BranchIf<%then,%else>(ToBool(%i));
    }}
}
make_instruction! {
    ForInLoopJump1->fn<block loop,block break>(iter:LuaValue,iterable:LuaValue,state:LuaValue)->(state:LuaValue){ entry:{
        %new_state = CallFunction2Ret1(%iter,%iterable,%state);
        %state = b::Move<LuaValue::TYPE>(%new_state);
        if lua_value::IsNil(%new_state) %loop %break;
    }}
}
make_instruction! {
    ForInLoopJump2->fn<block loop,block break>(iter:LuaValue,iterable:LuaValue,state:LuaValue)->(state:LuaValue,ret1:LuaValue){ entry:{
        %rets = CallFunction2(%iter,%iterable,%state);
        %new_state = GetRet0(%rets);
        %state = b::Move<LuaValue::TYPE>(%new_state);
        %ret1 = DoGetRet(%rets,b::IntTruncate<12,7>(1));
        if lua_value::IsNil(%new_state) %loop %break;
    }}
}
make_instruction! {
    ForInLoopJump->fn<block loop,block break>(iter:LuaValue,iterable:LuaValue,state:LuaValue)->(state:LuaValue,rets:Pointer<UnsizedArray<LuaValue>>){ entry:{
        %rets = CallFunction2(%iter,%iterable,%state);
        %new_state = GetRet0(%rets);
        %state = b::Move<LuaValue::TYPE>(%new_state);
        if lua_value::IsNil(%new_state) %loop %break;
    }}
}
make_instruction! {
    IForLoopJump->fn<block loop,block break>(end:I64,state:I64)->(int_state:I64){ entry:{
        if I64Le(%state,%end) %loop %break;
    }}
}
make_instruction! {
    FForLoopJump->fn<block loop,block break>(end:F64,state:F64)->(int_state:I64){ entry:{
        if F64Le(%state,%end) %loop %break;
    }}
}
make_instruction! { ForLoopInit->fn<block predict>(start:LuaValue,end:LuaValue)->(end:LuaValue,state:LuaValue){
    entry:{ if BoolOr(IsInteger(%start),IsInteger(%end)) %use_int %use_float; },
    use_int:{ %state=MoveValue(%start); %end=MoveValue(%end); branch %predict;},
    use_float:{ %state=ValueToFloatValue(%start); %end=ValueToFloatValue(%end); branch %predict; },
}}
make_instruction! { ForLoopJump->fn<block loop,block break>(end:LuaValue,state:LuaValue)->(state:LuaValue){
    entry:{ if IsInteger(%state) %use_int %use_float; },
    use_int:{if I64Le(GetIntegerValue(%state),GetIntegerValue(%end)) %loop %break;},
    use_float:{if F64Le(ToFloat(%state),ToFloat(%end)) %loop %break;},
}}
make_instruction! {ForLoopIncrease->fn<block predict>(v:LuaValue)->(v:LuaValue){
    entry:{ if IsInteger(%v) %use_int %use_float; },
    use_int:{ %v=I64ToValue(I64Add(GetIntegerValue(%v),1)); branch %predict; },
    use_float:{ %v=F64ToValue(F64Add(ToFloat(%v),1.0)); branch %predict; },
}}
make_instruction! { ForStepLoopInit->fn<block predict>(start:LuaValue,end:LuaValue,step:LuaValue)->(end:LuaValue,step:LuaValue,state:LuaValue){
    entry:{
        if F64Eq(0.0,ToFloat(%step)) %invalid %valid;},
    invalid:{%state=ConstNil();ThrowError();branch %predict;},
    valid:{ if BoolAnd(BoolAnd(IsInteger(%start),IsInteger(%end)),IsInteger(%step)) %use_int %use_float; },
    use_int:{ %state=MoveValue(%start); branch %predict;},
    use_float:{
        %state=ValueToFloatValue(%start);
        %end=ValueToFloatValue(%end);
        %step=ValueToFloatValue(%step);
        branch %predict; },
}}
make_instruction! { ForStepLoopJump->fn<block loop,block break>(end:LuaValue,step:LuaValue,state:LuaValue)->(state:LuaValue){
    entry:{
        if IsInteger(%state) %use_int %use_float; },
    use_int:{if I64Ge(GetIntegerValue(%step),0) %use_int_pos %use_int_neg;},
    use_int_pos:{if I64Le(GetIntegerValue(%state),GetIntegerValue(%end)) %loop %break;},
    use_int_neg:{if I64Ge(GetIntegerValue(%state),GetIntegerValue(%end)) %loop %break;},
    use_float:{
        if F64Gt(ToFloat(%step),0.0) %use_float_pos %use_float_neg;},
    use_float_pos:{
        if F64Le(ToFloat(%state),ToFloat(%end)) %loop %break;},
    use_float_neg:{if F64Ge(ToFloat(%state),ToFloat(%end)) %loop %break;},
}}
make_instruction! {ForStepLoopIncrease->fn<block predict>(v:LuaValue,step:LuaValue)->(v:LuaValue){
    entry:{
        if IsInteger(%v) %use_int %use_float; },
    use_int:{
        %v=I64ToValue(I64Add(GetIntegerValue(%v),GetIntegerValue(%step)));
        branch %predict; },
    use_float:{
        %v=F64ToValue(F64Add(GetFloatValue(%v),GetFloatValue(%step)));
        branch %predict; },
}}
#[make_native_function(F64Floor)]
pub unsafe extern "C" fn __vm_lua_lib_f64_floor(i: F64) -> F64 { F64(i.0.floor()) }
make_instruction! { F64ToI64->fn(f:F64)->(o:I64){
    entry:{
        %floor=F64Floor(%f);
        if F64Eq(%floor,%f) %is_int %not_int; },
    is_int:{%o=e::F64ToI64(%f);},
    not_int:{%o=0;ThrowError();},
}}
#[derive(Instruction)]
#[instruction(
    UniqueInstruction->{(i1:LuaValue)->(i1:LuaValue){
        None:{
            entry:{
                %i1_tag=lua_value::GetTag(%i1);
                if UsizeLt(%i1_tag,b::IntTruncate<12,7>(4)) %number %not_number; },
              number:{ if UsizeLt(%i1_tag,b::IntTruncate<12,7>(2)) %integer %not_integer; },
                integer:{
                    %i1_integer_value=GetIntegerValue(%i1);
                    SetState<%Integer>();
                    %i1=IntegerInstruction(%i1_integer_value); },
                not_integer:{
                    %i1_float_value=GetFloatValue(%i1);
                    SetState<%Float>();
                    %i1=FloatInstruction(%i1_float_value); },
              not_number:{
                  %i1_meta_function = GetMetaFunction(%i1);
                  if lua_value::IsNil(%i1_meta_function) %i1_has_no_meta_function %i1_has_meta_function; },
                i1_has_meta_function:{
                    SetState<%UseMetaMethodOfI1>();
                    %i1=CallFunction1Ret1(%i1_meta_function,%i1); },
                i1_has_no_meta_function:{ %i1=ConstNil();ThrowError(); },
        },
        Integer:{
            entry:{
                %i1_tag=lua_value::GetTag(%i1);
                if UsizeLt(%i1_tag,b::IntTruncate<12,7>(2)) %integer %other; },
            integer:{
                %i1_integer_value=GetIntegerValue(%i1);
                %i1=IntegerInstruction(%i1_integer_value); },
            other:{  %i1=CallState<%None>(%i1); },
        },
        Float:{
            entry:{
                %i1_tag=lua_value::GetTag(%i1);
                if UsizeLarge(b::IntTruncate<12,7>(2),UsizeSub(%i1_tag,b::IntTruncate<12,7>(2))) %float %other; },
            float:{
                %i1_float_value=GetFloatValue(%i1);
                %i1 = FloatInstruction(%i1_float_value); },
            other:{  %i1=CallState<%None>(%i1); },
        },
        UseMetaMethodOfI1:{
            entry:{
                %i1_meta_function = GetMetaFunction(%i1);
                if lua_value::IsNil(%i1_meta_function) %other %i1_has_meta_function; },
            i1_has_meta_function:{
                SetState<%UseMetaMethodOfI1>();
                %i1=CallFunction1Ret1(%i1_meta_function,%i1); },
            other:{ SetState<%None>(); },
        },
    }}
    )]
pub struct UniqueInstruction<
    IntegerInstruction: Instruction,
    FloatInstruction: Instruction,
    GetMetaFunction: Instruction,
>(PhantomData<(IntegerInstruction, FloatInstruction, GetMetaFunction)>);
#[derive(Instruction)]
#[instruction(
    FlipBinaryInstruction->{(i1:LuaValue,i2:LuaValue)->(i2:LuaValue){
        Init:{
            entry:{
                %i1_tag=lua_value::GetTag(%i1);
                %i2_tag=lua_value::GetTag(%i2);
                if UsizeLt(UsizeOr(%i1_tag,%i2_tag),b::IntTruncate<12,7>(4)) %double_number %not_double_number; },
            double_number:{ if UsizeLt(UsizeOr(%i1_tag,%i2_tag),b::IntTruncate<12,7>(2)) %double_integer %not_double_integer; },
            double_integer:{
                %i1_integer_value=GetIntegerValue(%i1);
                %i2_integer_value=GetIntegerValue(%i2);
                SetState<%DoubleInteger>();
                %i2=IntegerInstruction(%i1_integer_value,%i2_integer_value); },
            not_double_integer:{
                %i1_float_value=ToFloat(%i1);
                %i2_float_value=ToFloat(%i2);
                SetState<%DoubleFloat>();
                %i2=FloatInstruction(%i1_float_value,%i2_float_value); },
            not_double_number:{
                %i1_meta_function = GetMetaFunction(%i1);
                if lua_value::IsNil(%i1_meta_function) %i1_has_no_meta_function %i1_has_meta_function; },
            i1_has_meta_function:{
                SetState<%UseMetaMethodOfI1>();
                %i2=CallFunction2Ret1(%i1_meta_function,%i2,%i1); },
            i1_has_no_meta_function:{
                %i2_meta_function = GetMetaFunction(%i2);
                if lua_value::IsNil(%i2_meta_function) %i2_has_no_meta_function %i2_has_meta_function; },
            i2_has_meta_function:{
                SetState<%UseMetaMethodOfI2>();
                %i2=CallFunction2Ret1(%i2_meta_function,%i2,%i1); },
            i2_has_no_meta_function:{  %i2=ConstNil();ThrowError(); },
        },
        DoubleInteger:{
            entry:{
                %i1_tag=lua_value::GetTag(%i1);
                %i2_tag=lua_value::GetTag(%i2);
                if UsizeLt(UsizeOr(%i1_tag,%i2_tag),b::IntTruncate<12,7>(4)) %double_integer %other; },
            double_integer:{
                %i1_integer_value=GetIntegerValue(%i1);
                %i2_integer_value=GetIntegerValue(%i2);
                %i2=IntegerInstruction(%i1_integer_value,%i2_integer_value); },
            other:{  %i2=CallState<%Init>(%i1,%i2); },
        },
        DoubleFloat:{
            entry:{
                %i1_tag=lua_value::GetTag(%i1);
                %i2_tag=lua_value::GetTag(%i2);
                if UsizeLt(UsizeSub(UsizeOr(%i1_tag,%i2_tag),b::IntTruncate<12,7>(2)),b::IntTruncate<12,7>(2)) %double_float %other; },
            double_float:{
                %i1_float_value=GetFloatValue(%i1);
                %i2_float_value=GetFloatValue(%i2);
                %i2=FloatInstruction(%i1_float_value,%i2_float_value); },
            other:{  %i2=CallState<%Init>(%i1,%i2); },
        },
        UseMetaMethodOfI1:{
            entry:{
                %i1_meta_function = GetMetaFunction(%i1);
                if lua_value::IsNil(%i1_meta_function) %other %i1_has_meta_function; },
            i1_has_meta_function:{
                SetState<%UseMetaMethodOfI1>();
                %i2=CallFunction2Ret1(%i1_meta_function,%i2,%i1); },
            other:{ SetState<%Init>(); },
        },
        UseMetaMethodOfI2:{
            entry:{
                %i1_meta_function = GetMetaFunction(%i1);
                if lua_value::IsNil(%i1_meta_function) %other %i1_has_meta_function; },
            i1_has_meta_function:{
                SetState<%UseMetaMethodOfI1>();
                %i2=CallFunction2Ret1(%i1_meta_function,%i2,%i1); },
            i1_has_no_meta_function:{
                %i2_meta_function = GetMetaFunction(%i2);
                if lua_value::IsNil(%i2_meta_function) %other %i2_has_meta_function; },
            i2_has_meta_function:{
                SetState<%UseMetaMethodOfI2>();
                %i2=CallFunction2Ret1(%i2_meta_function,%i2,%i1); },
            other:{ SetState<%Init>(); },
        },
    }}
    )]
pub struct FlipBinaryInstruction<
    IntegerInstruction: Instruction,
    FloatInstruction: Instruction,
    GetMetaFunction: Instruction,
>(PhantomData<(IntegerInstruction, FloatInstruction, GetMetaFunction)>);
#[derive(Instruction)]
#[instruction(
    NegationBinaryInstruction->{(i1:LuaValue,i2:LuaValue)->(i2:LuaValue){
        Init:{
            entry:{
                %i1_tag=lua_value::GetTag(%i1);
                %i2_tag=lua_value::GetTag(%i2);
                if IsizeLt(b::IntTruncate<11,12>(UsizeOr(%i1_tag,%i2_tag)),b::IntTruncate<11,7>(4)) %double_number %not_double_number; },
            double_number:{ if UsizeLt(UsizeOr(%i1_tag,%i2_tag),b::IntTruncate<12,7>(2)) %double_integer %not_double_integer; },
            double_integer:{
                %i1_integer_value=GetIntegerValue(%i1);
                %i2_integer_value=GetIntegerValue(%i2);
                SetState<%DoubleInteger>();
                %i2=IntegerInstruction(%i1_integer_value,%i2_integer_value); },
            not_double_integer:{
                %i1_float_value=ToFloat(%i1);
                %i2_float_value=ToFloat(%i2);
                SetState<%DoubleFloat>();
                %i2=FloatInstruction(%i1_float_value,%i2_float_value); },
            not_double_number:{
                %i1_meta_function = GetMetaFunction(%i1);
                if lua_value::IsNil(%i1_meta_function) %i1_has_no_meta_function %i1_has_meta_function; },
            i1_has_meta_function:{
                SetState<%UseMetaMethodOfI1>();
                %i2=LogicalNot(CallFunction2Ret1(%i1_meta_function,%i1,%i2)); },
            i1_has_no_meta_function:{
                %i2_meta_function = GetMetaFunction(%i2);
                if lua_value::IsNil(%i2_meta_function) %i2_has_no_meta_function %i2_has_meta_function; },
            i2_has_meta_function:{
                SetState<%UseMetaMethodOfI2>();
                %i2=LogicalNot(CallFunction2Ret1(%i2_meta_function,%i1,%i2)); },
            i2_has_no_meta_function:{  %i2=ConstNil();ThrowError(); },
        },
        DoubleInteger:{
            entry:{
                %i1_tag=lua_value::GetTag(%i1);
                %i2_tag=lua_value::GetTag(%i2);
                if UsizeLt(UsizeOr(%i1_tag,%i2_tag),b::IntTruncate<12,7>(4)) %double_integer %other; },
            double_integer:{
                %i1_integer_value=GetIntegerValue(%i1);
                %i2_integer_value=GetIntegerValue(%i2);
                %i2=IntegerInstruction(%i1_integer_value,%i2_integer_value); },
            other:{  %i2=CallState<%Init>(%i1,%i2); },
        },
        DoubleFloat:{
            entry:{
                %i1_tag=lua_value::GetTag(%i1);
                %i2_tag=lua_value::GetTag(%i2);
                if UsizeLt(UsizeSub(UsizeOr(%i1_tag,%i2_tag),b::IntTruncate<12,7>(2)),b::IntTruncate<12,7>(2)) %double_float %other; },
            double_float:{
                %i1_float_value=GetFloatValue(%i1);
                %i2_float_value=GetFloatValue(%i2);
                %i2=FloatInstruction(%i1_float_value,%i2_float_value); },
            other:{  %i2=CallState<%Init>(%i1,%i2); },
        },
        UseMetaMethodOfI1:{
            entry:{
                %i1_meta_function = GetMetaFunction(%i1);
                if lua_value::IsNil(%i1_meta_function) %other %i1_has_meta_function; },
            i1_has_meta_function:{
                SetState<%UseMetaMethodOfI1>();
                %i2=LogicalNot(CallFunction2Ret1(%i1_meta_function,%i1,%i2)); },
            other:{ SetState<%Init>(); },
        },
        UseMetaMethodOfI2:{
            entry:{
                %i1_meta_function = GetMetaFunction(%i1);
                if lua_value::IsNil(%i1_meta_function) %other %i1_has_meta_function; },
            i1_has_meta_function:{
                SetState<%UseMetaMethodOfI1>();
                %i2=LogicalNot(CallFunction2Ret1(%i1_meta_function,%i1,%i2)); },
            i1_has_no_meta_function:{
                %i2_meta_function = GetMetaFunction(%i2);
                if lua_value::IsNil(%i2_meta_function) %other %i2_has_meta_function; },
            i2_has_meta_function:{
                SetState<%UseMetaMethodOfI2>();
                %i2=LogicalNot(CallFunction2Ret1(%i2_meta_function,%i1,%i2)); },
            other:{ SetState<%Init>(); },
        },
    }}
    )]
pub struct NegationBinaryInstruction<
    IntegerInstruction: Instruction,
    FloatInstruction: Instruction,
    GetMetaFunction: Instruction,
>(PhantomData<(IntegerInstruction, FloatInstruction, GetMetaFunction)>);
#[derive(Instruction)]
#[instruction(
    BinaryInstruction->{(i1:LuaValue,i2:LuaValue)->(i2:LuaValue){
        Init:{
            entry:{
                %i1_tag=lua_value::GetTag(%i1);
                %i2_tag=lua_value::GetTag(%i2);
                if UsizeLt(UsizeOr(%i1_tag,%i2_tag),b::IntTruncate<12,7>(4)) %double_number %not_double_number; },
            double_number:{ if UsizeLt(UsizeOr(%i1_tag,%i2_tag),b::IntTruncate<12,7>(2)) %double_integer %not_double_integer; },
            double_integer:{ if UsizeLt(UsizeOr(%i1_tag,%i2_tag),b::IntTruncate<12,7>(1)) %double_small_integer %not_double_small_integer; },
            double_small_integer:{
                %i1_integer_value=I64Shr(lua_value::DecodeIntegerUnchecked(%i1),4);
                %i2_integer_value=I64Shr(lua_value::DecodeIntegerUnchecked(%i2),4);
                SetState<%DoubleSmallInteger>();
                %i2=IntegerInstruction(%i1_integer_value,%i2_integer_value); },
            not_double_small_integer:{
                %i1_integer_value=GetIntegerValue(%i1);
                %i2_integer_value=GetIntegerValue(%i2);
                SetState<%DoubleInteger>();
                %i2=IntegerInstruction(%i1_integer_value,%i2_integer_value); },
            not_double_integer:{
                %i1_float_value=ToFloat(%i1);
                %i2_float_value=ToFloat(%i2);
                SetState<%DoubleFloat>();
                %i2=FloatInstruction(%i1_float_value,%i2_float_value); },
            not_double_number:{
                %i1_meta_function = GetMetaFunction(%i1);
                if lua_value::IsNil(%i1_meta_function) %i1_has_no_meta_function %i1_has_meta_function; },
            i1_has_meta_function:{
                SetState<%UseMetaMethodOfI1>();
                %i2=CallFunction2Ret1(%i1_meta_function,%i1,%i2); },
            i1_has_no_meta_function:{
                %i2_meta_function = GetMetaFunction(%i2);
                if lua_value::IsNil(%i2_meta_function) %i2_has_no_meta_function %i2_has_meta_function; },
            i2_has_meta_function:{
                SetState<%UseMetaMethodOfI2>();
                %i2=CallFunction2Ret1(%i2_meta_function,%i1,%i2); },
            i2_has_no_meta_function:{ %i2=ConstNil();  ThrowError(); },
        },
        DoubleSmallInteger:{
            entry:{
                %i1c=MoveValue(%i1);
                %i2c=MoveValue(%i2);
                %i1_tag=lua_value::GetTag(%i1c);
                %i2_tag=lua_value::GetTag(%i2c);
                %i1_integer_value=I64Shr(lua_value::DecodeIntegerUnchecked(%i1c),4);
                %i2_integer_value=I64Shr(lua_value::DecodeIntegerUnchecked(%i2c),4);
                if UsizeLt(UsizeOr(%i1_tag,%i2_tag),b::IntTruncate<12,7>(1)) %double_integer %other; },
            double_integer:{
                %i2=IntegerInstruction(%i1_integer_value,%i2_integer_value); },
            other:{  %i2=CallState<%Init>(%i1c,%i2c); },
        },
        DoubleInteger:{
            entry:{
                %i1_tag=lua_value::GetTag(%i1);
                %i2_tag=lua_value::GetTag(%i2);
                if UsizeLt(UsizeOr(%i1_tag,%i2_tag),b::IntTruncate<12,7>(2)) %double_integer %other; },
            double_integer:{
                %i1_integer_value=GetIntegerValue(%i1);
                %i2_integer_value=GetIntegerValue(%i2);
                %i2=IntegerInstruction(%i1_integer_value,%i2_integer_value); },
            other:{  %i2=CallState<%Init>(%i1,%i2); },
        },
        DoubleFloat:{
            entry:{
                %i1_tag=lua_value::GetTag(%i1);
                %i2_tag=lua_value::GetTag(%i2);
                if UsizeLt(UsizeSub(UsizeOr(%i1_tag,%i2_tag),b::IntTruncate<12,7>(2)),b::IntTruncate<12,7>(2)) %double_float %other; },
            double_float:{
                %i1_float_value=GetFloatValue(%i1);
                %i2_float_value=GetFloatValue(%i2);
                %i2=FloatInstruction(%i1_float_value,%i2_float_value); },
            other:{  %i2=CallState<%Init>(%i1,%i2); },
        },
        UseMetaMethodOfI1:{
            entry:{
                %i1_meta_function = GetMetaFunction(%i1);
                if lua_value::IsNil(%i1_meta_function) %other %i1_has_meta_function; },
            i1_has_meta_function:{
                SetState<%UseMetaMethodOfI1>();
                %i2=CallFunction2Ret1(%i1_meta_function,%i1,%i2); },
            other:{ SetState<%Init>(); },
        },
        UseMetaMethodOfI2:{
            entry:{
                %i1_meta_function = GetMetaFunction(%i1);
                if lua_value::IsNil(%i1_meta_function) %other %i1_has_meta_function; },
            i1_has_meta_function:{
                SetState<%UseMetaMethodOfI1>();
                %i2=CallFunction2Ret1(%i1_meta_function,%i1,%i2); },
            i1_has_no_meta_function:{
                %i2_meta_function = GetMetaFunction(%i2);
                if lua_value::IsNil(%i2_meta_function) %other %i2_has_meta_function; },
            i2_has_meta_function:{
                SetState<%UseMetaMethodOfI2>();
                %i2=CallFunction2Ret1(%i2_meta_function,%i1,%i2); },
            other:{ SetState<%Init>(); },
        },
    }}
    )]
pub struct BinaryInstruction<
    IntegerInstruction: Instruction,
    FloatInstruction: Instruction,
    GetMetaFunction: Instruction,
>(PhantomData<(IntegerInstruction, FloatInstruction, GetMetaFunction)>);
#[derive(Instruction)]
#[instruction(
    UniqueIntegerInstruction->{(i1:LuaValue,)->(i1:LuaValue){
        Init:{
            entry:{
                %i1_tag=lua_value::GetTag(%i1);
                if UsizeLt(%i1_tag,b::IntTruncate<12,7>(4)) %number %not_number; },
            number:{
                %i1_integer_value=GetIntegerValue(%i1);
                SetState<%Integer>();
                %i1=IntegerInstruction(%i1_integer_value); },
            not_number:{
                %i1_meta_function = GetMetaFunction(%i1);
                if lua_value::IsNil(%i1_meta_function) %i1_has_no_meta_function %i1_has_meta_function; },
            i1_has_meta_function:{
                SetState<%UseMetaMethodOfI1>();
                %i1=CallFunction1Ret1(%i1_meta_function,%i1);
            },
            i1_has_no_meta_function:{ %i1=ConstNil();ThrowError();  },
        },
        Integer:{
            entry:{
                %i1_tag=lua_value::GetTag(%i1);
                if UsizeLt(%i1_tag,b::IntTruncate<12,7>(2)) %number %other; },
            number:{
                %i1_integer_value=GetIntegerValue(%i1);
                %i1=IntegerInstruction(%i1_integer_value); },
            other:{ %i1=CallState<%Init>(%i1); },
        },
        UseMetaMethodOfI1:{
            entry:{
                %i1_meta_function = GetMetaFunction(%i1);
                if lua_value::IsNil(%i1_meta_function) %other %i1_has_meta_function; },
            i1_has_meta_function:{
                SetState<%UseMetaMethodOfI1>();
                %i1=CallFunction1Ret1(%i1_meta_function,%i1); },
            other:{ SetState<%Init>(); },
        },
    }}
    )]
pub struct UniqueIntegerInstruction<IntegerInstruction: Instruction, GetMetaFunction: Instruction>(
    PhantomData<(IntegerInstruction, GetMetaFunction)>,
);
pub type OptionLuaMetaFunctionsRefIsSome = nullable_option::IsSome<LuaMetaFunctions>;
#[derive(Instruction)]
#[instruction(
    BinaryIntegerInstruction->{(i1:LuaValue,i2:LuaValue)->(i2:LuaValue){
        Init:{
            entry:{
                ThrowError();
                %i1_tag=lua_value::GetTag(%i1);
                %i2_tag=lua_value::GetTag(%i2);
                if IsizeLt(b::IntTruncate<11,12>(UsizeOr(%i1_tag,%i2_tag)),b::IntTruncate<11,7>(4)) %double_number %not_double_number; },
            double_number:{
                %i1_integer_value=GetIntegerValue(%i1);
                %i2_integer_value=GetIntegerValue(%i2);
                SetState<%DoubleInteger>();
                %i2=IntegerInstruction(%i1_integer_value,%i2_integer_value); },
            not_double_number:{
                %i1_meta_function = GetMetaFunction(%i1);
                if lua_value::IsNil(%i1_meta_function) %i1_has_no_meta_function %i1_has_meta_function; },
            i1_has_meta_function:{
                SetState<%UseMetaMethodOfI1>();
                %i2=CallFunction2Ret1(%i1_meta_function,%i1,%i2);
            },
            i1_has_no_meta_function:{
                %i2_meta_function = GetMetaFunction(%i2);
                if lua_value::IsNil(%i2_meta_function) %i2_has_no_meta_function %i2_has_meta_function; },
            i2_has_meta_function:{
                SetState<%UseMetaMethodOfI2>();
                %i2=CallFunction2Ret1(%i2_meta_function,%i1,%i2); },
            i2_has_no_meta_function:{  %i2=ConstNil();ThrowError(); },
        },
        DoubleInteger:{
            entry:{
                %i1_tag=lua_value::GetTag(%i1);
                %i2_tag=lua_value::GetTag(%i2);
                if UsizeLt(UsizeOr(%i1_tag,%i2_tag),b::IntTruncate<12,7>(4)) %double_number %other; },
            double_number:{
                %i1_integer_value=GetIntegerValue(%i1);
                %i2_integer_value=GetIntegerValue(%i2);
                %i2=IntegerInstruction(%i1_integer_value,%i2_integer_value); },
            other:{  %i2=CallState<%Init>(%i1,%i2); },
        },
        UseMetaMethodOfI1:{
            entry:{
                %i1_meta_function = GetMetaFunction(%i1);
                if lua_value::IsNil(%i1_meta_function) %other %i1_has_meta_function; },
            i1_has_meta_function:{
                SetState<%UseMetaMethodOfI1>();
                %i2=CallFunction2Ret1(%i1_meta_function,%i1,%i2); },
            other:{ SetState<%Init>(); },
        },
        UseMetaMethodOfI2:{
            entry:{
                %i1_meta_function = GetMetaFunction(%i1);
                if lua_value::IsNil(%i1_meta_function) %i1_has_meta_function %other; },
            i1_has_meta_function:{
                SetState<%UseMetaMethodOfI1>();
                %i2=CallFunction2Ret1(%i1_meta_function,%i1,%i2);
            },
            i1_has_no_meta_function:{
                %i2_meta_function = GetMetaFunction(%i2);
                if lua_value::IsNil(%i2_meta_function) %other %i2_has_meta_function; },
            i2_has_meta_function:{
                SetState<%UseMetaMethodOfI2>();
                %i2=CallFunction2Ret1(%i2_meta_function,%i1,%i2); },
            other:{ SetState<%Init>(); },
        },
    }}
    )]
pub struct BinaryIntegerInstruction<IntegerInstruction: Instruction, GetMetaFunction: Instruction>(
    PhantomData<(IntegerInstruction, GetMetaFunction)>,
);
#[derive(Instruction)]
#[instruction( WrapToInteger->fn(i1:I64,i2:I64)->(o:LuaValue){ entry:{
    %o = I64ToValue(I(%i1,%i2));
} })]
pub struct WrapBinaryInteger<I: Instruction>(PhantomData<I>);
#[derive(Instruction)]
#[instruction( WrapToFloat->fn(i1:F64,i2:F64)->(o:LuaValue){ entry:{
    %o = F64ToValue(I(%i1,%i2));
} })]
pub struct WrapBinaryFloat<I: Instruction>(PhantomData<I>);
#[derive(Instruction)]
#[instruction( WrapToInteger->fn(i1:I64)->(o:LuaValue){ entry:{
    %o = I64ToValue(I(%i1));
} })]
pub struct WrapInteger<I: Instruction>(PhantomData<I>);
#[derive(Instruction)]
#[instruction( WrapToFloat->fn(i1:F64)->(o:LuaValue){ entry:{
    %o = F64ToValue(I(%i1));
} })]
pub struct WrapFloat<I: Instruction>(PhantomData<I>);
#[derive(Instruction)]
#[instruction( WrapBinaryIntToBool->fn(i1:I64,i2:I64)->(o:LuaValue){ entry:{
    %o = EncodeBoolean(I(%i1,%i2));
} })]
pub struct WrapBinaryIntToBool<I: Instruction>(PhantomData<I>);
#[derive(Instruction)]
#[instruction( FlipBinaryIntegerToBool->fn(v1:I64,v2:I64)->(o:LuaValue){ entry:{
    %o = EncodeBoolean(I(%v2,%v1));
} })]
pub struct FlipBinaryIntegerToBool<I: Instruction>(PhantomData<I>);
#[derive(Instruction)]
#[instruction( WrapBinaryFloatToBool->fn(i1:F64,i2:F64)->(o:LuaValue){ entry:{
    %o = EncodeBoolean(I(%i1,%i2));
} })]
pub struct WrapBinaryFloatToBool<I: Instruction>(PhantomData<I>);
#[derive(Instruction)]
#[instruction( WrapBinaryIntToFloat->fn(i1:I64,i2:I64)->(o:LuaValue){ entry:{
    %o = F64ToValue(I(I64ToF64(%i1),I64ToF64(%i2)));
} })]
pub struct WrapBinaryIntToFloat<I: Instruction>(PhantomData<I>);
pub type Add =
    BinaryInstruction<WrapBinaryInteger<I64Add>, WrapBinaryFloat<F64Add>, GetMetaValue<lua_meta_functions::ReadAdd>>;
pub type Sub =
    BinaryInstruction<WrapBinaryInteger<I64Sub>, WrapBinaryFloat<F64Sub>, GetMetaValue<lua_meta_functions::ReadSub>>;
pub type Mul =
    BinaryInstruction<WrapBinaryInteger<I64Mul>, WrapBinaryFloat<F64Mul>, GetMetaValue<lua_meta_functions::ReadMul>>;
pub type Pow =
    BinaryInstruction<WrapBinaryIntToFloat<FPow>, WrapBinaryFloat<FPow>, GetMetaValue<lua_meta_functions::ReadPow>>;
pub type IDiv = WrapBinaryIntToFloat<F64Div>;
pub type Div =
    BinaryInstruction<WrapBinaryIntToFloat<F64Div>, WrapBinaryFloat<F64Div>, GetMetaValue<lua_meta_functions::ReadDiv>>;
pub type IEqual = WrapBinaryIntToBool<I64Eq>;
pub type FEqual = WrapBinaryFloatToBool<F64Eq>;
pub type Equal = BinaryInstruction<IEqual, FEqual, GetMetaValue<lua_meta_functions::ReadEq>>;
pub type ILessOrEqual = WrapBinaryIntToBool<I64Le>;
pub type FLessOrEqual = WrapBinaryFloatToBool<F64Le>;
pub type LessOrEqual = BinaryInstruction<ILessOrEqual, FLessOrEqual, GetMetaValue<lua_meta_functions::ReadLe>>;
pub type ILess = WrapBinaryIntToBool<I64Lt>;
pub type FLess = WrapBinaryFloatToBool<F64Lt>;
pub type Less = BinaryInstruction<ILess, FLess, GetMetaValue<lua_meta_functions::ReadLt>>;
pub type INotEqual = WrapBinaryIntToBool<I64Ne>;
pub type FNotEqual = WrapBinaryFloatToBool<F64Ne>;
pub type NotEqual = NegationBinaryInstruction<INotEqual, FNotEqual, GetMetaValue<lua_meta_functions::ReadEq>>;
pub type ILarge = WrapBinaryIntToBool<I64Gt>;
pub type FLarge = WrapBinaryFloatToBool<F64Gt>;
pub type Large = FlipBinaryInstruction<ILarge, FLarge, GetMetaValue<lua_meta_functions::ReadLt>>;
pub type ILargeOrEqual = WrapBinaryIntToBool<I64Ge>;
pub type FLargeOrEqual = WrapBinaryFloatToBool<F64Ge>;
pub type LargeOrEqual = FlipBinaryInstruction<ILargeOrEqual, FLargeOrEqual, GetMetaValue<lua_meta_functions::ReadLe>>;
pub type Neg = UniqueInstruction<WrapInteger<I64Neg>, WrapFloat<F64Neg>, GetMetaValue<lua_meta_functions::ReadUnm>>;
pub type DivFloor = BinaryIntegerInstruction<WrapBinaryInteger<I64Div>, GetMetaValue<lua_meta_functions::ReadIdiv>>;
pub type Rem =
    BinaryInstruction<WrapBinaryInteger<I64Rem>, WrapBinaryFloat<F64Rem>, GetMetaValue<lua_meta_functions::ReadMod>>;
pub type BitAnd = BinaryIntegerInstruction<WrapBinaryInteger<I64And>, GetMetaValue<lua_meta_functions::ReadBand>>;
pub type BitOr = BinaryIntegerInstruction<WrapBinaryInteger<I64Or>, GetMetaValue<lua_meta_functions::ReadBor>>;
pub type BitXor = BinaryIntegerInstruction<WrapBinaryInteger<I64Xor>, GetMetaValue<lua_meta_functions::ReadBxor>>;
pub type BitNot = UniqueIntegerInstruction<WrapInteger<I64Not>, GetMetaValue<lua_meta_functions::ReadBnot>>;
pub type LeftShift = BinaryIntegerInstruction<WrapBinaryInteger<I64Shl>, GetMetaValue<lua_meta_functions::ReadShl>>;
pub type RightShift = BinaryIntegerInstruction<WrapBinaryInteger<I64Shr>, GetMetaValue<lua_meta_functions::ReadShr>>;
pub type MoveI64 = Move<I64>;
pub type MoveF64 = Move<F64>;
pub type MoveValue = Move<LuaValue>;
#[derive(TypeDeclaration)]
#[make_type(make_instruction)]
pub struct InlineCacheLine {
    pub shape: NullableOption<LuaShapeReference>,
    pub key: LuaValue,
    pub table: NullableOption<LuaTableReference>,
    pub invalid: NullableOption<BoolReference>,
    pub slot: U32,
}
impl<'l> MoveIntoObject<'l> for InlineCacheLineImpl {
    type Carrier = Self;

    fn set(this: Self, offset: usize, object_builder: &ObjectBuilder<'l>, token: &mut ghost_cell::GhostToken<'l>) {
        object_builder.borrow_mut(token).receive_at(offset).write(this.0);
    }
}
type NullableBoolReferenceDecodeSomeUnchecked = nullable_option::DecodeSomeUnchecked<BoolReference>;
type NullableTableReferenceIsSome = nullable_option::IsSome<LuaTableReference>;
type NullableTableReferenceDecodeSome = nullable_option::DecodeSomeUnchecked<LuaTableReference>;
type NullableShapeReferenceEncodeSome = nullable_option::EncodeSome<LuaShapeReference>;
make_instruction! {GetByCache->fn<mut cache:InlineCacheLine>(table:Pointer<LuaTable>)->(o:LuaValue){
    entry:{
        %shape=lua_table::ReadShape(%table);
        if UsizeEq(b::CastUnchecked<Usize::TYPE,NullableOption::<LuaShapeReference>::TYPE>(inline_cache_line::ReadShape(%cache)),b::CastUnchecked<Usize::TYPE,NullableOption::<LuaShapeReference>::TYPE>(NullableShapeReferenceEncodeSome(%shape))) %correct_shape %none;
    },
    correct_shape:{ if b::Read<Bool::TYPE>(b::Deref<BoolReference::TYPE>(NullableBoolReferenceDecodeSomeUnchecked(inline_cache_line::ReadInvalid(%cache)))) %none %valid; },
    valid:{if NullableTableReferenceIsSome(inline_cache_line::ReadTable(%cache)) %use_metatable %use_raw;},
    use_raw:{%o=Read<LuaValue::TYPE>(LocateSlot(%table,b::UIntExtend<12,6>(inline_cache_line::ReadSlot(%cache))));},
    use_metatable:{%o=Read<LuaValue::TYPE>(LocateSlot(b::Deref<LuaTableReference::TYPE>(NullableTableReferenceDecodeSome(inline_cache_line::ReadTable(%cache))),b::UIntExtend<12,6>(inline_cache_line::ReadSlot(%cache))));},
    none:{%o=ConstNil();},
}}
make_instruction! {SetByCache->fn<mut cache:InlineCacheLine>(table:Pointer<LuaTable>,value:LuaValue)->(o:Bool){
    entry:{
        %shape=lua_table::ReadShape(%table);
        if UsizeEq(b::CastUnchecked<Usize::TYPE,NullableOption::<LuaShapeReference>::TYPE>(inline_cache_line::ReadShape(%cache)),b::CastUnchecked<Usize::TYPE,NullableOption::<LuaShapeReference>::TYPE>(NullableShapeReferenceEncodeSome(%shape))) %correct_shape %none;
    },
    correct_shape:{ if b::Read<Bool::TYPE>(b::Deref<BoolReference::TYPE>(NullableBoolReferenceDecodeSomeUnchecked(inline_cache_line::ReadInvalid(%cache)))) %none %valid; },
    valid:{if NullableTableReferenceIsSome(inline_cache_line::ReadTable(%cache)) %use_metatable %use_raw;},
    use_raw:{
        Write<LuaValue::TYPE>(LocateSlot(%table,b::UIntExtend<12,6>(inline_cache_line::ReadSlot(%cache))),%value);
        %o=true; },
    use_metatable:{
        Write<LuaValue::TYPE>(LocateSlot(b::Deref<LuaTableReference::TYPE>(NullableTableReferenceDecodeSome(inline_cache_line::ReadTable(%cache))),b::UIntExtend<12,6>(inline_cache_line::ReadSlot(%cache))),%value);
        %o=true; },
    none:{%o=false;},
}}
#[make_native_function(GetSlot)]
pub unsafe extern "C" fn __vm_lua_lib_get_slot(shape: Pointer<LuaShape>, key: Direct<LuaValue>) -> I64 {
    let hash_map = shape.as_ref().ref_fields().get().as_ref().unwrap();
    hash_map
        .get(&key.0)
        .map(|slot_metadata| I64(slot_metadata.get_slot().0 as i64))
        .unwrap_or(I64(-1))
}
type EncodeSomeShapeReference = nullable_option::EncodeSome<LuaShapeReference>;
type EncodeSomeBoolReference = nullable_option::EncodeSome<BoolReference>;
type EncodeSomeTableReference = nullable_option::EncodeSome<LuaTableReference>;
make_instruction! {
    GetElement->fn<mut cache:InlineCacheLine>(obj:LuaValue,key:LuaValue)->(value:LuaValue){
      entry:{
          branch %loop;},
      loop:{
          phi %obj:LuaValue={%entry=>%obj,%use_index_table=>%index};
          if lua_value::IsTable(%obj) %is_table %not_found; },
        is_table:{
          %table=b::Deref<LuaTableReference::TYPE>(lua_value::DecodeTableUnchecked(%obj));
          %shape=b::Deref<LuaShapeReference::TYPE>(lua_table::ReadShape(%table));
          %slot=GetSlot(%shape,%key);
          if I64Eq(%slot,-1) %not_found %found;},
        found:{
            %v=Read<LuaValue::TYPE>(LocateSlot(%table,b::IntTruncate<12,7>(%slot)));
            if lua_value::IsNil(%v) %not_found %finish; },
        finish:{
            inline_cache_line::WriteKey(%cache,%key);
            inline_cache_line::WriteShape(%cache,EncodeSomeShapeReference(b::Clone<LuaShapeReference::TYPE>(lua_table::ReadShape(%table))));
            inline_cache_line::WriteSlot(%cache,b::IntTruncate<6,7>(%slot));
            inline_cache_line::WriteInvalid(%cache,EncodeSomeBoolReference(b::Clone<BoolReference::TYPE>(lua_shape::ReadInvalid(%shape))));
            inline_cache_line::WriteTable(%cache,EncodeSomeTableReference(lua_value::DecodeTableUnchecked(%obj)));
            %value=b::Move<LuaValue::TYPE>(%v); },
        not_found:{
            %index=GetMetaValueIndex(%obj);
            if lua_value::IsTable(%index) %use_index_table %not_index_table; },
          not_index_table:{if lua_value::IsNil(%index) %use_nil %index_callable;},
          index_callable:{ %value=CallFunction2Ret1(%index,%obj,%key); },
        use_nil:{%value=ConstNil();},
      use_index_table:{
          %index_table=lua_value::DecodeTableUnchecked(%index);
          branch %loop;
      },
    }
}
#[make_native_function(ShapeAction)]
pub unsafe extern "C" fn __vm_lua_lib_shape_action(
    shape: Pointer<LuaShape>,
    key: Direct<LuaValue>,
) -> Direct<NullableOption<LuaShapeReference>> {
    Direct(
        shape
            .as_ref()
            .ref_action_of_field()
            .get()
            .as_mut()
            .unwrap()
            .get(&key.0)
            .map(|(action, _slot)| NullableOptionImpl::encode_some(Pointer::new(action.0.cast())))
            .unwrap_or(NullableOptionImpl::encode_none(())),
    )
}
#[make_native_function(InsertField)]
pub unsafe extern "C" fn __vm_lua_lib_insert_field(shape: Pointer<LuaShape>, key: Direct<LuaValue>) -> Usize {
    let hash_map = shape.as_ref().ref_fields().get().as_mut().unwrap();
    let len = hash_map.len();
    unsafe {
        let mut slot_metadata = MaybeUninit::<LuaSlotMetadataImpl>::zeroed();
        let slot_metadata_ref = slot_metadata.assume_init_mut();
        slot_metadata_ref.set_slot(Usize(len));
        hash_map.insert(key.0, slot_metadata.assume_init_read());
    }
    Usize(len)
}
#[make_native_function(InsertAction)]
pub unsafe extern "C" fn __vm_lua_lib_insert_action(
    shape: Pointer<LuaShape>,
    key: Direct<LuaValue>,
    new_shape: LuaShapeReference,
    slot: Usize,
) {
    shape
        .as_ref()
        .ref_action_of_field()
        .get()
        .as_mut()
        .unwrap()
        .insert(key.0, (new_shape, slot.0));
}
#[make_native_function(CloneLuaShape)]
pub unsafe extern "C" fn __vm_lua_lib_clone_shape(dest: Pointer<LuaShape>, src: Pointer<LuaShape>) {
    let mut dest = dest;
    dest.as_ref_mut().set_fields(UnsafeCell::new(
        src.as_ref().ref_fields().get().as_ref().unwrap().clone(),
    ));
    dest.as_ref_mut().set_meta_functions(src.as_ref().get_meta_functions());
    dest.as_ref_mut().set_as_meta_table(src.as_ref().get_as_meta_table());
    dest.as_ref_mut().set_max_int_index(src.as_ref().get_max_int_index());
    dest.as_ref_mut().set_is_owned(src.as_ref().get_is_owned());
    dest.as_ref_mut().set_invalid(src.as_ref().get_invalid());
    dest.as_ref_mut().set_action_of_field(Default::default());
    dest.as_ref_mut().set_action_of_metatable(Default::default());
}
type GetMetaValueIndex = GetMetaValue<lua_meta_functions::ReadIndex>;
type GetMetaValueNewIndex = GetMetaValue<lua_meta_functions::ReadNewindex>;
type NullableShapeIsSome = e::nullable_option::IsSome<LuaShapeReference>;
type NullableShapeDecodeSome = e::nullable_option::DecodeSomeUnchecked<LuaShapeReference>;
make_instruction! {
    SetElement->fn<mut cache:InlineCacheLine>(value:LuaValue,key:LuaValue,elem:LuaValue){
      entry:{branch %loop;},
      loop:{
          phi %value:LuaValue={%entry=>%value,%use_new_index_table=>%new_index};
          if lua_value::IsTable(%value) %is_table %not_table; },
        is_table:{
          %table=b::Deref<LuaTableReference::TYPE>(lua_value::DecodeTableUnchecked(%value));
          %shape=b::Deref<LuaShapeReference::TYPE>(lua_table::ReadShape(%table));
          %slot=GetSlot(%shape,%key);
          if I64Less(%slot,0) %not_found %found;},
        not_table:{
            %new_index=GetMetaValueNewIndex(%value);
            if lua_value::IsNil(%new_index) %other %use_new_index; },
        other:{ThrowError();},
        found:{
            inline_cache_line::WriteKey(%cache,%key);
            inline_cache_line::WriteShape(%cache,EncodeSomeShapeReference(b::Clone<LuaShapeReference::TYPE>(lua_table::ReadShape(%table))));
            inline_cache_line::WriteSlot(%cache,b::IntTruncate<6,7>(%slot));
            inline_cache_line::WriteInvalid(%cache,EncodeSomeBoolReference(b::Clone<BoolReference::TYPE>(lua_shape::ReadInvalid(%shape))));
            inline_cache_line::WriteTable(%cache,EncodeSomeTableReference(lua_value::DecodeTableUnchecked(%value)));
            Write<LuaValue::TYPE>(LocateSlot(%table,b::IntTruncate<12,7>(%slot)),%elem); },
        not_found:{
            %new_index=lua_meta_functions::ReadNewindex(b::Deref<LuaMetaFunctionsReference::TYPE>(lua_shape::ReadMetaFunctions(%shape)));
            if lua_value::IsNil(%new_index) %add_slot %use_new_index;
        },
          use_new_index:{ if lua_value::IsTable(%new_index) %use_new_index_table %use_new_index_function; },
          use_new_index_function:{ %r=CallFunction3Ret1(%new_index,%value,%key,%elem); },
          use_new_index_table:{
              %new_index_table=lua_value::DecodeTableUnchecked(%new_index);
              branch %loop; },
          add_slot:{ if lua_shape::ReadIsOwned(%shape) %extend %other_slot; },
            extend:{
              %slot = InsertField(%shape,%key);
              Write<LuaValue::TYPE>(LocateNewSlot(%table,%slot),%elem); },
            other_slot:{
                %goto_action=ShapeAction(%shape,%key);
                if NullableShapeIsSome(%goto_action) %goto_shape %clone_shape; },
              goto_shape:{
                  lua_table::WriteShape(%table,NullableShapeDecodeSome(%goto_action));
                  Write<LuaValue::TYPE>(LocateNewSlot(%table,%slot),%elem); },
              clone_shape:{
                  %new_shape_ref=b::AllocSized<LuaShapeReference::TYPE>();
                  %new_shape=b::Deref<LuaShapeReference::TYPE>(%new_shape_ref);
                  CloneLuaShape(%new_shape,%shape);
                  b::Drop<LuaShapeReference::TYPE>(lua_table::ReadShape(%table));
                  lua_table::WriteShape(%table,b::Clone<LuaShapeReference::TYPE>(%new_shape_ref));
                  %slot = InsertField(%new_shape,%key);
                  InsertAction(%shape,%key,b::Clone<LuaShapeReference::TYPE>(%new_shape_ref),%slot);
                  Write<LuaValue::TYPE>(LocateNewSlot(%table,%slot),%elem); },
    }
}
type NullableLuaValueArrayDecodeSome = nullable_pointer::DecodeSomeUnchecked<UnsizedArray<LuaValue>>;
type NullableLuaValueArrayEncodeSome = nullable_pointer::EncodeSome<UnsizedArray<LuaValue>>;
type NullableLuaValueArrayIsSome = nullable_pointer::IsSome<UnsizedArray<LuaValue>>;
make_instruction! {
    LocateSlot->fn(object:Pointer<LuaTable>,slot:Usize)->(o:Pointer<LuaValue>){
        entry:{
            %fast_len = GetLength<UnsizedArray::<LuaValue>::TYPE>(lua_table::LocateFastFields(%object));
            if UsizeLarge(%fast_len,%slot) %fast %slow;
        },
        fast:{ %o = b::LocateElement<UnsizedArray::<LuaValue>::TYPE>(lua_table::LocateFastFields(%object),%slot); },
        slow:{ %o = b::LocateElement<UnsizedArray::<LuaValue>::TYPE>(NullableLuaValueArrayDecodeSome(lua_table::ReadSlowFields(%object)),UsizeSub(%slot,%fast_len)); }
    }
}
type ReadLuaValueArray = e::ReadElement<LuaValue, UnsizedArray<LuaValue>>;
type WriteLuaValueArray = e::WriteElement<LuaValue, UnsizedArray<LuaValue>>;
make_instruction! {
    LocateNewSlot->fn(object:Pointer<LuaTable>,slot:Usize)->(o:Pointer<LuaValue>){
        entry:{
            %fast_len = GetLength<UnsizedArray::<LuaValue>::TYPE>(lua_table::LocateFastFields(%object));
            if UsizeLarge(%fast_len,%slot) %fast %slow;
        },
        fast:{ %o = b::LocateElement<UnsizedArray::<LuaValue>::TYPE>(lua_table::LocateFastFields(%object),%slot); },
        slow:{ if NullableLuaValueArrayIsSome(lua_table::ReadSlowFields(%object)) %no_alloc %alloc; },
        alloc:{
           %default_slow_count = b::IntTruncate<12,7>(7);
           %slow_field_vec=b::NonGCAllocUnsized<LuaValueArrayReference::TYPE>(%default_slow_count);
           %i=b::IntTruncate<12,7>(0);
           branch %alloc_fill;
        },
        alloc_fill:{
           phi %i:Usize={%alloc=>%i,%alloc_fill=>%i1};
           WriteLuaValueArray(%slow_field_vec,%i,ConstNil());
           %i1=UsizeAdd(%i,b::IntTruncate<12,7>(1));
           if UsizeLe(%i1,%default_slow_count) %alloc_fill %alloc_write;
        },
        alloc_write:{
            lua_table::WriteSlowFields(%object,NullableLuaValueArrayEncodeSome(%slow_field_vec));
            %o = b::LocateElement<UnsizedArray::<LuaValue>::TYPE>(%slow_field_vec,UsizeSub(%slot,%fast_len));
        },
        no_alloc:{
           %slow_fields=NullableLuaValueArrayDecodeSome(lua_table::ReadSlowFields(%object));
           %slow_len=b::GetLength<UnsizedArray::<LuaValue>::TYPE>(%slow_fields);
           if UsizeLt(UsizeSub(%slot,%fast_len),%slow_len) %no_grow %grow;
        },
        no_grow:{ %o = b::LocateElement<UnsizedArray::<LuaValue>::TYPE>(%slow_fields,UsizeSub(%slot,%fast_len)); },
        grow:{
            %new_slow_count = UsizeAdd(UsizeShl(%slow_len,b::IntTruncate<12,7>(1)),b::IntTruncate<12,7>(1));
            %new_slow_field_vec=b::NonGCAllocUnsized<LuaValueArrayReference::TYPE>(%new_slow_count);
            %i=b::IntTruncate<12,7>(0);
            branch %copy;
        },
        copy:{
           phi %i:Usize={%grow=>%i,%copy=>%i1};
           WriteLuaValueArray(%new_slow_field_vec,%i,ReadLuaValueArray(%slow_fields,%i));
           %i1=UsizeAdd(%i,b::IntTruncate<12,7>(1));
           if UsizeLe(%i1,%slow_len) %copy %copy_complete;
        },
        copy_complete:{ branch %fill; },
        fill:{
           phi %i1:Usize={%copy_complete=>%i1,%fill=>%i2};
           WriteLuaValueArray(%new_slow_field_vec,%i1,ConstNil());
           %i2=UsizeAdd(%i1,b::IntTruncate<12,7>(1));
           if UsizeLe(%i,%new_slow_count) %fill %free;
        },
        free:{
            lua_table::WriteSlowFields(%object,NullableLuaValueArrayEncodeSome(%new_slow_field_vec));
            b::NonGCFree<LuaValueArrayReference::TYPE>(%slow_fields);
            %o = b::LocateElement<UnsizedArray::<LuaValue>::TYPE>(%new_slow_field_vec,UsizeSub(%slot,%fast_len));
        },
    }
}
make_instruction! {
    GetField->{<const field:LuaValue,mut cache:InlineCacheLine,mut number_of_continuous_miss:U8>(object:LuaValue)->(o:LuaValue){
        Init:{
            entry:{ if lua_value::IsTable(%object) %is_object %is_not_object; },
            is_object:{
                %number_of_continuous_miss_value = U8Add(b::Read<U8::TYPE>(%number_of_continuous_miss),b::IntTruncate<2,7>(1));
                if U8Lt(%number_of_continuous_miss_value,b::IntTruncate<2,7>(8)) %do_not_use_cache %set_cache; },
            do_not_use_cache:{ %o =GetElement<%cache>(%object,%field); },
            set_cache:{ SetState<%Cached>(); %o =GetElement<%cache>(%object,%field); },
            is_not_object:{ %o=GetElement<%cache>(%object,%field); },
        },
        Cached:{
            entry:{ if lua_value::IsTable(%object) %is_object %is_not_object; },
            is_object:{
                %value=GetByCache<%cache>(b::Deref<LuaTableReference::TYPE>(lua_value::DecodeTableUnchecked(%object)));
                if lua_value::IsNil(%value) %miss %hit; },
                hit:{
                    b::Write<U8::TYPE>(%number_of_continuous_miss,b::IntTruncate<2,7>(0));
                    %o=b::Move<LuaValue::TYPE>(%value); },
                miss:{
                    %number_of_continuous_miss_value = U8Add(b::Read<U8::TYPE>(%number_of_continuous_miss),b::IntTruncate<2,7>(1));
                    %o = GetElement<%cache>(%object,%field);
                    if U8Lt(%number_of_continuous_miss_value,b::IntTruncate<2,7>(8)) %clean_cache %update_cache; },
            clean_cache:{ SetState<%Init>(); },
            update_cache:{},
            is_not_object:{ %o=GetElement<%cache>(%object,%field); },
        },
    }}
}
make_instruction! {
    SetField->{<const field:LuaValue,mut cache:InlineCacheLine,mut number_of_continuous_miss:U8>(object:LuaValue,value:LuaValue){
        Init:{
            entry:{ if lua_value::IsTable(%object) %is_object %is_not_object; },
            is_object:{
                %number_of_continuous_miss_value = U8Add(b::Read<U8::TYPE>(%number_of_continuous_miss),b::IntTruncate<2,7>(1));
                SetElement<%cache>(%object,%field,%value);
                if U8Lt(%number_of_continuous_miss_value,b::IntTruncate<2,7>(8)) %do_not_use_cache %set_cache; },
            do_not_use_cache:{},
            set_cache:{ SetState<%Cached>();  },
            is_not_object:{SetElement<%cache>(%object,%field,%value);},
        },
        Cached:{
            entry:{ if lua_value::IsTable(%object) %is_object %is_not_object; },
            is_object:{
                %cached=SetByCache<%cache>(b::Deref<LuaTableReference::TYPE>(lua_value::DecodeTableUnchecked(%object)),%value);
                if BoolNot(%cached) %miss %hit; },
                hit:{
                    b::Write<U8::TYPE>(%number_of_continuous_miss,b::IntTruncate<2,7>(0));
                    %o=b::Move<LuaValue::TYPE>(%value); },
                miss:{
                    %number_of_continuous_miss_value = U8Add(b::Read<U8::TYPE>(%number_of_continuous_miss),b::IntTruncate<2,7>(1));
                    SetElement<%cache>(%object,%field,%value);
                    if U8Lt(%number_of_continuous_miss_value,b::IntTruncate<2,7>(8)) %clean_cache %update_cache; },
            clean_cache:{ SetState<%Init>(); },
            update_cache:{},
            is_not_object:{SetElement<%cache>(%object,%field,%value);},
        },
    }}
}
make_instruction! {
    GetGlobal->fn<const field:LuaValue,mut cache:InlineCacheLine>(state:LuaStateReference)->(o:LuaValue){
        entry:{
            %value=GetByCache<%cache>(b::Deref<LuaTableReference::TYPE>(lua_state::ReadGlobal(b::Deref<LuaStateReference::TYPE>(%state))));
            if lua_value::IsNil(%value) %miss %hit; },
        hit:{ %o=b::Move<LuaValue::TYPE>(%value); },
        miss:{ %o=GetElement<%cache>(lua_value::EncodeTable(lua_state::ReadGlobal(b::Deref<LuaStateReference::TYPE>(%state))),%field); },
    }
}
make_instruction! {
    SetGlobal->fn<const field:LuaValue,mut cache:InlineCacheLine>(state:LuaStateReference,value:LuaValue){
        entry:{
            %cached=SetByCache<%cache>(b::Deref<LuaTableReference::TYPE>(lua_state::ReadGlobal(b::Deref<LuaStateReference::TYPE>(%state))),%value);
            if BoolNot(%cached) %miss %hit; },
        hit:{ },
        miss:{ SetElement<%cache>(lua_value::EncodeTable(lua_state::ReadGlobal(b::Deref<LuaStateReference::TYPE>(%state))),%field,%value); },
    }
}
make_instruction! {Return->fn(r:Pointer<UnsizedArray<LuaValue>>){entry:{
    b::Return<Pointer::<UnsizedArray<LuaValue>>::TYPE>(%r);
}}}
make_instruction! {Return0->fn(){entry:{
    %array=b::StackAllocUnsized<UnsizedArray::<LuaValue>::TYPE>(b::IntTruncate<12,7>(0));
    b::SetLength<UnsizedArray::<LuaValue>::TYPE>(%array,b::IntTruncate<12,7>(0));
    b::Return<Pointer::<UnsizedArray<LuaValue>>::TYPE>(%array);
}}}
make_instruction! {Return1->fn(r0:LuaValue){entry:{
    %array=b::StackAllocUnsized<UnsizedArray::<LuaValue>::TYPE>(b::IntTruncate<12,7>(1));
    b::SetLength<UnsizedArray::<LuaValue>::TYPE>(%array,b::IntTruncate<12,7>(1));
    LuaValueArraySet(%array,b::IntTruncate<12,7>(0),%r0);
    b::Return<Pointer::<UnsizedArray<LuaValue>>::TYPE>(%array);
}}}
make_instruction! {Return2->fn(r0:LuaValue,r1:LuaValue){entry:{
    %array=b::StackAllocUnsized<UnsizedArray::<LuaValue>::TYPE>(b::IntTruncate<12,7>(2));
    b::SetLength<UnsizedArray::<LuaValue>::TYPE>(%array,b::IntTruncate<12,7>(2));
    LuaValueArraySet(%array,b::IntTruncate<12,7>(0),%r0);
    LuaValueArraySet(%array,b::IntTruncate<12,7>(1),%r1);
    b::Return<Pointer::<UnsizedArray<LuaValue>>::TYPE>(%array);
}}}
make_instruction! {Return3->fn(r0:LuaValue,r1:LuaValue,r2:LuaValue){entry:{
    %array=b::StackAllocUnsized<UnsizedArray::<LuaValue>::TYPE>(b::IntTruncate<12,7>(3));
    b::SetLength<UnsizedArray::<LuaValue>::TYPE>(%array,b::IntTruncate<12,7>(3));
    LuaValueArraySet(%array,b::IntTruncate<12,7>(0),%r0);
    LuaValueArraySet(%array,b::IntTruncate<12,7>(1),%r1);
    LuaValueArraySet(%array,b::IntTruncate<12,7>(2),%r2);
    b::Return<Pointer::<UnsizedArray<LuaValue>>::TYPE>(%array);
}}}
make_instruction! {Return0VaSlice->fn(va_rets:Slice<LuaValue>){entry:{
    %array=b::StackAllocUnsized<UnsizedArray::<LuaValue>::TYPE>(UsizeAdd(b::IntTruncate<12,7>(0),LuaValueSliceLen(%va_rets)));
    b::SetLength<UnsizedArray::<LuaValue>::TYPE>(%array,UsizeAdd(b::IntTruncate<12,7>(0),LuaValueSliceLen(%va_rets)));
    LuaValueSliceCopy(LuaValueSubSlice(UnsizedLuaValueArrayToSlice(%array),b::IntTruncate<12,7>(0),LuaValueSliceLen(%va_rets)),%va_rets);
    b::Return<Pointer::<UnsizedArray<LuaValue>>::TYPE>(%array);
}}}
make_instruction! {Return1VaSlice->fn(r0:LuaValue,va_rets:Slice<LuaValue>){entry:{
    %array=b::StackAllocUnsized<UnsizedArray::<LuaValue>::TYPE>(UsizeAdd(b::IntTruncate<12,7>(1),LuaValueSliceLen(%va_rets)));
    b::SetLength<UnsizedArray::<LuaValue>::TYPE>(%array,UsizeAdd(b::IntTruncate<12,7>(1),LuaValueSliceLen(%va_rets)));
    LuaValueSliceCopy(LuaValueSubSlice(UnsizedLuaValueArrayToSlice(%array),b::IntTruncate<12,7>(1),LuaValueSliceLen(%va_rets)),%va_rets);
    LuaValueArraySet(%array,b::IntTruncate<12,7>(0),%r0);
    b::Return<Pointer::<UnsizedArray<LuaValue>>::TYPE>(%array);
}}}
make_instruction! {Return2VaSlice->fn(r0:LuaValue,r1:LuaValue,va_rets:Slice<LuaValue>){entry:{
    %array=b::StackAllocUnsized<UnsizedArray::<LuaValue>::TYPE>(UsizeAdd(b::IntTruncate<12,7>(2),LuaValueSliceLen(%va_rets)));
    b::SetLength<UnsizedArray::<LuaValue>::TYPE>(%array,UsizeAdd(b::IntTruncate<12,7>(1),LuaValueSliceLen(%va_rets)));
    LuaValueSliceCopy(LuaValueSubSlice(UnsizedLuaValueArrayToSlice(%array),b::IntTruncate<12,7>(2),LuaValueSliceLen(%va_rets)),%va_rets);
    LuaValueArraySet(%array,b::IntTruncate<12,7>(0),%r0);
    LuaValueArraySet(%array,b::IntTruncate<12,7>(1),%r1);
    b::Return<Pointer::<UnsizedArray<LuaValue>>::TYPE>(%array);
}}}
make_instruction! {Return3VaSlice->fn(r0:LuaValue,r1:LuaValue,r2:LuaValue,va_rets:Slice<LuaValue>){entry:{
    %array=b::StackAllocUnsized<UnsizedArray::<LuaValue>::TYPE>(UsizeAdd(b::IntTruncate<12,7>(3),LuaValueSliceLen(%va_rets)));
    b::SetLength<UnsizedArray::<LuaValue>::TYPE>(%array,UsizeAdd(b::IntTruncate<12,7>(3),LuaValueSliceLen(%va_rets)));
    LuaValueSliceCopy(LuaValueSubSlice(UnsizedLuaValueArrayToSlice(%array),b::IntTruncate<12,7>(3),LuaValueSliceLen(%va_rets)),%va_rets);
    LuaValueArraySet(%array,b::IntTruncate<12,7>(0),%r0);
    LuaValueArraySet(%array,b::IntTruncate<12,7>(1),%r1);
    LuaValueArraySet(%array,b::IntTruncate<12,7>(2),%r2);
    b::Return<Pointer::<UnsizedArray<LuaValue>>::TYPE>(%array);
}}}
make_instruction! {ReturnVaSlice->fn(rets:Slice<LuaValue>,va_rets:Slice<LuaValue>){entry:{
    %array=b::StackAllocUnsized<UnsizedArray::<LuaValue>::TYPE>(UsizeAdd(LuaValueSliceLen(%rets),LuaValueSliceLen(%va_rets)));
    b::SetLength<UnsizedArray::<LuaValue>::TYPE>(%array,UsizeAdd(LuaValueSliceLen(%rets),LuaValueSliceLen(%va_rets)));
    LuaValueSliceCopy(LuaValueSubSlice(UnsizedLuaValueArrayToSlice(%array),LuaValueSliceLen(%rets),LuaValueSliceLen(%va_rets)),%va_rets);
    LuaValueSliceCopy(LuaValueSubSlice(UnsizedLuaValueArrayToSlice(%array),b::IntTruncate<12,7>(0),LuaValueSliceLen(%rets)),%rets);
    b::Return<Pointer::<UnsizedArray<LuaValue>>::TYPE>(%array);
}}}
make_instruction! {ReturnVA->fn(rets:Slice<LuaValue>,va_rets:Pointer<UnsizedArray<LuaValue>>){entry:{
    ReturnVaSlice(%rets,UnsizedLuaValueArrayToSlice(%va_rets));
}}}
make_instruction! {Return0VA->fn(va_rets:Pointer<UnsizedArray<LuaValue>>){entry:{
    Return0VaSlice(UnsizedLuaValueArrayToSlice(%va_rets));
}}}
make_instruction! {Return1VA->fn(r0:LuaValue,va_rets:Pointer<UnsizedArray<LuaValue>>){entry:{
    Return1VaSlice(%r0,UnsizedLuaValueArrayToSlice(%va_rets));
}}}
make_instruction! {Return2VA->fn(r0:LuaValue,r1:LuaValue,va_rets:Pointer<UnsizedArray<LuaValue>>){entry:{
    Return2VaSlice(%r0,%r1,UnsizedLuaValueArrayToSlice(%va_rets));
}}}
make_instruction! {Return3VA->fn(r0:LuaValue,r1:LuaValue,r2:LuaValue,va_rets:Pointer<UnsizedArray<LuaValue>>){entry:{
    Return3VaSlice(%r0,%r1,%r2,UnsizedLuaValueArrayToSlice(%va_rets));
}}}
make_instruction! {
    GetRet0->fn(rets:Pointer<UnsizedArray<LuaValue>>)->(r:LuaValue){
        entry:{
            if UsizeGt(b::IntTruncate<12,7>(0),b::GetLength<UnsizedArray::<LuaValue>::TYPE>(%rets)) %empty %not_empty; },
        empty:{%r=ConstNil();},
        not_empty:{%r=LuaValueArrayGet(%rets,b::IntTruncate<12,7>(0));},
    }
}
make_instruction! {
    DoGetRet->fn(rets:Pointer<UnsizedArray<LuaValue>>,index:Usize)->(r:LuaValue){
        entry:{
            if UsizeGt(%index,b::GetLength<UnsizedArray::<LuaValue>::TYPE>(%rets)) %empty %not_empty; },
        empty:{%r=ConstNil();},
        not_empty:{%r=LuaValueArrayGet(%rets,%index);},
    }
}
make_instruction! {
    GetRet->fn<const index:Usize>(rets:Pointer<UnsizedArray<LuaValue>>)->(r:LuaValue){
        entry:{
            if UsizeGt(%index,b::GetLength<UnsizedArray::<LuaValue>::TYPE>(%rets)) %empty %not_empty; },
        empty:{%r=ConstNil();},
        not_empty:{%r=LuaValueArrayGet(%rets,%index);},
    }
}
make_instruction! {
    GetArg->fn<const index:Usize>(args:Slice<LuaValue>)->(r:LuaValue){
        entry:{
            if UsizeGt(%index,LuaValueSliceLen(%args)) %empty %not_empty; },
        empty:{%r=ConstNil();},
        not_empty:{%r=LuaValueSliceGet(%args,%index);},
    }
}
make_instruction! {GetVaArgs->fn<const index:Usize>(args:Slice<LuaValue>)->(va_args:Slice<LuaValue>){ entry:{
        %va_args=LuaValueSubSlice(%args,%index,UsizeSub(LuaValueSliceLen(%args),%index));
}}}
#[make_native_function(PrintDebug)]
pub extern "C" fn __vm_lua_lib_print_debug(value: Direct<LuaValue>) {
    let mut buffer = Vec::new();
    unsafe {
        extend_to_buffer(&mut buffer, value);
    }
    debug!("{}", String::from_utf8_lossy(&buffer));
}
#[make_native_function(BreakPoint)]
pub extern "C" fn __vm_lua_lib_break_point() { let _a = 0; }
