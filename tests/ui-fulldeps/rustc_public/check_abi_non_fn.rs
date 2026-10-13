//@ run-pass
//! Test that function ABI queries return an error for instances and signatures that have no
//! function ABI, instead of panicking.

//@ ignore-stage1
//@ ignore-cross-compile
//@ ignore-remote

#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;
#[macro_use]
extern crate rustc_public;

use std::convert::TryFrom;
use std::io::Write;
use std::ops::ControlFlow;

use rustc_public::mir::mono::Instance;
use rustc_public::ty::{GenericArgs, RigidTy, TyKind};
use rustc_public::{CrateDef, CrateItem};

const CRATE_NAME: &str = "input";

fn test_stable_mir() -> ControlFlow<()> {
    let items = rustc_public::all_local_items();
    let get = |name: &str| -> CrateItem {
        *items.iter().find(|item| item.name() == name).unwrap_or_else(|| panic!("{}", name))
    };

    // Constants and statics are not functions.
    for name in ["input::CONST", "input::STATIC"] {
        let instance = Instance::try_from(get(name)).expect("Expected an instance");
        assert!(instance.fn_abi().is_err(), "Expected an error for {}", name);
    }
    let static_def = rustc_public::local_crate().statics()[0];
    assert!(Instance::from(static_def).fn_abi().is_err(), "Expected an error for a static");

    // LLVM intrinsics are not called through a function ABI.
    let caller = get("input::sqrt");
    let intrinsic = rustc_public::local_crate()
        .fn_defs()
        .into_iter()
        .find(|def| def.name() == "input::llvm_sqrt")
        .expect("Expected the LLVM intrinsic");
    let instance = Instance::resolve(intrinsic, &GenericArgs(vec![])).unwrap();
    assert!(instance.fn_abi().is_err(), "Expected an error for an LLVM intrinsic instance");
    assert!(
        intrinsic.fn_sig().fn_ptr_abi().is_err(),
        "Expected an error for an LLVM intrinsic signature"
    );

    // Regular functions still have an ABI.
    let instance = Instance::try_from(caller).unwrap();
    let abi = instance.fn_abi().expect("Expected the ABI of a regular function");
    assert_eq!(abi.args.len(), 1);
    let TyKind::RigidTy(RigidTy::FnDef(def, _)) = instance.ty().kind() else { unreachable!() };
    assert!(def.fn_sig().fn_ptr_abi().is_ok());

    ControlFlow::Continue(())
}

/// This test will generate and analyze a dummy crate using the stable mir.
/// For that, it will first write the dummy crate into a file.
/// Then it will create a `RustcPublic` using custom arguments and then
/// it will run the compiler.
fn main() {
    let path = "non_fn_abi_input.rs";
    generate_input(&path).expect("Expected input file to be generated");
    let args = &[
        "rustc".to_string(),
        "--crate-type=lib".to_string(),
        "--crate-name".to_string(),
        CRATE_NAME.to_string(),
        path.to_string(),
    ];
    run!(args, test_stable_mir).expect("Expected the checks to succeed");
}

fn generate_input(path: &str) -> std::io::Result<()> {
    let mut file = std::fs::File::create(path)?;
    write!(
        file,
        r#"
        #![allow(internal_features)]
        #![feature(link_llvm_intrinsics)]

        pub const CONST: u64 = 7;
        pub static STATIC: u32 = 1;

        unsafe extern "llvm-intrinsic" {{
            #[link_name = "llvm.sqrt.f64"]
            fn llvm_sqrt(x: f64) -> f64;
        }}

        pub fn sqrt(x: f64) -> f64 {{
            unsafe {{ llvm_sqrt(x) }}
        }}
        "#
    )?;
    Ok(())
}
