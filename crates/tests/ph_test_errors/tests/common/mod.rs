use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

const CASE_DIR_PREFIX: &str = "ph-test-errors-case";
const DIR_NAME_SEPARATOR: &str = "-";
const RLIB_SOURCE_NAME: &str = "lib.rs";
const RLIB_FILE_PREFIX: &str = "lib";
const RLIB_EXTENSION: &str = ".rlib";
const RUSTC_PROGRAM: &str = "rustc";
const START_TICK: u64 = 0;
const TICK_STEP: u64 = 1;

static TICK: AtomicU64 = AtomicU64::new(START_TICK);

pub fn write_probe(name: &str, source: &str) -> PathBuf {
    let dir = scratch_dir();
    let path = dir.join(name);
    fs::write(&path, source).expect("probe");
    path
}

pub fn write_rlib(name: &str, source: &str) -> PathBuf {
    let dir = scratch_dir();
    let source_path = dir.join(RLIB_SOURCE_NAME);
    fs::write(&source_path, source).expect("rlib source");
    let rlib = dir.join(format!("{RLIB_FILE_PREFIX}{name}{RLIB_EXTENSION}"));
    let program = std::env::var("RUSTC").unwrap_or_else(|_| RUSTC_PROGRAM.to_owned());
    let output = Command::new(program)
        .arg("--edition")
        .arg("2024")
        .arg("--crate-type")
        .arg("lib")
        .arg("--crate-name")
        .arg(name)
        .arg(&source_path)
        .arg("-o")
        .arg(&rlib)
        .output()
        .expect("rustc");
    assert!(
        output.status.success(),
        "rlib `{name}` failed to build\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    rlib
}

fn scratch_dir() -> PathBuf {
    let n = TICK.fetch_add(TICK_STEP, Ordering::Relaxed);
    let id = std::process::id();
    let dir = std::env::temp_dir().join(format!(
        "{CASE_DIR_PREFIX}{DIR_NAME_SEPARATOR}{id}{DIR_NAME_SEPARATOR}{n}"
    ));
    fs::create_dir_all(&dir).expect("temp dir");
    dir
}
