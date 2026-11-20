#![allow(missing_docs)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use ph_test_errors::ErrorTester;

static TICK: AtomicU64 = AtomicU64::new(0);

fn write_probe(name: &str, source: &str) -> PathBuf {
    let n = TICK.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("ph-test-errors-case-{}-{n}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(name);
    fs::write(&path, source).expect("probe");
    path
}

#[test]
fn missing_name_is_an_error_diagnostic() {
    let path = write_probe("missing.rs", "fn main() { missing; }\n");
    ErrorTester::file(&path)
        .assert_code("E0425")
        .assert_description("cannot find")
        .assert_fail();
}

#[test]
fn clean_probe_succeeds() {
    let path = write_probe(
        "ok.rs",
        "pub fn add(left: i32, right: i32) -> i32 { left.wrapping_add(right) }\n",
    );
    ErrorTester::file(path).assert_pass();
}

#[test]
fn missing_file_is_an_error() {
    let missing = std::env::temp_dir().join(format!(
        "ph-test-errors-absent-{}-{}.rs",
        std::process::id(),
        TICK.fetch_add(1, Ordering::Relaxed)
    ));
    ErrorTester::file(missing)
        .assert_description("couldn't find file")
        .assert_fail();
}
