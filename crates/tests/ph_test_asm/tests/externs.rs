#![allow(missing_docs, dead_code)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{scratch_dir, write_rlib};
use ph_test_asm::{AsmCompare, AsmTester};

const BASE: &str = r"
#[inline]
pub fn double(value: i32) -> i32 {
    value.wrapping_mul(2)
}
";

const DEMO: &str = r"
#[inline]
pub fn twice(value: i32) -> i32 {
    base::double(value)
}
";

const THROUGH_DEMO: &str = r"
#[unsafe(no_mangle)]
pub fn on_tick(value: i32) -> i32 {
    demo::twice(value)
}
";

const LOCAL: &str = r"
#[unsafe(no_mangle)]
pub fn on_tick(value: i32) -> i32 {
    value.wrapping_mul(2)
}
";

/// `libdemo.rlib`, which links `libbase.rlib` from the same directory.
fn demo_rlib(dir: &Path) -> PathBuf {
    let base = write_rlib(dir, "base", BASE, &[]);
    write_rlib(dir, "demo", DEMO, &[("base", &base)])
}

fn probe(dir: &Path, name: &str, source: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, source).expect("probe");
    path
}

#[test]
fn extern_crate_links_a_library_and_its_dependency() {
    let dir = scratch_dir();
    let demo = demo_rlib(&dir);
    let path = probe(&dir, "probe.rs", THROUGH_DEMO);
    AsmTester::new(&path)
        .extern_crate("demo", &demo)
        .function("on_tick")
        .on_function(|body| {
            body.assert_no_virtual_calls();
            body.assert_no_branch();
            body.assert_no_checks();
        });
}

#[test]
#[should_panic(expected = "rustc failed")]
fn a_probe_without_its_extern_crate_does_not_compile() {
    let dir = scratch_dir();
    let path = probe(&dir, "probe.rs", THROUGH_DEMO);
    AsmTester::new(&path)
        .function("on_tick")
        .on_function(|_| {});
}

#[test]
fn compare_links_the_extern_crate_for_both_probes() {
    let dir = scratch_dir();
    let demo = demo_rlib(&dir);
    let left = probe(&dir, "left.rs", THROUGH_DEMO);
    let right = probe(&dir, "right.rs", LOCAL);
    AsmCompare::files(&left, &right)
        .extern_crate("demo", &demo)
        .function("on_tick")
        .assert_equivalent();
}
