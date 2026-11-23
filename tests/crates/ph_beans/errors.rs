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
fn duplicate_provider_is_rejected() {
    fail(
        "
        #[derive(Component)]
        #[provider(Count, Count)]
        struct Parent;
        ",
        "duplicate bean in `provider`",
    );
}

#[test]
fn duplicate_inject_is_rejected() {
    fail(
        "
        #[derive(Component)]
        #[inject(Count, Count)]
        struct Child;
        ",
        "duplicate bean in `inject`",
    );
}

#[test]
fn a_bean_in_both_lists_is_rejected() {
    fail(
        "
        #[derive(Component)]
        #[provider(Count)]
        #[inject(Count)]
        struct Child;
        ",
        "bean is in both `provider` and `inject`",
    );
}

#[test]
fn five_providers_are_rejected() {
    fail(
        "
        #[derive(Component)]
        #[provider(A, B, C, D, E)]
        struct Parent;
        ",
        "accepts at most 4 beans",
    );
}

#[test]
fn a_generic_component_is_rejected() {
    fail(
        "
        #[derive(Component)]
        struct Holder<T>(T);
        ",
        "does not support generic",
    );
}

#[test]
fn four_injects_are_rejected() {
    fail(
        "
        #[derive(Component)]
        #[inject(A, B, C, D)]
        struct Child;
        ",
        "accepts at most 3 beans",
    );
}

fn fail(body: &str, description: &str) {
    let source = format!("use ph_beans_derive::Component;\n{body}\n");
    ErrorTester::source(source)
        .extern_crate("ph_beans_derive", proc_macro_library("ph_beans_derive"))
        .assert_description(description);
}
