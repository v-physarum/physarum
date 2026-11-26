//! Performance requirements on release assembly (`crates/ph_beans/docs/performance.md`).
//!
//! The probe `probes/emit.rs` is compiled in release against the built
//! `ph_beans` library. Each system writes its own constant, so the store
//! count shows which systems run.

#![forbid(unsafe_code)]
#![allow(missing_docs)]

mod library;

use std::path::PathBuf;

use library::{CRATE, library};
use ph_test_asm::{AsmTester, Asserts};

/// Stores of `emit_direct`: the two systems write their adjacent `u32` states
/// in one store. No map is deferrable, so the scope buffer is not touched.
const DIRECT_STORES: u32 = 1;

fn probe() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/crates/ph_beans/probes/emit.rs")
}

fn on(function: &str, check: impl FnOnce(&Asserts)) {
    AsmTester::new(probe())
        .extern_crate(CRATE, library())
        .function(function)
        .on_function(check);
}

/// No call, allocation or unwind path from `ph_beans` on the emit path.
fn plain(body: &Asserts) {
    body.assert_no_virtual_calls();
    body.assert_no_alloc();
    body.assert_no_invoke();
}

// PERF-1, step 1: the systems of the emitted event are a direct call. With
// nothing deferrable there is no branch, no load, and no store but theirs.
#[test]
fn perf_1_a_direct_emit_has_no_branch_and_no_load() {
    on("emit_direct", |body| {
        plain(body);
        body.assert_no_branch();
        body.assert_max_loads(0);
        body.assert_max_stores(DIRECT_STORES);
    });
}

// PERF-1, step 2 gate: with no `.emit` nothing can be deferred. The queue is
// not read: no load, no branch, and only the store of the direct system.
#[test]
fn perf_1_a_pipeline_without_emit_never_reads_the_queue() {
    on("emit_without_declared", |body| {
        plain(body);
        body.assert_no_branch();
        body.assert_max_loads(0);
        body.assert_max_stores(DIRECT_STORES);
    });
}

// PERF-1 holds across beans: a `.forward` into a provided bean is the
// child's system, inlined into the parent's emit.
#[test]
fn perf_1_a_forward_into_a_child_is_a_direct_call() {
    on("emit_into_child", |body| {
        plain(body);
        body.assert_no_branch();
        body.assert_max_loads(0);
        body.assert_max_stores(DIRECT_STORES);
    });
}

// A descriptor read in a system is a constant of the build: the injected
// slot is not read, and the only store is the system's.
#[test]
fn perf_1_a_descriptor_read_is_a_constant() {
    on("emit_reads_info", |body| {
        plain(body);
        body.assert_no_branch();
        body.assert_max_loads(0);
        body.assert_max_stores(DIRECT_STORES);
    });
}

// PERF-1, step 2: the queue is walked in a loop of plain compares and
// stores. Nothing is called, allocated or unwound, in the loop or around it.
#[test]
fn perf_1_the_defer_queue_runs_without_calls() {
    on("emit_with_deferred", plain);
    AsmTester::new(probe())
        .extern_crate(CRATE, library())
        .function("emit_with_deferred")
        .hot_loop(plain);
}
