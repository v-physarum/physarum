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

const COMPILE_ERROR: &str = "\
compile_error!(\"open outer range\");
compile_error!(\"invalid range\");
";

const CODED_AND_PLAIN: &str = "\
fn demo() {
    compile_error!(\"open outer range\");
    let _missing = missing;
}
";

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

#[test]
fn compile_error_matches_a_description_without_a_code() {
    ErrorTester::source(COMPILE_ERROR)
        .assert_description("open outer range")
        .assert_description("invalid range");
}

#[test]
fn coded_and_uncoded_errors_match_separately() {
    ErrorTester::source(CODED_AND_PLAIN)
        .assert_description("open outer range")
        .assert_code_description("E0425", "cannot find value");
}

#[test]
#[should_panic(expected = "(none): open outer range")]
fn compile_error_does_not_match_a_code() {
    ErrorTester::source("compile_error!(\"open outer range\");\n")
        .assert_code_description("E0308", "open outer range");
}

#[test]
#[should_panic(expected = "E0425: cannot find value")]
fn coded_error_does_not_match_a_description_without_a_code() {
    ErrorTester::source(MISSING).assert_description("cannot find value");
}

#[test]
#[should_panic(expected = "assertion failed: error (none) description not in the message")]
fn wrong_uncoded_description_reports_the_message_that_arrived() {
    ErrorTester::source("compile_error!(\"open outer range\");\n")
        .assert_description("not in the message");
}

#[test]
fn extern_crates_are_passed_as_name_and_path() {
    let demo = common::write_rlib("demo", "pub fn present() {}\n");
    let other = common::write_rlib("other", "pub fn present() {}\n");
    let path = common::write_probe(
        "probe.rs",
        "pub fn probe() {\n    demo::missing();\n    other::missing();\n}\n",
    );
    ErrorTester::file(&path)
        .extern_crate("demo", &demo)
        .extern_crate("other", &other)
        .assert_code_description("E0425", "cannot find function `missing` in crate `demo`")
        .assert_code_description("E0425", "cannot find function `missing` in crate `other`");
}
