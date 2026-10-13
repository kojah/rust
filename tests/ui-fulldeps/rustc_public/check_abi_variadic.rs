//@ run-pass
//! Test the ABI of C-variadic functions whose calling convention is not `C`.

//@ ignore-stage1
//@ ignore-cross-compile
//@ ignore-remote
//@ only-x86_64

#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;
#[macro_use]
extern crate rustc_public;

use std::convert::TryFrom;
use std::io::Write;
use std::ops::ControlFlow;

use rustc_public::abi::{CallConvention, FnAbi};
use rustc_public::mir::mono::Instance;
use rustc_public::{CrateDef, CrateItem, CrateItems, ItemKind};

const CRATE_NAME: &str = "input";

fn test_stable_mir() -> ControlFlow<()> {
    let items = rustc_public::all_local_items();

    let sysv64 = fn_abi(&items, "input::sysv64_variadic");
    check_variadic(&sysv64, CallConvention::X86_64SysV);
    let win64 = fn_abi(&items, "input::win64_variadic");
    check_variadic(&win64, CallConvention::X86_64Win64);

    // The same conventions through function pointers.
    let holder = get_item(&items, "input::fn_ptr_holder");
    let holder = Instance::try_from(holder).expect("Expected fn_ptr_holder instance");
    let body = holder.body().expect("Expected fn_ptr_holder body");
    let args = body.arg_locals();
    let ptr_abi = |index: usize| {
        args[index]
            .ty
            .kind()
            .fn_sig()
            .expect("Expected function pointer signature")
            .fn_ptr_abi()
            .expect("Expected function pointer ABI")
    };
    assert_eq!(ptr_abi(0), sysv64, "Expected matching sysv64 function and pointer ABIs");
    assert_eq!(ptr_abi(1), win64, "Expected matching win64 function and pointer ABIs");

    ControlFlow::Continue(())
}

fn fn_abi(items: &CrateItems, name: &str) -> FnAbi {
    let instance = Instance::try_from(get_item(items, name)).expect("Expected function instance");
    instance.fn_abi().expect("Expected function ABI")
}

fn check_variadic(abi: &FnAbi, conv: CallConvention) {
    assert_eq!(abi.conv, conv, "Expected {conv:?} calling convention");
    assert!(abi.c_variadic, "Expected C variadic function");
    assert_eq!(abi.fixed_count, 1, "Expected one fixed argument");
    assert_eq!(abi.args.len(), 1, "Expected one argument");
}

fn get_item(items: &CrateItems, name: &str) -> CrateItem {
    *items
        .iter()
        .find(|item| item.kind() == ItemKind::Fn && item.name() == name)
        .unwrap_or_else(|| panic!("Expected function `{}`", name))
}

/// This test will generate and analyze a dummy crate using the stable mir.
/// For that, it will first write the dummy crate into a file.
/// Then it will create a `RustcPublic` using custom arguments and then
/// it will run the compiler.
fn main() {
    let path = "variadic_abi_input.rs";
    generate_input(&path).expect("Expected input file to be generated");
    let args = &[
        "rustc".to_string(),
        "-Cpanic=abort".to_string(),
        "--crate-type=lib".to_string(),
        "--crate-name".to_string(),
        CRATE_NAME.to_string(),
        path.to_string(),
    ];
    run!(args, test_stable_mir).expect("Expected ABI checks to succeed");
}

fn generate_input(path: &str) -> std::io::Result<()> {
    let mut file = std::fs::File::create(path)?;
    write!(
        file,
        r#"
        #[unsafe(naked)]
        pub unsafe extern "sysv64" fn sysv64_variadic(n: u32, _: ...) -> u32 {{
            core::arch::naked_asm!("ret")
        }}

        #[unsafe(naked)]
        pub unsafe extern "win64" fn win64_variadic(n: u32, _: ...) -> u32 {{
            core::arch::naked_asm!("ret")
        }}

        pub fn fn_ptr_holder(
            sysv64: unsafe extern "sysv64" fn(u32, ...) -> u32,
            win64: unsafe extern "win64" fn(u32, ...) -> u32,
        ) {{
            // We only care about the signature.
            todo!()
        }}
        "#
    )?;
    Ok(())
}
