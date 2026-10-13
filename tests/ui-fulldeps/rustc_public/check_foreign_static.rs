//@ run-pass
//! Test that evaluating the initializer of a foreign static returns an error.

//@ ignore-stage1
//@ ignore-cross-compile
//@ ignore-remote

#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;
#[macro_use]
extern crate rustc_public;

use std::io::Write;
use std::ops::ControlFlow;

use rustc_public::CrateDef;

const CRATE_NAME: &str = "input";

fn test_stable_mir() -> ControlFlow<()> {
    let statics = rustc_public::local_crate().statics();
    let get = |name: &str| *statics.iter().find(|def| def.name() == name).unwrap();

    let foreign = get("input::FOREIGN");
    assert!(foreign.eval_initializer().is_err(), "Expected an error for a foreign static");

    let local = get("input::LOCAL");
    let alloc = local.eval_initializer().expect("Expected the initializer of a local static");
    assert_eq!(alloc.read_uint(), Ok(7), "Expected the value of LOCAL");

    ControlFlow::Continue(())
}

/// This test will generate and analyze a dummy crate using the stable mir.
/// For that, it will first write the dummy crate into a file.
/// Then it will create a `RustcPublic` using custom arguments and then
/// it will run the compiler.
fn main() {
    let path = "foreign_static_input.rs";
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
        unsafe extern "C" {{
            pub static FOREIGN: i32;
        }}

        pub static LOCAL: u32 = 7;
        "#
    )?;
    Ok(())
}
