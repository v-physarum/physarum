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

#[test]
fn missing_name_matches_code_and_description() {
    ErrorTester::source(MISSING).assert_code_description("E0425", "cannot find value");
}

#[test]
fn file_matches_the_type_error() {
    let path = common::write_probe("fail.rs", TYPE_ERROR);
    ErrorTester::file(&path).assert_code_description("E0308", "mismatched types");
}

#[test]
fn two_errors_match_either_pair() {
    ErrorTester::source(TWO)
        .assert_code_description("E0308", "mismatched types")
        .assert_code_description("E0425", "cannot find value");
}

#[test]
#[should_panic(expected = "E0425: cannot find value")]
fn wrong_code_reports_the_code_that_arrived() {
    ErrorTester::source(MISSING).assert_code_description("E0308", "mismatched types");
}

#[test]
#[should_panic(expected = "assertion failed: error E0425 description not in the message")]
fn wrong_description_reports_the_message_that_arrived() {
    ErrorTester::source(MISSING).assert_code_description("E0425", "not in the message");
}
