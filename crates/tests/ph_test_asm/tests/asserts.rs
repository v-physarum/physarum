#![allow(missing_docs, dead_code)]

mod common;

use common::{SHAPES, host_triple, write_probe};
use ph_test_asm::{AsmTester, Rules};
use ph_test_errors::ErrorTester;

const VECTOR_LOAD_LIMIT: u32 = 8;
const NO_STORES: u32 = 0;
const VECTOR_INSTRUCTION_LIMIT: u32 = 64;
const EMPTY_BUDGET: u32 = 0;

fn shapes() -> std::path::PathBuf {
    write_probe("shapes.rs", SHAPES)
}

#[test]
fn hot_loop_accepts_release_vector_body() {
    let path = shapes();
    let triple = host_triple();
    AsmTester::new(&path).function("sum").hot_loop(|hot| {
        hot.assert_simd();
        hot.assert_no_bounds();
        hot.assert_no_branch();
        hot.assert_no_virtual_calls();
        hot.assert_no_select();
        hot.assert_no_div();
        hot.assert_no_invoke();
        hot.assert_no_alloc();
        hot.assert_no_atomic();
        hot.assert_no_memcpy();
        hot.assert_no_checks();
        hot.assert_no_gather();
        hot.assert_no_libm();
        hot.assert_no_fma();
        hot.assert_counted_loop();
        hot.assert_no_alloca_in_loop();
        hot.assert_noalias();
        hot.assert_max_loads(VECTOR_LOAD_LIMIT);
        hot.assert_max_stores(NO_STORES);
        hot.on_target(&triple)
            .assert_max_instructions(VECTOR_INSTRUCTION_LIMIT);
        Rules::new()
            .simd()
            .no_bounds()
            .no_branch()
            .no_virtual_calls()
            .counted_loop()
            .noalias()
            .apply(hot);
    });
}

#[test]
fn function_scope_rejects_branches_the_hot_loop_allows() {
    let path = shapes();
    AsmTester::new(&path).function("sum").on_function(|body| {
        body.assert_simd();
        body.assert_no_bounds();
    });
}

#[test]
#[should_panic(expected = "assertion failed: conditional branch")]
fn function_scope_sees_loop_branches() {
    let path = shapes();
    AsmTester::new(&path).function("sum").on_function(|body| {
        body.assert_no_branch();
    });
}

#[test]
#[should_panic(expected = "assertion failed: simd")]
fn scalar_add_is_not_simd() {
    let path = shapes();
    AsmTester::new(&path)
        .function("add")
        .on_function(ph_test_asm::Asserts::assert_simd);
}

#[test]
#[should_panic(expected = "has no loop")]
fn scalar_add_has_no_hot_loop() {
    let path = shapes();
    AsmTester::new(&path).function("add").hot_loop(|_| {});
}

#[test]
#[should_panic(expected = "was not defined")]
fn missing_function_names_the_symbol() {
    let path = shapes();
    AsmTester::new(&path)
        .function("missing")
        .on_function(|_| {});
}

#[test]
#[should_panic(expected = "assertion failed: bounds check")]
fn index_keeps_a_bounds_check() {
    let path = shapes();
    AsmTester::new(&path)
        .function("at")
        .on_function(ph_test_asm::Asserts::assert_no_bounds);
}

#[test]
#[should_panic(expected = "assertion failed: select")]
fn pick_is_a_select() {
    let path = shapes();
    AsmTester::new(&path)
        .function("pick")
        .on_function(ph_test_asm::Asserts::assert_no_select);
}

#[test]
fn direct_call_is_not_virtual() {
    let path = shapes();
    AsmTester::new(&path)
        .function("call_direct")
        .on_function(|body| {
            body.assert_no_virtual_calls();
        });
}

#[test]
#[should_panic(expected = "assertion failed: virtual call")]
fn function_pointer_is_virtual() {
    let path = shapes();
    AsmTester::new(&path)
        .function("call_fn")
        .on_function(ph_test_asm::Asserts::assert_no_virtual_calls);
}

#[test]
#[should_panic(expected = "assertion failed: division")]
fn quotient_divides() {
    let path = shapes();
    AsmTester::new(&path)
        .function("quotient")
        .on_function(ph_test_asm::Asserts::assert_no_div);
}

#[test]
#[should_panic(expected = "assertion failed: panic check")]
fn quotient_panics_on_zero() {
    let path = shapes();
    AsmTester::new(&path)
        .function("quotient")
        .on_function(ph_test_asm::Asserts::assert_no_checks);
}

#[test]
#[should_panic(expected = "assertion failed: allocation")]
fn heap_allocates() {
    let path = shapes();
    AsmTester::new(&path)
        .function("heap")
        .on_function(ph_test_asm::Asserts::assert_no_alloc);
}

#[test]
#[should_panic(expected = "assertion failed: atomic")]
fn bump_is_atomic() {
    let path = shapes();
    AsmTester::new(&path)
        .function("bump")
        .on_function(ph_test_asm::Asserts::assert_no_atomic);
}

#[test]
#[should_panic(expected = "assertion failed: memcpy")]
fn duplicate_copies_memory() {
    let path = shapes();
    AsmTester::new(&path)
        .function("duplicate")
        .on_function(ph_test_asm::Asserts::assert_no_memcpy);
}

#[test]
#[should_panic(expected = "assertion failed: libm")]
fn sine_calls_libm() {
    let path = shapes();
    AsmTester::new(&path)
        .function("sine")
        .on_function(ph_test_asm::Asserts::assert_no_libm);
}

#[test]
#[should_panic(expected = "assertion failed: fma")]
fn fused_calls_fma() {
    let path = shapes();
    AsmTester::new(&path)
        .function("fused")
        .on_function(ph_test_asm::Asserts::assert_no_fma);
}

#[test]
#[should_panic(expected = "assertion failed: invoke")]
fn dropping_panic_invokes() {
    let path = shapes();
    AsmTester::new(&path)
        .function("may_invoke")
        .on_function(ph_test_asm::Asserts::assert_no_invoke);
}

#[test]
#[should_panic(expected = "assertion failed: alloca in loop")]
fn black_box_buffer_allocates_in_the_loop() {
    let path = shapes();
    AsmTester::new(&path)
        .function("with_alloca")
        .hot_loop(ph_test_asm::Asserts::assert_no_alloca_in_loop);
}

#[test]
fn slice_pointer_is_noalias() {
    let path = shapes();
    AsmTester::new(&path)
        .function("length")
        .on_function(ph_test_asm::Asserts::assert_noalias);
}

#[test]
#[should_panic(expected = "assertion failed: noalias")]
fn raw_pointer_is_not_noalias() {
    let path = shapes();
    AsmTester::new(&path)
        .function("raw")
        .on_function(ph_test_asm::Asserts::assert_noalias);
}

#[test]
#[should_panic(expected = "assertion failed: counted loop")]
fn straight_line_add_is_not_counted() {
    let path = shapes();
    AsmTester::new(&path)
        .function("add")
        .on_function(ph_test_asm::Asserts::assert_counted_loop);
}

#[test]
#[should_panic(expected = "assertion failed: loads")]
fn hot_loop_load_cap_is_enforced() {
    let path = shapes();
    AsmTester::new(&path)
        .function("sum")
        .hot_loop(|hot| hot.assert_max_loads(EMPTY_BUDGET));
}

#[test]
#[should_panic(expected = "assertion failed: instructions")]
fn instruction_cap_rejects_an_empty_budget() {
    let path = shapes();
    let triple = host_triple();
    AsmTester::new(&path).function("sum").hot_loop(|hot| {
        hot.on_target(&triple).assert_max_instructions(EMPTY_BUDGET);
    });
}

#[cfg(target_arch = "x86_64")]
#[test]
#[should_panic(expected = "assertion failed: gather")]
fn gather_intrinsic_is_visible() {
    let path = shapes();
    AsmTester::new(&path)
        .function("gather")
        .on_function(ph_test_asm::Asserts::assert_no_gather);
}

#[test]
fn unclosed_delimiter_has_no_code() {
    ErrorTester::source("pub fn {\n").assert_description("unclosed delimiter");
}
