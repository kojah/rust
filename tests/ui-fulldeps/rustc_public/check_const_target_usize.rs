//@ run-pass
//! Test that `MirConst::eval_target_usize` only accepts pointer-sized integers.

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

use rustc_public::mir::MirVisitor;
use rustc_public::mir::visit::Location;
use rustc_public::ty::{MirConst, RigidTy, TyKind, UintTy};
use rustc_public::{CrateDef, ItemKind};

const CRATE_NAME: &str = "input";

#[derive(Default)]
struct ConstVisitor {
    consts: Vec<MirConst>,
}

impl MirVisitor for ConstVisitor {
    fn visit_mir_const(&mut self, constant: &MirConst, _location: Location) {
        self.consts.push(constant.clone());
    }
}

fn test_stable_mir() -> ControlFlow<()> {
    for (name, expected) in [("input::small", None), ("input::pointer_sized", Some(7))] {
        let item = rustc_public::all_local_items()
            .into_iter()
            .find(|item| item.kind() == ItemKind::Fn && item.name() == name)
            .unwrap();
        let mut visitor = ConstVisitor::default();
        visitor.visit_body(&item.expect_body());
        let constant = visitor
            .consts
            .iter()
            .find(|c| matches!(c.ty().kind(), TyKind::RigidTy(RigidTy::Uint(_))))
            .unwrap();
        let result = constant.eval_target_usize();
        match expected {
            None => {
                assert_eq!(
                    constant.ty().kind(),
                    TyKind::RigidTy(RigidTy::Uint(UintTy::U8)),
                    "Expected a u8 constant"
                );
                assert!(result.is_err(), "Expected an error for a u8 constant: {:?}", result);
            }
            Some(value) => assert_eq!(result, Ok(value), "Expected a usize constant"),
        }
    }
    ControlFlow::Continue(())
}

/// This test will generate and analyze a dummy crate using the stable mir.
/// For that, it will first write the dummy crate into a file.
/// Then it will create a `RustcPublic` using custom arguments and then
/// it will run the compiler.
fn main() {
    let path = "target_usize_input.rs";
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
        pub fn small() -> u8 {{
            1
        }}

        pub fn pointer_sized() -> usize {{
            7
        }}
        "#
    )?;
    Ok(())
}
