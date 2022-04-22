#![feature(in_band_lifetimes)]
#![feature(box_syntax)]
#![feature(try_trait)]
#![feature(ptr_internals)]
#![feature(negative_impls)]
#![feature(map_first_last)]
#![feature(slice_ptr_len)]
#![feature(slice_ptr_get)]
#![feature(exclusive_range_pattern)]
#![feature(asm)]
#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![feature(const_generics)]
#![feature(default_free_fn)]
#![feature(core_intrinsics)]
#[macro_use]
extern crate failure_derive;
extern crate memory;
extern crate runtime;
// use classes::ClassesModule;

// use getset::{CopyGetters, Getters, MutGetters, Setters};
use failure::format_err;
use jvm_core::ClassLoaderRef;
use std::env::args;

use util::Result;

fn main() -> Result<()> {
    let mut args = args();
    while let Some(arg) = args.next() {
        match &*arg {
            "-version" => {
                println!("{}", "My JVM 0.0.1");
                break;
            }
            _ => {
                println!("unknown argument {}, ignored", arg);
            }
        }
    }
    {
        // let launch_class = "Main"; // TODO
        // let bootstrap_class_loader = ClassLoaderRef::create_bootstrap_class_loader()?;
        // let bootstrap_class_set = bootstrap_class_loader.get_boostrap_class_set();
        // let platform_class_loader =
        // ClassLoaderRef::create_platform_class_loader(&bootstrap_class_loader)?;
        // let app_class_loader = ClassLoaderRef::create_app_class_loader(&bootstrap_class_loader)?;
        //
        // let boot_class = app_class_loader.get_inited_class()?;
        // let main_method = boot_class
        // .get_method(
        // MethodNameAndParameterType::new(PooledStr::from("main")),
        // vec![Parameter::with_type(
        // bootstrap_class_set.java_lang_String.to_array().into(),
        // )],
        // )
        // .ok_or_else(|| format_err!("main method not found"))?;

        let arg0: Vec<String> = args.collect();
    };
    Ok(())
    // class_file::constants::test::main()
}
// pub trait Module {
//     // fn new_uninitalized() -> MaybeUninit<Self>;
//     // fn initialize(this: MaybeUninit<Self>);
// }
// pub struct JavaVirtualMachine {
//     memory: Arc<dyn MemoryModule>,
//     runtime: Arc<dyn RuntimeModule>,
//     calsses: Arc<dyn ClassesModule>,
// }
// impl JavaVirtualMachine {}
// // use getset::{CopyGetters, Getters, MutGetters, Setters};
