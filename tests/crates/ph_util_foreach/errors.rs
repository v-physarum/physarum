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
        "impl_foreach!(range = -1..0, body = { struct Ix + #; });",
        "cannot paste a negative index",
    );
    assert_err(
        "impl_foreach!(range = -1..0, body = { Name[#]; });",
        "fan mode domain `0..=index` is empty",
    );
    assert_err(
        "impl_foreach!(range = 0..1, body = { struct Ix #; });",
        "Name + #",
    );
    assert_err(
        "impl_foreach!(range = 0..1, body = { struct Ix#; });",
        "Name + #",
    );
    assert_err(
        "impl_foreach!(range = 0..2, body = { (T #)[0..1]; });",
        "Name + #",
    );
    assert_err(
        "impl_foreach!(range = 0..1, body = { fn at #() {} });",
        "Name + #",
    );
    assert_err(
        "impl_foreach!(range = 0..2, body = { X Name[0..1]; });",
        "not concatenation",
    );
}

#[test]
fn outer_range_errors() {
    for input in [
        "impl_foreach!(range = 0.., body = { K; });",
        "impl_foreach!(range = ..4, body = { K; });",
        "impl_foreach!(range = ..=4, body = { K; });",
        "impl_foreach!(range = .., body = { K; });",
        "impl_foreach!(range = 0..::2, body = { K; });",
        "impl_foreach!(range = ..4::2, body = { K; });",
        "impl_foreach!(range = ..=4::2, body = { K; });",
        "impl_foreach!(range = ..::2, body = { K; });",
    ] {
        assert_err(input, "open outer range");
    }
    assert_err(
        "impl_foreach!(range = 5..3, body = { K; });",
        "invalid range",
    );
    assert_err(
        "impl_foreach!(range = 5..=3, body = { K; });",
        "invalid range",
    );
    assert_err(
        "impl_foreach!(range = 5..4, body = { K; });",
        "invalid range",
    );
    assert_err(
        "impl_foreach!(range = 0..4::0, body = { K; });",
        "step must be a positive integer",
    );
    assert_err(
        "impl_foreach!(range = 0..4::-1, body = { K; });",
        "step must be a positive integer",
    );
    assert_err(
        "impl_foreach!(range = 0..#, body = { K; });",
        "outer range cannot use `#`",
    );
    assert_err(
        "impl_foreach!(range = 0..4::#, body = { K; });",
        "outer range cannot use `#`",
    );
    assert_err(
        "impl_foreach!(range = , body = { K; });",
        "expected a range",
    );
    assert_err(
        "impl_foreach!(0..4);",
        "expected a `{ ... }` body after the range",
    );
    assert_err(
        "impl_foreach!(range = 0..2, body = { K; } extra);",
        "unexpected token after the body",
    );
    assert_err(
        "impl_foreach!(not_a_range = 0..2 { K; });",
        "write `impl_foreach!(range = .., body = { .. })`",
    );
}

#[test]
fn pattern_errors() {
    assert_err(
        "impl_foreach!(range = 0..4, body = { Name[0..10]; });",
        "outside the domain",
    );
    assert_err(
        "impl_foreach!(range = 0..10::2, body = { Name[0..12]; });",
        "outside the domain",
    );
    assert_err(
        "impl_foreach!(range = 0..=10, body = { Name[4..=3]; });",
        "invalid range",
    );
    assert_err(
        "impl_foreach!(range = 0..4, body = { A[0..#, Off]; });",
        "exactly one range pattern",
    );
    assert_err(
        "impl_foreach!(range = 0..4, body = { Name[0..1..2]; });",
        "exactly one range pattern",
    );
    assert_err(
        "impl_foreach!(range = 0..4, body = { Name[0..2::]; });",
        "expected a step",
    );
    assert_err(
        "impl_foreach!(range = 0..4, body = { Name[0..4::0]; });",
        "step must be a positive integer",
    );
    assert_err(
        "impl_foreach!(range = 1..2, body = { Name[0..=#::#-1]; });",
        "step must be a positive integer",
    );
    assert_err(
        "impl_foreach!(range = 0..2, body = { Name[#-1..=#]; });",
        "outside the domain",
    );
    assert_err(
        "impl_foreach!(range = 0..4, body = { Name[#+]; });",
        "expected an integer literal",
    );
}

#[test]
fn unnamed_trailing_block_is_valid() {
    impl_foreach!(0..2 {
        let _k = #;
    });
    impl_foreach!(0..2, {
        let _comma = #;
    });
    impl_foreach!(range = 0..2, body = {
        let _named = #;
    });
}
