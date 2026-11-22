#![allow(missing_docs, clippy::must_use_candidate)]

use ph_proc_macro_dylib::proc_macro_library;
use ph_test_errors::ErrorTester;
use ph_util_foreach::impl_foreach;

fn assert_err(input: &str, description: &str) {
    let source = format!("use ph_util_foreach::impl_foreach;\n{input}\n");
    ErrorTester::source(source)
        .extern_crate("ph_util_foreach", proc_macro_library("ph_util_foreach"))
        .assert_description(description);
}

#[test]
fn spaced_paste_and_negative_index_are_errors() {
    assert_err(
        "impl_foreach!(n in -1..0 { struct paste(Ix, n); });",
        "cannot paste a negative index",
    );
    assert_err(
        "impl_foreach!(n in -1..0 { Name[0..=n]; });",
        "fan mode domain `0..=index` is empty",
    );
    assert_err(
        "impl_foreach!(n in 0..1 { struct paste(Ix); });",
        "paste(Ident, n)",
    );
    assert_err(
        "impl_foreach!(n in 0..1 { struct paste(Ix, n + 1); });",
        "paste(Ident, n)",
    );
    assert_err(
        "impl_foreach!(n in 0..2 { (paste(T))[0..1]; });",
        "paste(Ident, n)",
    );
    assert_err(
        "impl_foreach!(n in 0..1 { fn paste(at)() {} });",
        "paste(Ident, n)",
    );
    assert_err(
        "impl_foreach!(n in 0..2 { X Name[0..1]; });",
        "repeat(Name, range)",
    );
    assert_err(
        "impl_foreach!(n in 0..2 { X + Name[0..1]; });",
        "repeat(Name, range)",
    );
}

#[test]
fn outer_range_errors() {
    for input in [
        "impl_foreach!(n in 0.. { K; });",
        "impl_foreach!(n in ..4 { K; });",
        "impl_foreach!(n in ..=4 { K; });",
        "impl_foreach!(n in .. { K; });",
        "impl_foreach!(n in (0..).step_by(2) { K; });",
        "impl_foreach!(n in (..4).step_by(2) { K; });",
        "impl_foreach!(n in (..=4).step_by(2) { K; });",
        "impl_foreach!(n in (..).step_by(2) { K; });",
    ] {
        assert_err(input, "open outer range");
    }
    assert_err("impl_foreach!(n in 5..3 { K; });", "invalid range");
    assert_err("impl_foreach!(n in 5..=3 { K; });", "invalid range");
    assert_err("impl_foreach!(n in 5..4 { K; });", "invalid range");
    assert_err(
        "impl_foreach!(n in (0..4).step_by(0) { K; });",
        "step must be a positive integer",
    );
    assert_err(
        "impl_foreach!(n in (0..4).step_by(-1) { K; });",
        "step must be a positive integer",
    );
    assert_err(
        "impl_foreach!(n in 0..n { K; });",
        "outer range cannot use the index",
    );
    assert_err(
        "impl_foreach!(n in (0..4).step_by(n) { K; });",
        "outer range cannot use the index",
    );
    assert_err("impl_foreach!(n in { K; });", "expected a range");
    assert_err(
        "impl_foreach!(n in 0..4);",
        "expected a `{ ... }` body after the range",
    );
    assert_err(
        "impl_foreach!(n in 0..2 { K; } extra);",
        "unexpected token after the body",
    );
    assert_err(
        "impl_foreach!(not_a_range = 0..2 { K; });",
        "write `impl_foreach!(n in 0..10 { ... })`",
    );
    assert_err(
        "impl_foreach!(n in 0X10..0X12 { K; });",
        "unsuffixed integer literal",
    );
    assert_err(
        "impl_foreach!(end in 0..1 { K; });",
        "cannot be named `end`",
    );
}

#[test]
fn pattern_errors() {
    assert_err(
        "impl_foreach!(n in 0..4 { Name[0..10]; });",
        "outside the domain",
    );
    assert_err(
        "impl_foreach!(n in (0..10).step_by(2) { Name[0..12]; });",
        "outside the domain",
    );
    assert_err(
        "impl_foreach!(n in 0..=10 { Name[4..=3]; });",
        "invalid range",
    );
    assert_err(
        "impl_foreach!(n in 0..4 { A[0..n, Off]; });",
        "exactly one range pattern",
    );
    assert_err(
        "impl_foreach!(n in 0..4 { Name[0..1..2]; });",
        "exactly one range pattern",
    );
    assert_err(
        "impl_foreach!(n in 0..4 { Name[(0..2).step_by()]; });",
        "expected a step",
    );
    assert_err(
        "impl_foreach!(n in 0..4 { Name[(0..4).step_by(0)]; });",
        "step must be a positive integer",
    );
    assert_err(
        "impl_foreach!(n in 1..2 { Name[(0..=n).step_by(n - 1)]; });",
        "step must be a positive integer",
    );
    assert_err(
        "impl_foreach!(n in 0..2 { Name[n - 1..=n]; });",
        "outside the domain",
    );
    assert_err(
        "impl_foreach!(n in 0..4 { Name[n + 1.5..=n]; });",
        "unsuffixed integer literal",
    );
    assert_err(
        "impl_foreach!(n in 0..=4 { Name[-1..=-1]; });",
        "outside the domain",
    );
}

#[test]
fn index_binding_rejects_a_bad_right_hand_side() {
    assert_err(
        "impl_foreach!(n in 0..1 { $id = Ix; });",
        "$name = paste(Ident, n);",
    );
    assert_err(
        "impl_foreach!(n in 0..1 { $id = paste(Ix); });",
        "paste(Ident, n)",
    );
    assert_err(
        "impl_foreach!(n in 0..1 { $id = paste(Ix, n) });",
        "$name = paste(Ident, n);",
    );
    assert_err(
        "impl_foreach!(n in 0..1 { $id = n + 1; });",
        "$name = paste(Ident, n);",
    );
    assert_err(
        "impl_foreach!(n in -1..0 { $id = paste(Ix, n); });",
        "cannot paste a negative index",
    );
}

#[test]
fn unnamed_trailing_block_is_valid() {
    impl_foreach!(n in 0..2 {
        let _k = n;
    });
    impl_foreach!(n in 0..2, {
        let _comma = n;
    });
}
