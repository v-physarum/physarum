#![allow(missing_docs)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use ph_build_test::Probe;

static TICK: AtomicU64 = AtomicU64::new(0);

fn write_probe(name: &str, source: &str) -> PathBuf {
    let n = TICK.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("ph-build-test-case-{}-{n}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(name);
    fs::write(&path, source).expect("probe");
    path
}

#[test]
fn missing_name_is_an_error_diagnostic() {
    let path = write_probe("missing.rs", "fn main() { missing; }\n");
    let diagnostics = Probe::new(&path).diagnostics().expect("diagnostics");
    assert!(!diagnostics.succeeded());
    assert!(diagnostics.error_codes().contains(&"E0425"));
    assert!(
        diagnostics
            .error_messages()
            .iter()
            .any(|message| message.contains("cannot find"))
    );
    let message = diagnostics
        .messages()
        .iter()
        .find(|item| item.code() == Some("E0425"))
        .expect("E0425");
    assert_eq!(message.spans()[0].line_start(), 1);
    assert!(message.spans()[0].file_name().ends_with("missing.rs"));
    assert!(
        message.spans()[0]
            .label()
            .is_some_and(|label| label.contains("not found"))
    );
}

#[test]
fn clean_probe_succeeds() {
    let path = write_probe(
        "ok.rs",
        "pub fn add(left: i32, right: i32) -> i32 { left.wrapping_add(right) }\n",
    );
    let diagnostics = Probe::new(path).diagnostics().expect("diagnostics");
    assert!(diagnostics.succeeded());
    assert_eq!(diagnostics.error_codes(), [] as [&str; 0]);
}

#[test]
fn missing_file_is_an_error() {
    let missing = std::env::temp_dir().join(format!(
        "ph-build-test-absent-{}-{}.rs",
        std::process::id(),
        TICK.fetch_add(1, Ordering::Relaxed)
    ));
    let diagnostics = Probe::new(missing).diagnostics().expect("diagnostics");
    assert!(!diagnostics.succeeded());
    assert!(
        diagnostics
            .error_messages()
            .iter()
            .any(|message| message.contains("couldn't find file"))
    );
}
