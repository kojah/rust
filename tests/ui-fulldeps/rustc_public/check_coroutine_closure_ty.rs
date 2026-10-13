//@ run-pass
//! Test that the type of an async closure can be converted to rustc_public.

//@ ignore-stage1
//@ ignore-cross-compile
//@ ignore-remote
//@ edition: 2024

#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;
#[macro_use]
extern crate rustc_public;

use std::io::Write;
use std::ops::ControlFlow;

use rustc_public::CrateDef;
use rustc_public::ty::{RigidTy, TyKind};

const CRATE_NAME: &str = "input";

fn test_stable_mir() -> ControlFlow<()> {
    let item = rustc_public::all_local_items()
        .into_iter()
        .find(|item| item.name() == "input::make")
        .expect("Expected function `make`");
    let ty = item.expect_body().ret_local().ty;
    let TyKind::RigidTy(RigidTy::CoroutineClosure(def, args)) = ty.kind() else {
        panic!("Expected a coroutine closure type, found {:?}", ty.kind());
    };
    assert_eq!(def.name(), "input::make::{closure#0}");
    assert!(!args.0.is_empty(), "Expected the closure's generic arguments");

    // Converting the type back to the compiler must also work.
    assert!(ty.layout().is_ok(), "Expected the layout of the async closure");

    ControlFlow::Continue(())
}

/// This test will generate and analyze a dummy crate using the stable mir.
/// For that, it will first write the dummy crate into a file.
/// Then it will create a `RustcPublic` using custom arguments and then
/// it will run the compiler.
fn main() {
    let path = "coroutine_closure_input.rs";
    generate_input(&path).expect("Expected input file to be generated");
    let args = &[
        "rustc".to_string(),
        "--edition=2024".to_string(),
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
        pub fn make() -> impl AsyncFn() -> u8 {{
            async || 1
        }}
        "#
    )?;
    Ok(())
}
