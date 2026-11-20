use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

const CASE_DIR_PREFIX: &str = "ph-test-errors-case";

static TICK: AtomicU64 = AtomicU64::new(0);

pub fn write_probe(name: &str, source: &str) -> PathBuf {
    let dir = scratch_dir();
    let path = dir.join(name);
    fs::write(&path, source).expect("probe");
    path
}

pub fn write_rlib(name: &str, source: &str) -> PathBuf {
    let dir = scratch_dir();
    let source_path = dir.join("lib.rs");
    fs::write(&source_path, source).expect("rlib source");
    let rlib = dir.join(format!("lib{name}.rlib"));
    let program = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
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
    let n = TICK.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("{CASE_DIR_PREFIX}-{}-{n}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    dir
}
