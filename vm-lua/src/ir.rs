use super::instruction as i;

use runtime::instructions::bootstrap as b;
use runtime_extra::{self as e};

make_instruction_set! {
  LuaInstructionSet=[
    Nop->b::Nop,//0
    MoveI64->i::MoveI64,MoveF64->i::MoveF64,MoveValue->i::MoveValue,//3
    ConstM1->i::ConstM1,ConstZero->i::ConstZero,ConstOne->i::ConstOne,ConstI64->e::I64Const,ConstF64->e::F64Const,ConstValue->i::ConstValue,//8
    ConstNil->i::ConstNil,ConstTrue->i::ConstTrue,ConstFalse->i::ConstFalse,//11
    GetGlobal->i::GetGlobal,SetGlobal->i::SetGlobal,//13
    GetField->i::GetField, SetField->i::SetField,//15
    GetElement->i::GetElement, SetElement->i::SetElement,//17
    I64ToValue->i::I64ToValue, F64ToValue->i::F64ToValue,//19
    I64ToF64->e::I64ToF64, F64ToI64->i::F64ToI64,//21
    IAdd->e::I64Add, FAdd->e::F64Add, Add->i::Add,//24
    ISub->e::I64Sub, FSub->e::F64Sub, Sub->i::Sub,//27
    IMul->e::I64Mul, FMul->e::F64Mul, Mul->i::Mul,//30
    FPow->i::FPow, Pow->i::Pow,//32
    IDiv->i::IDiv, FDiv->e::F64Div, Div->i::Div,//35
    IDivFloor->e::I64Div, FDivFloor->i::FDivFloor, DivFloor->i::DivFloor,//38
    IRem->e::I64Rem, FRem->e::F64Rem, Rem->i::Rem,//41
    IBitXor->e::I64Xor, BitXor->i::BitXor,//43
    IBitAnd->e::I64And, BitAnd->i::BitAnd,//45
    IBitOr->e::I64Or, BitOr->i::BitOr,//47
    IBitNot->e::I64Not, BitNot->i::BitNot,//49
    INeg->e::I64Neg, FNeg->e::F64Neg, Neg->i::Neg,//53
    LogicalNot->i::LogicalNot,//54
    ILeftShift->e::I64LeftShift, LeftShift->i::LeftShift,//56
    IRightShift->e::I64RightShift, RightShift->i::RightShift,//58
    ILess->i::ILess, FLess->i::FLess, Less->i::Less,//61
    ILessOrEqual->i::ILessOrEqual, FLessOrEqual->i::FLessOrEqual, LessOrEqual->i::LessOrEqual,//64
    IEqual->i::IEqual, FEqual->i::FEqual, Equal->i::Equal,//67
    ILarge->i::ILarge, FLarge->i::FLarge, Large->i::Large,//70
    ILargeOrEqual->i::ILargeOrEqual, FLargeOrEqual->i::FLargeOrEqual, LargeOrEqual->i::LargeOrEqual,//73
    INotEqual->i::INotEqual, FNotEqual->i::FNotEqual, NotEqual->i::NotEqual,//76
    Concat->i::Concat, Length->i::Length,//78
    Goto->e::Goto,//79
    BuildTable->i::BuildTable,//80
    NewClosure->i::NewClosure,//81
    ForLoopJump->i::ForLoopJump,ForLoopInit->i::ForLoopInit,ForLoopIncrease->i::ForLoopIncrease,//84
    ForLoopStepJump->i::ForLoopStepJump,ForLoopStepInit->i::ForLoopStepInit,ForLoopStepIncrease->i::ForLoopStepIncrease,//87
    IfBranch->i::BranchIf,//88
    GetUpVariable->i::GetUpVariable, SetUpVariable->i::SetUpVariable, SetUpValue->i::SetUpValue,//91
    MakeSlice->b::MakeSlice,
    CallFunction->i::CallFunction,CallFunctionVaSlice->i::CallFunctionVaSlice,CallFunctionVA->i::CallFunctionVA,
    CallFunctionRet1->i::CallFunctionRet1,CallFunctionVaSliceRet1->i::CallFunctionVaSliceRet1,CallFunctionVaRet1->i::CallFunctionVaRet1,
    CallFunction0->i::CallFunction0,CallFunction0VaSlice->i::CallFunction0VaSlice,CallFunction0VA->i::CallFunction0VA,
    CallFunction0Ret1->i::CallFunction0Ret1,CallFunction0VaSliceRet1->i::CallFunction0VaSliceRet1,CallFunction0VaRet1->i::CallFunction0VaRet1,
    CallFunction1->i::CallFunction1,CallFunction1VaSlice->i::CallFunction1VaSlice,CallFunction1VA->i::CallFunction1VA,
    CallFunction1Ret1->i::CallFunction1Ret1,CallFunction1VaSliceRet1->i::CallFunction1VaSliceRet1,CallFunction1VaRet1->i::CallFunction1VaRet1,
    CallFunction2->i::CallFunction2,CallFunction2VaSlice->i::CallFunction2VaSlice,CallFunction2VA->i::CallFunction2VA,
    CallFunction2Ret1->i::CallFunction2Ret1,CallFunction2VaSliceRet1->i::CallFunction2VaSliceRet1,CallFunction2VaRet1->i::CallFunction2VaRet1,
    CallFunction3->i::CallFunction3,CallFunction3VaSlice->i::CallFunction3VaSlice,CallFunction3VA->i::CallFunction3VA,
    CallFunction3Ret1->i::CallFunction3Ret1,CallFunction3VaSliceRet1->i::CallFunction3VaSliceRet1,CallFunction3VaRet1->i::CallFunction3VaRet1,
    GetRet0->i::GetRet0,GetRet->i::GetRet,GetVaArg->i::GetVaArg,
    ReturnVaSlice->i::ReturnVaSlice,Return0VaSlice->i::Return0VaSlice,Return1VaSlice->i::Return1VaSlice,Return2VaSlice->i::Return2VaSlice,Return3VaSlice->i::Return3VaSlice,
    ReturnVA->i::ReturnVA,Return0VA->i::Return0VA,Return1VA->i::Return1VA,Return2VA->i::Return2VA,Return3VA->i::Return3VA,
    Return->i::Return,Return0->i::Return0,Return1->i::Return1,Return2->i::Return2,Return3->i::Return3,
    BreakPoint->i::BreakPoint,
    MakeTable->i::MakeTable,MakeTable0->i::MakeTable0,
    ForInLoopJump->i::ForInLoopJump,
    ConstClosure0->i::ConstClosure0,ConstClosure->i::ConstClosure,SetUpRef->i::SetUpRef,NewUpValue->i::NewUpValue
  ]
}
