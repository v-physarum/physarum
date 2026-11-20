use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

const CASE_DIR_PREFIX: &str = "ph-test-errors-case";

static TICK: AtomicU64 = AtomicU64::new(0);

pub fn write_probe(name: &str, source: &str) -> PathBuf {
    let n = TICK.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("{CASE_DIR_PREFIX}-{}-{n}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(name);
    fs::write(&path, source).expect("probe");
    path
}
