#[macro_use]
extern crate runtime_derive;
#[cfg(test)]
mod test {

    use llvm_runtime::JITCompiler;
    use memory_mmmu::MemoryMMMU;
    use runtime::{
        code::{BlockBuilder, BuddyRegisterPool, FunctionBuilder, FunctionPack},
        instructions::bootstrap as b,
    };
    use vm_core::{ExecutableResourceTrait, FunctionTypeBuilder, ResourceConverter, _ghost_cell::GhostToken};

    use runtime_extra as e;
    runtime_derive::make_instruction_set! {
        EvalInstructionSet = [
            I64Add->e::I64Add,
            I64Mul->e::I64Mul,
            ReturnI64->fn(v:e::I64){ entry:{
                    b::Return<e::I64::TYPE>(%v);
            } },
            ConstI64->e::I64Const,
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
        util::set_signal_handler();
        GhostToken::new(|mut token| {
            let mut function_builder = FunctionBuilder::<EvalInstructionSet>::new();
            let mut block_builder = BlockBuilder::<EvalInstructionSet>::default();
            type Register<T> = runtime::code::Register<T, BuddyRegisterPool>;

            let arg0 = Register::<e::I64>::new_const(0);
            let arg1 = Register::<e::I64>::new_const(1);
            let ret = Register::<e::I64>::new_const(0);
            let regester_count = 2;
            I64Add::emit(&mut block_builder, &mut token, &arg1, &arg0).unwrap();
            ReturnI64::emit(&mut block_builder, &mut token, &ret).unwrap();
            dbg!(&block_builder.codes());
            function_builder.add_block(block_builder);
            ret.forget();
            arg0.forget();
            arg1.forget();
            let function_type = FunctionTypeBuilder::default().args(vec![e::I64::TYPE, e::I64::TYPE].into()).return_type(Some(e::I64::TYPE)).build()?;
            let pack = function_builder.pack(&mut token, function_type, regester_count)?;
            dbg!(&pack);

            let runtime: JITCompiler<EvalInstructionSet, MemoryMMMU> = JITCompiler::new().map_err(|e| {
                println!("{}", e);
                e
            })?;
            let function_resource = runtime.create(pack)?;

            unsafe {
                let function_address: unsafe extern "C" fn(i64, i64) -> i64 = std::mem::transmute(
                    ExecutableResourceTrait::<FunctionPack<EvalInstructionSet>>::get_object(&*function_resource).unwrap().lock().unwrap().get_export_ptr(0),
                );
                let result = (function_address)(1, 1);
                assert_eq!(result, 2);
            }
            Ok(())
        })
    }
}
