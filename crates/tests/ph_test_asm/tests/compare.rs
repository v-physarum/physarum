#![allow(missing_docs, dead_code)]

mod common;

use common::{ADD_THEN_SUB, SUB_THEN_ADD, step_loop, write_probe};
use ph_test_asm::AsmCompare;

#[test]
fn identical_sources_match_at_every_scope() {
    let left = write_probe("left.rs", ADD_THEN_SUB);
    let right = write_probe("right.rs", ADD_THEN_SUB);
    AsmCompare::files(&left, &right).assert_equivalent();
    AsmCompare::files(&left, &right)
        .function("add")
        .assert_equivalent();
}

#[test]
fn swapped_functions_match_after_sort() {
    let left = write_probe("left.rs", ADD_THEN_SUB);
    let right = write_probe("right.rs", SUB_THEN_ADD);
    AsmCompare::files(&left, &right).assert_equivalent();
}

#[test]
fn one_function_can_match_while_the_module_does_not() {
    let left = write_probe("left.rs", &step_loop(1));
    let right = write_probe(
        "right.rs",
        &step_loop(1).replace(
            "pub fn other() -> i32 {\n    1\n}",
            "pub fn other() -> i32 {\n    9\n}",
        ),
    );
    AsmCompare::files(&left, &right)
        .function("k")
        .assert_equivalent();
    AsmCompare::files(&left, &right)
        .function("k")
        .hot_loop()
        .assert_equivalent();
}

#[test]
#[should_panic(expected = "probes are not equivalent\n---")]
fn different_modules_show_a_diff() {
    let left = write_probe("left.rs", ADD_THEN_SUB);
    let right = write_probe("right.rs", &step_loop(1));
    AsmCompare::files(&left, &right).assert_equivalent();
}

#[test]
#[should_panic(expected = "probes are not equivalent\n---")]
fn different_hot_loops_show_a_diff() {
    let left = write_probe("left.rs", &step_loop(1));
    let right = write_probe("right.rs", &step_loop(2));
    AsmCompare::files(&left, &right)
        .function("k")
        .hot_loop()
        .assert_equivalent();
}
