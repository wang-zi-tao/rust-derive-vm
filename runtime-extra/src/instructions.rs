use std::marker::PhantomData;

use crate::ty::*;
use jvm_core::{Pointer, Slice, TypeDeclaration, UnsizedArray};
use runtime::instructions::{bootstrap as b, bootstrap::*};
macro_rules! declare_instruction {
    (Const,$ty:ty,$name:ident) => {
        make_instruction!($name->fn<const value:$ty>()->(o:$ty){
          entry:{
            %o=b::Move<$ty::TYPE>(%value);
          }
        });
    };
    (Unique,$generic:expr ,$ty:ty, $instructions:ident , $name:ident) => {
        make_instruction!($name->fn(i1:$ty)->(i1:$ty){
          entry:{
            %i1=$instructions<$generic>(%i1);
          }
        });
    };
    (Binary,$generic:expr ,$ty:ty, $instructions:ident , $name:ident) => {
        make_instruction!($name->fn(i1:$ty,i2:$ty)->(i2:$ty){
          entry:{
            %i2=$instructions<$generic>(%i1,%i2);
          }
        });
    };
    (BinaryCompare,$generic:expr ,$ty:ty, $instructions:ident , $name:ident) => {
        make_instruction!($name->fn(i1:$ty,i2:$ty)->(o:Bool){
          entry:{
            %o=$instructions<$generic>(%i1,%i2);
          }
        });
    };
    (IntBinary,$name:ident,$name_8:ident,$name_16:ident,$name_32:ident,$name_64:ident,$name_size:ident) => {
      declare_instruction!(Binary,1,I8,$name , $name_8);
      declare_instruction!(Binary,3,I16,$name , $name_16);
      declare_instruction!(Binary,5,I32,$name , $name_32);
      declare_instruction!(Binary,7,I64,$name , $name_64);
      declare_instruction!(Binary,11,Isize,$name , $name_size);
    };
    (IntUnique,$name:ident,$name_8:ident,$name_16:ident,$name_32:ident,$name_64:ident,$name_size:ident) => {
      declare_instruction!(Unique,1,I8,$name , $name_8);
      declare_instruction!(Unique,3,I16,$name , $name_16);
      declare_instruction!(Unique,5,I32,$name , $name_32);
      declare_instruction!(Unique,7,I64,$name , $name_64);
      declare_instruction!(Unique,11,Isize,$name , $name_size);
    };
    (UIntUnique,$name:ident,$name_8:ident,$name_16:ident,$name_32:ident,$name_64:ident,$name_size:ident) => {
      declare_instruction!(Unique,2,U8,$name , $name_8);
      declare_instruction!(Unique,4,U16,$name , $name_16);
      declare_instruction!(Unique,6,U32,$name , $name_32);
      declare_instruction!(Unique,8,U64,$name , $name_64);
      declare_instruction!(Unique,12,Usize,$name , $name_size);
    };
    (UIntBinary,$name:ident,$name_8:ident,$name_16:ident,$name_32:ident,$name_64:ident,$name_size:ident) => {
      declare_instruction!(Binary,2,U8,$name , $name_8);
      declare_instruction!(Binary,4,U16,$name , $name_16);
      declare_instruction!(Binary,6,U32,$name , $name_32);
      declare_instruction!(Binary,8,U64,$name , $name_64);
      declare_instruction!(Binary,12,Usize,$name , $name_size);
    };
    (FloatUnique,$name:ident,$name_32:ident,$name_64:ident) => {
      declare_instruction!(Unique,32,F32,$name , $name_32);
      declare_instruction!(Unique,64,F64,$name , $name_64);
    };
    (FloatBinary,$name:ident,$name_32:ident,$name_64:ident) => {
      declare_instruction!(Binary,32,F32,$name , $name_32);
      declare_instruction!(Binary,64,F64,$name , $name_64);
    };
    (IntCompare,$name:ident,$name_8:ident,$name_16:ident,$name_32:ident,$name_64:ident,$name_size:ident) => {
      declare_instruction!(BinaryCompare,1,I8,$name , $name_8);
      declare_instruction!(BinaryCompare,3,I16,$name , $name_16);
      declare_instruction!(BinaryCompare,5,I32,$name , $name_32);
      declare_instruction!(BinaryCompare,7,I64,$name , $name_64);
      declare_instruction!(BinaryCompare,11,Isize,$name , $name_size);
    };
    (UIntCompare,$name:ident,$name_8:ident,$name_16:ident,$name_32:ident,$name_64:ident,$name_size:ident) => {
      declare_instruction!(BinaryCompare,2,U8,$name , $name_8);
      declare_instruction!(BinaryCompare,4,U16,$name , $name_16);
      declare_instruction!(BinaryCompare,6,U32,$name , $name_32);
      declare_instruction!(BinaryCompare,8,U64,$name , $name_64);
      declare_instruction!(BinaryCompare,12,Usize,$name , $name_size);
    };
    (FloatCompare,$name:ident,$name_32:ident,$name_64:ident) => {
      declare_instruction!(BinaryCompare,32,F32,$name , $name_32);
      declare_instruction!(BinaryCompare,64,F64,$name , $name_64);
    };
}
make_instruction!(TODO->fn(){ entry:{ }});
declare_instruction!(UIntBinary, Add, U8Add, U16Add, U32Add, U64Add, UsizeAdd);
declare_instruction!(UIntBinary, Sub, U8Sub, U16Sub, U32Sub, U64Sub, UsizeSub);
declare_instruction!(UIntBinary, Mul, U8Mul, U16Mul, U32Mul, U64Mul, UsizeMul);
declare_instruction!(UIntBinary, Div, U8Div, U16Div, U32Div, U64Div, UsizeDiv);
declare_instruction!(UIntBinary, Rem, U8Rem, U16Rem, U32Rem, U64Rem, UsizeRem);
declare_instruction!(UIntBinary, And, U8And, U16And, U32And, U64And, UsizeAnd);
declare_instruction!(UIntBinary, Or, U8Or, U16Or, U32Or, U64Or, UsizeOr);
declare_instruction!(UIntBinary, Xor, U8Xor, U16Xor, U32Xor, U64Xor, UsizeXor);
declare_instruction!(UIntBinary, Shl, U8Shl, U16Shl, U32Shl, U64Shl, UsizeShl);
declare_instruction!(UIntBinary, Ushr, U8Shr, U16Shr, U32Shr, U64Shr, UsizeShr);
declare_instruction!(UIntUnique, Neg, U8Neg, U16Neg, U32Neg, U64Neg, UsizeNeg);
declare_instruction!(UIntUnique, Not, U8Not, U16Not, U32Not, U64Not, UsizeNot);
declare_instruction!(UIntBinary, Shl, U8LeftShift, U16LeftShift, U32LeftShift, U64LeftShift, UsizeLeftShift);
declare_instruction!(UIntBinary, Ushr, U8RightShift, U16RightShift, U32RightShift, U64RightShift, UsizeRightShift);

declare_instruction!(UIntCompare, UcmpLt, U8Lt, U16Lt, U32Lt, U64Lt, UsizeLt);
declare_instruction!(UIntCompare, UcmpLe, U8Le, U16Le, U32Le, U64Le, UsizeLe);
declare_instruction!(UIntCompare, UcmpGt, U8Gt, U16Gt, U32Gt, U64Gt, UsizeGt);
declare_instruction!(UIntCompare, UcmpGe, U8Ge, U16Ge, U32Ge, U64Ge, UsizeGe);
declare_instruction!(UIntCompare, UcmpEq, U8Eq, U16Eq, U32Eq, U64Eq, UsizeEq);
declare_instruction!(UIntCompare, UcmpNe, U8Ne, U16Ne, U32Ne, U64Ne, UsizeNe);

declare_instruction!(UIntCompare, UcmpLt, U8Less, U16Less, U32Less, U64Less, UsizeLess);
declare_instruction!(UIntCompare, UcmpLe, U8LessOrEqual, U16LessOrEqual, U32LessOrEqual, U64LessOrEqual, UsizeLessOrEqual);
declare_instruction!(UIntCompare, UcmpGt, U8Large, U16Large, U32Large, U64Large, UsizeLarge);
declare_instruction!(UIntCompare, UcmpGe, U8LargeOrEqual, U16LargeOrEqual, U32LargeOrEqual, U64LargeOrEqual, UsizeLargeOrEqual);
declare_instruction!(UIntCompare, UcmpEq, U8Equal, U16Equal, U32Equal, U64Equal, UsizeEqual);
declare_instruction!(UIntCompare, UcmpNe, U8NotEqual, U16NotEqual, U32NotEqual, U64NotEqual, UsizeNotEqual);

declare_instruction!(Binary, 0, Bool, And, BoolAnd);
declare_instruction!(Binary, 0, Bool, Or, BoolOr);
declare_instruction!(Binary, 0, Bool, Xor, BoolXor);
make_instruction!(BoolNot->fn(i:Bool)->(o:Bool){ entry:{
    %o=Not<0>(%i);
}});

declare_instruction!(IntBinary, Add, I8Add, I16Add, I32Add, I64Add, IsizeAdd);
declare_instruction!(IntBinary, Sub, I8Sub, I16Sub, I32Sub, I64Sub, IsizeSub);
declare_instruction!(IntBinary, Mul, I8Mul, I16Mul, I32Mul, I64Mul, IsizeMul);
declare_instruction!(IntBinary, Div, I8Div, I16Div, I32Div, I64Div, IsizeDiv);
declare_instruction!(IntBinary, Rem, I8Rem, I16Rem, I32Rem, I64Rem, IsizeRem);
declare_instruction!(IntBinary, And, I8And, I16And, I32And, I64And, IsizeAnd);
declare_instruction!(IntBinary, Or, I8Or, I16Or, I32Or, I64Or, IsizeOr);
declare_instruction!(IntBinary, Xor, I8Xor, I16Xor, I32Xor, I64Xor, IsizeXor);
declare_instruction!(IntBinary, Shl, I8Shl, I16Shl, I32Shl, I64Shl, IsizeShl);
declare_instruction!(IntBinary, Shr, I8Shr, I16Shr, I32Shr, I64Shr, IsizeShr);
declare_instruction!(IntUnique, Neg, I8Neg, I16Neg, I32Neg, I64Neg, IsizeNeg);
declare_instruction!(IntUnique, Not, I8Not, I16Not, I32Not, I64Not, IsizeNot);

declare_instruction!(IntBinary, Shl, I8LeftShift, I16LeftShift, I32LeftShift, I64LeftShift, IsizeLeftShift);
declare_instruction!(IntBinary, Shr, I8RightShift, I16RightShift, I32RightShift, I64RightShift, IsizeRightShift);

declare_instruction!(IntCompare, CmpLt, I8Lt, I16Lt, I32Lt, I64Lt, IsizeLt);
declare_instruction!(IntCompare, CmpLe, I8Le, I16Le, I32Le, I64Le, IsizeLe);
declare_instruction!(IntCompare, CmpGt, I8Gt, I16Gt, I32Gt, I64Gt, IsizeGt);
declare_instruction!(IntCompare, CmpGe, I8Ge, I16Ge, I32Ge, I64Ge, IsizeGe);
declare_instruction!(IntCompare, CmpEq, I8Eq, I16Eq, I32Eq, I64Eq, IsizeEq);
declare_instruction!(IntCompare, CmpNe, I8Ne, I16Ne, I32Ne, I64Ne, IsizeNe);

declare_instruction!(IntCompare, CmpLt, I8Less, I16Less, I32Less, I64Less, IsizeLess);
declare_instruction!(IntCompare, CmpLe, I8LessOrEqual, I16LessOrEqual, I32LessOrEqual, I64LessOrEqual, IsizeLessOrEqual);
declare_instruction!(IntCompare, CmpGt, I8Large, I16Large, I32Large, I64Large, IsizeLarge);
declare_instruction!(IntCompare, CmpGe, I8LargeOrEqual, I16LargeOrEqual, I32LargeOrEqual, I64LargeOrEqual, IsizeLargeOrEqual);
declare_instruction!(IntCompare, CmpEq, I8Equal, I16Equal, I32Equal, I64Equal, IsizeEqual);
declare_instruction!(IntCompare, CmpNe, I8NotEqual, I16NotEqual, I32NotEqual, I64NotEqual, IsizeNotEqual);

declare_instruction!(FloatBinary, FAdd, F32Add, F64Add);
declare_instruction!(FloatBinary, FSub, F32Sub, F64Sub);
declare_instruction!(FloatBinary, FMul, F32Mul, F64Mul);
declare_instruction!(FloatBinary, FDiv, F32Div, F64Div);
declare_instruction!(FloatBinary, FRem, F32Rem, F64Rem);
declare_instruction!(FloatUnique, FNeg, F32Neg, F64Neg);

declare_instruction!(FloatCompare, FcmpLt, F32Lt, F64Lt);
declare_instruction!(FloatCompare, FcmpLe, F32Le, F64Le);
declare_instruction!(FloatCompare, FcmpGt, F32Gt, F64Gt);
declare_instruction!(FloatCompare, FcmpGe, F32Ge, F64Ge);
declare_instruction!(FloatCompare, FcmpEq, F32Eq, F64Eq);
declare_instruction!(FloatCompare, FcmpNe, F32Ne, F64Ne);

declare_instruction!(FloatCompare, CmpLt, F32Less, F64Less);
declare_instruction!(FloatCompare, CmpLe, F32LessOrEqual, F64LessOrEqual);
declare_instruction!(FloatCompare, CmpGt, F32Large, F64Large);
declare_instruction!(FloatCompare, CmpGe, F32LargeOrEqual, F64LargeOrEqual);
declare_instruction!(FloatCompare, CmpEq, F32Equal, F64Equal);
declare_instruction!(FloatCompare, CmpNe, F32NotEqual, F64NotEqual);

declare_instruction!(Const, Bool, BoolConst);

declare_instruction!(Const, I8, I8Const);
declare_instruction!(Const, I16, I16Const);
declare_instruction!(Const, I32, I32Const);
declare_instruction!(Const, I64, I64Const);

declare_instruction!(Const, U8, U8Const);
declare_instruction!(Const, U16, U16Const);
declare_instruction!(Const, U32, U32Const);
declare_instruction!(Const, U64, U64Const);

declare_instruction!(Const, F32, F32Const);
declare_instruction!(Const, F64, F64Const);

make_instruction!(I64AsF64->fn(i:I64)->(o:F64){
  entry:{
    %o=CastUnchecked<F64::TYPE,I64::TYPE>(%i);
  }
});
make_instruction!(I32AsF32->fn(i:I32)->(o:F32){
  entry:{
    %o=CastUnchecked<F32::TYPE,I32::TYPE>(%i);
  }
});
make_instruction!(F64AsI64->fn(i:F64)->(o:I64){
  entry:{
    %o=CastUnchecked<I64::TYPE,F64::TYPE>(%i);
  }
});
make_instruction!(F32AsI32->fn(i:F32)->(o:I32){
  entry:{
    %o=CastUnchecked<I32::TYPE,F32::TYPE>(%i);
  }
});

make_instruction!(I64ToF64->fn(i:I64)->(o:F64){
  entry:{
    %o=IntToFloat<64,7>(%i);
  }
});
make_instruction!(I32ToF32->fn(i:I32)->(o:F32){
  entry:{
    %o=IntToFloat<32,5>(%i);
  }
});
make_instruction!(F64ToI64->fn(i:F64)->(o:I64){
  entry:{
    %o=FloatToInt<7,64>(%i);
  }
});
make_instruction!(F32ToI32->fn(i:F32)->(o:I32){
  entry:{
    %o=FloatToInt<5,32>(%i);
  }
});
make_instruction!(I64Inc->fn(i:I64)->(o:I64){
  entry:{
    %o=I64Add(%i,1);
  }
});
make_instruction!(F64Inc->fn(i:F64)->(o:F64){
  entry:{
    %o=F64Add(%i,1);
  }
});

#[derive(Instruction)]
#[instruction(LocateFieldFor->fn(ptr:Pointer<Struct>)->(field:Pointer<Field>){ entry:{
        %field = LocateField<Struct::TYPE,INDEX>(%ptr);
    }},
    INDEX={kind=Int}
)]
pub struct LocateFieldFor<Struct: TypeDeclaration, Field: TypeDeclaration, const INDEX: i64>(PhantomData<Struct>, PhantomData<Field>);

#[derive(Instruction)]
#[instruction(ReadFieldFor->fn(ptr:Pointer<Struct>)->(field:Field){ entry:{
        %field = Read<Field::TYPE>(LocateField<Struct::TYPE,INDEX>(%ptr));
    }},
    INDEX={kind=Int}
)]
pub struct ReadFieldFor<Struct: TypeDeclaration, Field: TypeDeclaration, const INDEX: i64>(PhantomData<(Field, Struct)>);

#[derive(Instruction)]
#[instruction(WriteFieldFor->fn(ptr:Pointer<Struct>,field:Field){ entry:{
        Write<Field::TYPE>(LocateField<Struct::TYPE,INDEX>(%ptr),%field);
    }},
    INDEX={kind=Int}
)]
pub struct WriteFieldFor<Struct: TypeDeclaration, Field: TypeDeclaration, const INDEX: i64>(PhantomData<(Field, Struct)>);

#[derive(Instruction)]
#[instruction(LocateFieldFor->fn(ptr:Pointer<Struct>)->(field:Field){ entry:{
        %field = GetField<Struct::TYPE,INDEX>(%ptr);
    }},
    INDEX={kind=Int}
)]
pub struct GetFieldFor<Struct: TypeDeclaration, Field: TypeDeclaration, const INDEX: i64>(PhantomData<(Field, Struct)>);

#[derive(Instruction)]
#[instruction(LocateFieldFor->fn(struct:Struct,field:Field)->(struct:Struct){ entry:{
        %struct = SetField<Struct::TYPE,INDEX>(%struct,%field);
    }},
    INDEX={kind=Int}
)]
pub struct SetFieldFor<Struct: TypeDeclaration, Field: TypeDeclaration, const INDEX: i64>(PhantomData<(Field, Struct)>);

#[derive(Instruction)]
#[instruction(ReadTagFor->fn(enum:Pointer<Enum>)->(tag:Usize){ entry:{
        %tag = ReadTag<Enum::TYPE>(%enum);
    }})]
pub struct ReadTagFor<Enum: TypeDeclaration>(PhantomData<Enum>);

#[derive(Instruction)]
#[instruction(WriteTagFor->fn(enum:Pointer<Enum>,tag:Usize){ entry:{
        WriteTag<Enum::TYPE>(%enum,%tag);
    }})]
pub struct WriteTagFor<Enum: TypeDeclaration>(PhantomData<Enum>);

#[derive(Instruction)]
#[instruction(ReadTagAndCheckFor->fn(enum:Pointer<Enum>)->(result:Bool){ entry:{
        %result = UsizeEq(ReadTag<Enum::TYPE>(%enum),b::IntTruncate<12,7>(TAG));
    }},
    TAG={kind=Int}
)]
pub struct ReadTagAndCheckFor<Enum: TypeDeclaration, const TAG: i64>(PhantomData<Enum>);

#[derive(Instruction)]
#[instruction(LocateVariantUncheckedFor->fn(enum:Pointer<Enum>)->(variant:Pointer<Variant>){ entry:{
        %variant = CastUnchecked<Pointer::<Enum>::TYPE,Pointer::<Variant>::TYPE>(%enum);
    }},
    TAG={kind=Int}
)]
pub struct LocateVariantUncheckedFor<Enum: TypeDeclaration, Variant: TypeDeclaration, const TAG: i64>(PhantomData<(Variant, Enum)>);

#[derive(Instruction)]
#[instruction(ReadVariantUncheckedFor->fn(enum:Pointer<Enum>)->(variant:Variant){ entry:{
        %variant = Read<Variant::TYPE>(CastUnchecked<Pointer::<Enum>::TYPE,Pointer::<Variant>::TYPE>(%enum));
    }},
    TAG={kind=Int}
)]
pub struct ReadVariantUncheckedFor<Enum: TypeDeclaration, Variant: TypeDeclaration, const TAG: i64>(PhantomData<(Variant, Enum)>);

#[derive(Instruction)]
#[instruction(WriteVariantFor->fn(enum:Pointer<Enum>,variant:Variant){ entry:{
        Write<Variant::TYPE>(CastUnchecked<Pointer::<Enum>::TYPE,Pointer::<Variant>::TYPE>(%enum),%variant);
        WriteTag<Enum::TYPE>(%enum,TAG);
    }},
    TAG={kind=Int}
)]
pub struct WriteVariantFor<Enum: TypeDeclaration, Variant: TypeDeclaration, const TAG: i64>(PhantomData<(Variant, Enum)>);

#[derive(Instruction)]
#[instruction(GetTagFor->fn(enum:Enum)->(tag:Usize){ entry:{
        %tag = GetTag<Enum::TYPE>(%enum);
    }})]
pub struct GetTagFor<Enum: TypeDeclaration>(PhantomData<Enum>);

#[derive(Instruction)]
#[instruction(GetTagAndCheckFor->fn(enum:Enum)->(result:Bool){ entry:{
        %result = UsizeEq(GetTag<Enum::TYPE>(%enum),b::IntTruncate<12,7>(TAG));
    }},
    TAG={kind=Int}
)]
pub struct GetTagAndCheckFor<Enum: TypeDeclaration, const TAG: i64>(PhantomData<Enum>);

#[derive(Instruction)]
#[instruction(DecodeVariantUnchecked->fn(enum:Enum)->(variant:Variant){ entry:{
        %variant = DecodeVariantUnchecked<Enum::TYPE,TAG>(%enum);
    }},
    TAG={kind=Int}
)]
pub struct DecodeVariantUncheckedFor<Enum: TypeDeclaration, Variant: TypeDeclaration, const TAG: i64>(PhantomData<(Variant, Enum)>);

#[derive(Instruction)]
#[instruction(EncodeVariantFor->fn(variant:Variant)->(enum:Enum){ entry:{
        %enum = EncodeVariant<Enum::TYPE,TAG>(%variant);
    }},
    TAG={kind=Int}
)]
pub struct EncodeVariantFor<Enum: TypeDeclaration, Variant: TypeDeclaration, const TAG: i64>(PhantomData<(Variant, Enum)>);

#[derive(Instruction)]
#[instruction(PointerOffset->fn(ptr:Pointer<F>)->(out:Pointer<T>){ entry:{
        %out = CastUnchecked<Pointer::<T>::TYPE,Usize::TYPE>(UsizeAdd(CastUnchecked<Usize::TYPE,Pointer::<F>::TYPE>(%ptr),%offset));
    }}
)]
pub struct PointerOffset<F: TypeDeclaration, T: TypeDeclaration>(PhantomData<F>, PhantomData<T>);

#[derive(Instruction)]
#[instruction(PointerEq->fn(ptr1:Pointer<T>,ptr2:Pointer<T>)->(o:Bool){ entry:{
        %o = UsizeEq(CastUnchecked<Usize::TYPE,Pointer::<T>::TYPE>(%ptr1),CastUnchecked<Usize::TYPE,Pointer::<T>::TYPE>(%ptr2));
    }}
)]
pub struct PointerEq<T: TypeDeclaration>(PhantomData<T>);

make_instruction! {
    Goto->fn<block then>(){ entry:{
        branch %then;
    }}
}

#[derive(Instruction)]
#[instruction(ReadElement->fn(ptr:Pointer<P>,index:Usize)->(out:E){ entry:{
        %out=Read<E::TYPE>(LocateElement<P::TYPE>(%ptr,%index));
    }}
)]
pub struct ReadElement<E: TypeDeclaration, P: TypeDeclaration>(PhantomData<(E, P)>);
#[derive(Instruction)]
#[instruction(WriteElement->fn(ptr:Pointer<P>,index:Usize,value:E){ entry:{
        Write<E::TYPE>(LocateElement<P::TYPE>(%ptr,%index),%value);
    }}
)]
pub struct WriteElement<E: TypeDeclaration, P: TypeDeclaration>(PhantomData<(E, P)>);

#[derive(Instruction)]
#[instruction(Move->fn(i:T)->(o:T){ entry:{
    %o = b::Move<T::TYPE>(%i);
    }}
)]
pub struct Move<T: TypeDeclaration>(PhantomData<T>);

#[derive(Instruction)]
#[instruction(SliceLen->fn(i:Slice<T>)->(o:Usize){ entry:{
    %o = GetField<Slice::<T>::TYPE,1>(%i);
    }}
)]
pub struct SliceLen<T: TypeDeclaration>(PhantomData<T>);

#[derive(Instruction)]
#[instruction(LocateSliceElement->fn(i:Slice<T>,index:Usize)->(o:Pointer<T>){ entry:{
    %o = LocateElement<Pointer::<T>::TYPE>(GetField<Slice::<T>::TYPE,0>(%i),%index);
    }}
)]
pub struct LocateSliceElement<T: TypeDeclaration>(PhantomData<T>);

#[derive(Instruction)]
#[instruction(SliceGet->fn(i:Slice<T>,index:Usize)->(o:T){ entry:{
    %element_ptr = LocateElement<Pointer::<T>::TYPE>(GetField<Slice::<T>::TYPE,0>(%i),%index);
    %o = Read<T::TYPE>(%element_ptr);
    }}
)]
pub struct SliceGet<T: TypeDeclaration>(PhantomData<T>);

#[derive(Instruction)]
#[instruction(SliceSet->fn(i:Slice<T>,index:Usize,value:T){ entry:{
    %element_ptr = LocateElement<Pointer::<T>::TYPE>(GetField<Slice::<T>::TYPE,0>(%i),%index);
    Write<T::TYPE>(%element_ptr,%value);
    }}
)]
pub struct SliceSet<T: TypeDeclaration>(PhantomData<T>);

#[derive(Instruction)]
#[instruction(SubSlice->fn(i:Slice<T>,start:Usize,len:Usize)->(o:Slice<T>){ entry:{
    %element_ptr = LocateElement<Pointer::<T>::TYPE>(GetField<Slice::<T>::TYPE,0>(%i),%start);
    %o=SetField<Slice::<T>::TYPE,1>(SetField<Slice::<T>::TYPE,0>(%i,%element_ptr),%len);
    }}
)]
pub struct SubSlice<T: TypeDeclaration>(PhantomData<T>);

#[derive(Instruction)]
#[instruction(SliceCopy->fn(dst:Slice<T>,src:Slice<T>){ entry:{
    %len = GetField<Slice::<T>::TYPE,1>(%src);
    %dst_ptr = GetField<Slice::<T>::TYPE,0>(%dst);
    %src_ptr = GetField<Slice::<T>::TYPE,0>(%src);
    MemoryCopy<T::TYPE>(%dst_ptr,%src_ptr,%len);
    }}
)]
pub struct SliceCopy<T: TypeDeclaration>(PhantomData<T>);

#[derive(Instruction)]
#[instruction(UnsizedArrayToSlice->fn(array:Pointer<UnsizedArray<T>>)->(slice:Slice<T>){ entry:{
    %new_slice=UninitedStruct<Slice::<T>::TYPE>();
    %new_slice=SetField<Slice::<T>::TYPE,1>(%new_slice,GetLength<UnsizedArray::<T>::TYPE>(%array));
    %slice=SetField<Slice::<T>::TYPE,0>(%new_slice,LocateElement<UnsizedArray::<T>::TYPE>(%array,b::IntTruncate<12,7>(0)));
    }}
)]
pub struct UnsizedArrayToSlice<T: TypeDeclaration>(PhantomData<T>);
