#![allow(missing_docs)]

mod common;

use ph_test_errors::ErrorTester;

const MISSING: &str = "fn main() { missing; }\n";

const TYPE_ERROR: &str = r#"pub fn demo() {
    let value: i32 = "no";
    let _ = value;
}
"#;

const TWO: &str = r#"pub fn demo() {
    let value: i32 = "no";
    let _missing = missing;
    let _ = value;
}
"#;

const TRAIT: &str = "\
pub trait Marker {}\n\
pub fn need<T: Marker>(value: T) -> T { value }\n\
pub fn demo() { let _ = need(1u32); }\n\
";

const ON_UNIMPLEMENTED: &str = r#"#[diagnostic::on_unimplemented(
    message = "my type `{Self}` is not a marker",
    label = "required for this bound",
    note = "implement the marker"
)]
pub trait Marker {}
pub fn need<T: Marker>(value: T) -> T { value }
pub fn demo() { let _ = need(1u32); }
"#;

const OK: &str = "pub fn add(left: i32, right: i32) -> i32 {\n    left.wrapping_add(right)\n}\n";

const WARN: &str = "fn unused() {}\n";

#[test]
fn missing_name_chain() {
    ErrorTester::source(MISSING)
        .assert_code("E0425")
        .assert_description("cannot find")
        .assert_code_description("E0425", "cannot find value")
        .assert_fail()
        .assert_no_warning();
}

#[test]
fn file_reports_the_type_error() {
    let path = common::write_probe("fail.rs", TYPE_ERROR);
    ErrorTester::file(&path)
        .assert_code("E0308")
        .assert_description("mismatched types")
        .assert_code_description("E0308", "mismatched types")
        .assert_fail();
}

#[test]
fn trait_bound_has_help_and_a_note() {
    ErrorTester::source(TRAIT)
        .assert_code("E0277")
        .assert_description("is not satisfied")
        .assert_help("no implementations")
        .assert_note("required by a bound");
}

#[test]
fn two_errors_are_both_reported() {
    ErrorTester::source(TWO)
        .assert_code("E0308")
        .assert_code("E0425")
        .assert_fail();
}

#[test]
fn clean_probe_passes() {
    ErrorTester::source(OK).assert_pass().assert_no_warning();
}

#[test]
fn dead_code_is_a_warning() {
    ErrorTester::source(WARN)
        .assert_pass()
        .assert_warning_code("dead_code")
        .assert_note("dead_code");
}

#[test]
fn on_unimplemented_note_is_visible() {
    ErrorTester::source(ON_UNIMPLEMENTED)
        .assert_code("E0277")
        .assert_note("implement the marker");
}

#[test]
#[should_panic(expected = "assertion failed: probe compiles")]
fn missing_name_does_not_pass() {
    ErrorTester::source(MISSING).assert_pass();
}

#[test]
#[should_panic(expected = "assertion failed: error E0308")]
fn missing_name_is_not_a_type_error() {
    ErrorTester::source(MISSING).assert_code("E0308");
}

#[test]
#[should_panic(expected = "assertion failed: probe fails")]
fn clean_probe_does_not_fail() {
    ErrorTester::source(OK).assert_fail();
}

#[test]
#[should_panic(expected = "assertion failed: no warnings")]
fn dead_code_is_not_clean() {
    ErrorTester::source(WARN).assert_no_warning();
}

#[test]
#[should_panic(expected = "assertion failed: help")]
fn missing_name_has_no_help() {
    ErrorTester::source(MISSING).assert_help("consider");
}
