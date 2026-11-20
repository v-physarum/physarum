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

const FAIL_FILE_NAME: &str = "fail.rs";
const PROBE_FILE_NAME: &str = "probe.rs";

#[test]
fn missing_name_matches_its_code() {
    ErrorTester::source(MISSING).assert_code("E0425");
}

#[test]
fn missing_name_matches_its_message() {
    ErrorTester::source(MISSING).assert_description("cannot find value");
}

#[test]
fn file_matches_the_type_error() {
    let path = common::write_probe(FAIL_FILE_NAME, TYPE_ERROR);
    ErrorTester::file(&path).assert_code("E0308");
    ErrorTester::file(&path).assert_description("mismatched types");
}

#[test]
fn two_errors_match_either_code_or_message() {
    ErrorTester::source(TWO)
        .assert_code("E0308")
        .assert_code("E0425");
    ErrorTester::source(TWO)
        .assert_description("mismatched types")
        .assert_description("cannot find value");
}

#[test]
#[should_panic(expected = "E0425: cannot find value")]
fn wrong_code_reports_the_code_that_arrived() {
    ErrorTester::source(MISSING).assert_code("E0308");
}

#[test]
#[should_panic(expected = "assertion failed: description not in the message")]
fn wrong_description_reports_the_message_that_arrived() {
    ErrorTester::source(MISSING).assert_description("not in the message");
}

#[test]
fn compile_error_matches_a_message_when_the_code_is_null() {
    ErrorTester::source(COMPILE_ERROR)
        .assert_description("open outer range")
        .assert_description("invalid range");
}

#[test]
fn code_and_message_match_on_their_own() {
    ErrorTester::source(CODED_AND_PLAIN).assert_code("E0425");
    ErrorTester::source(CODED_AND_PLAIN)
        .assert_description("open outer range")
        .assert_description("cannot find value");
}

#[test]
#[should_panic(expected = "(none): open outer range")]
fn compile_error_does_not_match_a_code() {
    ErrorTester::source("compile_error!(\"open outer range\");\n").assert_code("E0308");
}

#[test]
#[should_panic(expected = "assertion failed: description not in the message")]
fn wrong_uncoded_description_reports_the_message_that_arrived() {
    ErrorTester::source("compile_error!(\"open outer range\");\n")
        .assert_description("not in the message");
}

#[test]
fn extern_crates_are_passed_as_name_and_path() {
    let demo = common::write_rlib("demo", "pub fn present() {}\n");
    let other = common::write_rlib("other", "pub fn present() {}\n");
    let path = common::write_probe(
        PROBE_FILE_NAME,
        "pub fn probe() {\n    demo::missing();\n    other::missing();\n}\n",
    );
    ErrorTester::file(&path)
        .extern_crate("demo", &demo)
        .extern_crate("other", &other)
        .assert_code("E0425")
        .assert_description("cannot find function `missing` in crate `demo`")
        .assert_description("cannot find function `missing` in crate `other`");
}
