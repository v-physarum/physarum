//! Compile-fail pairs for the derive.
//!
//! A missing provider is the `compile_fail` doctest on [`ph_beans::At`].
//! The matching program that compiles is the tree test: distinct beans, a bean
//! in only one list, and a parent that provides every inject.

#![forbid(unsafe_code)]
#![allow(missing_docs)]

use ph_proc_macro_dylib::proc_macro_library;
use ph_test_errors::ErrorTester;

#[test]
fn duplicate_provide_is_rejected() {
    fail(
        "
        #[derive(Module)]
        #[provide(Count::T, Count::T)]
        struct Parent;
        ",
        "duplicate bean in `provide`",
    );
}

#[test]
fn duplicate_inject_is_rejected() {
    fail(
        "
        #[derive(Module)]
        #[inject(Count::T, Count::MUT)]
        struct Child;
        ",
        "duplicate bean in `inject`",
    );
}

#[test]
fn a_bean_in_both_lists_is_rejected() {
    fail(
        "
        #[derive(Module)]
        #[provide(Count::T)]
        #[inject(Count::MUT)]
        struct Child;
        ",
        "bean is in both `provide` and `inject`",
    );
}

#[test]
fn five_provides_are_rejected() {
    fail(
        "
        #[derive(Module)]
        #[provide(A::T, B::T, C::T, D::T, E::T)]
        struct Parent;
        ",
        "accepts at most 4 beans",
    );
}

#[test]
fn a_generic_module_is_rejected() {
    fail(
        "
        #[derive(Module)]
        struct Holder<T>(T);
        ",
        "does not support generic",
    );
}

#[test]
fn five_injects_are_rejected() {
    fail(
        "
        #[derive(Module)]
        #[inject(A::T, B::T, C::T, D::T, E::T)]
        struct Child;
        ",
        "accepts at most 4 beans",
    );
}

#[test]
fn provides_plus_injects_past_four_are_rejected() {
    fail(
        "
        #[derive(Module)]
        #[provide(A::T, B::T, C::T)]
        #[inject(D::T, E::T)]
        struct Child;
        ",
        "accepts at most 4 beans",
    );
}

#[test]
fn a_bare_type_is_rejected() {
    fail(
        "
        #[derive(Module)]
        #[provide(Count)]
        struct Parent;
        ",
        "expected Type::T or Type::MUT",
    );
}

#[test]
fn the_old_mut_marker_is_rejected() {
    fail(
        "
        #[derive(Module)]
        #[provide(Keys::Mut)]
        struct Parent;
        ",
        "expected Type::T or Type::MUT",
    );
}

#[test]
fn provide_on_a_data_bean_is_rejected() {
    fail(
        "
        #[derive(Module)]
        #[provide(Count::T)]
        struct Holder(u8);
        ",
        "a module is a unit struct",
    );
}

fn fail(body: &str, description: &str) {
    let source = format!("use ph_beans_derive::Module;\n{body}\n");
    ErrorTester::source(source)
        .extern_crate("ph_beans_derive", proc_macro_library("ph_beans_derive"))
        .assert_description(description);
}
