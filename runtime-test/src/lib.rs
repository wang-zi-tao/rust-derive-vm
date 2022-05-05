use llvm_runimte::Interpreter;
use memory_mmmu::MemoryMMMU;
use runtime::{
    code::{BlockBuilder, BuddyRegisterPool, FunctionBuilder, FunctionPack},
    instructions::bootstrap as b,
};
use vm_core::{ExecutableResourceTrait, FunctionTypeBuilder, ResourceFactory, TypeDeclaration, _ghost_cell::GhostToken};

use runtime_extra as e;
#[macro_use]
extern crate runtime_derive;
runtime_derive::make_instruction_set! {
    EvalInstructionSet = [
        I64Add->e::I64Add,
        I64Mul->e::I64Mul,
        ReturnI64->fn(v:e::I64){ entry:{
                b::Return<e::I64::TYPE>(%v);
        } },
        // ConstI64->e::ConstI64,
        ConstI64_2->fn<const value:e::I64>()->(o:e::I64){ entry:{
                %o = b::Move<e::I64::TYPE>(%value);
        } },
        ReturnConstI64->fn<const value:e::I64>(){ entry:{
                b::Return<e::I64::TYPE>(%value);
        } },
    ]
}
#[test]
fn test() -> failure::Fallible<()> {
    GhostToken::new(|mut token| {
        let mut function_builder = FunctionBuilder::<EvalInstructionSet>::new();
        let mut block_builder = BlockBuilder::<EvalInstructionSet>::default();
        type Register<T> = runtime::code::Register<T, BuddyRegisterPool>;

        let arg0 = Register::<e::I64>::new_const(0);
        let arg1 = Register::<e::I64>::new_const(1);
        let ret = Register::<e::I64>::new_const(0);
        let regester_count = 2;
        // ConstI64::emit(&mut block_builder, &mut token, e::I64(4), &ret).unwrap();
        // dbg!(block_builder.codes().borrow_mut(&mut token));
        I64Add::emit(&mut block_builder, &mut token, &arg1, &arg0).unwrap();
        ReturnI64::emit(&mut block_builder, &mut token, &ret).unwrap();
        // dbg!(block_builder.codes().borrow_mut(&mut token));
        // ReturnConstI64::emit(&mut block_builder, &mut token, e::I64(4)).unwrap();
        function_builder.add_block(block_builder);
        ret.forget();
        arg0.forget();
        arg1.forget();
        let function_type = FunctionTypeBuilder::default().args(vec![e::I64::TYPE, e::I64::TYPE].into()).return_type(Some(e::I64::TYPE)).build()?;
        let pack = function_builder.pack(&mut token, function_type, regester_count)?;
        // dbg!(&pack);

        let interpreter: Interpreter<EvalInstructionSet, MemoryMMMU> = Interpreter::new().map_err(|e| {
            println!("{}", e);
            e
        })?;
        let function_resource = interpreter.create(pack)?;

        // unsafe {
        //     let function_address: *const unsafe extern "C" fn(i64, i64) -> i64 = function_resource.get_address();
        //     let result = (*function_address)(1, 1);
        //     assert_eq!(result, 2);
        // }
        unsafe {
            // let function_address0: *const unsafe extern "C" fn(i64, i64) -> i64 = function_resource.get_address();
            let function_address: unsafe extern "C" fn(i64, i64) -> i64 = std::mem::transmute(
                ExecutableResourceTrait::<FunctionPack<EvalInstructionSet>>::get_object(&*function_resource).unwrap().lock().unwrap().get_export_ptr(0),
            );
            // assert_eq!(*function_address0, function_address);
            let result = (function_address)(1, 1);
            assert_eq!(result, 2);
        }
        Ok(())
    })
}
