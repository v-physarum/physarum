#![allow(missing_docs, clippy::must_use_candidate)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

fn proc_macro_library() -> PathBuf {
    let mut roots = Vec::new();
    if let Some(target) = std::env::var_os("CARGO_TARGET_DIR") {
        roots.push(PathBuf::from(target));
    }
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(Path::to_path_buf);
        for _ in 0..8 {
            let Some(current) = dir else {
                break;
            };
            roots.push(current.clone());
            dir = current.parent().map(Path::to_path_buf);
        }
    }
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    if let Some(workspace) = manifest.ancestors().nth(3) {
        roots.push(workspace.join("target"));
    }
    let found = roots.iter().filter_map(|root| find_dylib(root)).collect();
    newest_path(found)
        .unwrap_or_else(|| panic!("could not find libph_util_foreach.so; looked in {roots:?}"))
}

fn find_dylib(dir: &Path) -> Option<PathBuf> {
    let mut found = Vec::new();
    for candidate in [
        dir.join("libph_util_foreach.so"),
        dir.join("debug/libph_util_foreach.so"),
    ] {
        if candidate.is_file() {
            found.push(candidate);
        }
    }
    for deps in [dir.join("deps"), dir.join("debug/deps")] {
        if let Some(library) = newest_library(&deps) {
            found.push(library);
        }
    }
    for build in [
        dir.join("build/ph_util_foreach"),
        dir.join("debug/build/ph_util_foreach"),
    ] {
        let Ok(entries) = std::fs::read_dir(&build) else {
            continue;
        };
        for entry in entries.flatten() {
            if let Some(library) = newest_library(&entry.path().join("out")) {
                found.push(library);
            }
        }
    }
    newest_path(found)
}

fn newest_library(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let matches = entries.flatten().filter_map(|entry| {
        let path = entry.path();
        let name = path.file_name()?.to_str()?;
        if name.starts_with("libph_util_foreach") && name.ends_with(".so") {
            Some(path)
        } else {
            None
        }
    });
    newest_path(matches.collect())
}

fn newest_path(paths: Vec<PathBuf>) -> Option<PathBuf> {
    paths.into_iter().max_by_key(|path| {
        path.metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
    })
}

fn assert_err(input: &str, needle: &str) {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("ph-util-foreach-{}-{id}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap_or_else(|error| panic!("temp dir: {error}"));
    let source = dir.join("input.rs");
    let output = dir.join("input.rmeta");
    let program = format!("use ph_util_foreach::impl_foreach;\n{input}\n");
    std::fs::write(&source, program).unwrap_or_else(|error| panic!("write source: {error}"));
    let library = proc_macro_library();
    let compiled = std::process::Command::new("rustc")
        .arg("--edition")
        .arg("2024")
        .arg("--crate-type")
        .arg("lib")
        .arg("--emit=metadata")
        .arg("--cap-lints")
        .arg("allow")
        .arg("--extern")
        .arg(format!("ph_util_foreach={}", library.display()))
        .arg(&source)
        .arg("-o")
        .arg(&output)
        .output()
        .unwrap_or_else(|error| panic!("rustc: {error}"));
    let stderr = String::from_utf8_lossy(&compiled.stderr);
    assert!(
        !compiled.status.success(),
        "expected a compile error for `{input}`, stderr:\n{stderr}"
    );
    assert!(
        stderr.contains(needle),
        "error for `{input}` does not contain `{needle}`:\n{stderr}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn spaced_paste_and_negative_index_are_errors() {
    assert_err(
        "impl_foreach! { (-1..0) { struct Ix + #; } }",
        "cannot paste a negative index",
    );
    assert_err(
        "impl_foreach! { (-1..0) { Name[#]; } }",
        "fan mode domain `0..=index` is empty",
    );
    assert_err("impl_foreach! { (0..1) { struct Ix #; } }", "Name + #");
    assert_err("impl_foreach! { (0..1) { struct Ix#; } }", "Name + #");
    assert_err("impl_foreach! { (0..2) { (T #)[0..1]; } }", "Name + #");
    assert_err("impl_foreach! { (0..1) { fn at #() {} } }", "Name + #");
    assert_err(
        "impl_foreach! { (0..2) { X Name[0..1]; } }",
        "not concatenation",
    );
}

#[test]
fn outer_range_errors() {
    for input in [
        "impl_foreach! { (0..) { K; } }",
        "impl_foreach! { (..4) { K; } }",
        "impl_foreach! { (..=4) { K; } }",
        "impl_foreach! { (..) { K; } }",
        "impl_foreach! { (0..::2) { K; } }",
        "impl_foreach! { (..4::2) { K; } }",
        "impl_foreach! { (..=4::2) { K; } }",
        "impl_foreach! { (..::2) { K; } }",
    ] {
        assert_err(input, "open outer range");
    }
    assert_err("impl_foreach! { (5..3) { K; } }", "invalid range");
    assert_err("impl_foreach! { (5..=3) { K; } }", "invalid range");
    assert_err("impl_foreach! { (5..4) { K; } }", "invalid range");
    assert_err(
        "impl_foreach! { (0..4::0) { K; } }",
        "step must be a positive integer",
    );
    assert_err(
        "impl_foreach! { (0..4::-1) { K; } }",
        "step must be a positive integer",
    );
    assert_err(
        "impl_foreach! { (0..#) { K; } }",
        "outer range cannot use `#`",
    );
    assert_err(
        "impl_foreach! { (0..4::#) { K; } }",
        "outer range cannot use `#`",
    );
    assert_err("impl_foreach! { () { K; } }", "expected a range");
    assert_err(
        "impl_foreach! { (0..4) }",
        "expected a `{ ... }` body after the range",
    );
    assert_err(
        "impl_foreach! { (0..2) { K; } extra }",
        "unexpected token after the body",
    );
    assert_err(
        "impl_foreach!(0..2 { K; });",
        "write `impl_foreach! { (range) { body } }`",
    );
}

#[test]
fn pattern_errors() {
    assert_err(
        "impl_foreach! { (0..4) { Name[0..10]; } }",
        "outside the domain",
    );
    assert_err(
        "impl_foreach! { (0..10::2) { Name[0..12]; } }",
        "outside the domain",
    );
    assert_err(
        "impl_foreach! { (0..=10) { Name[4..=3]; } }",
        "invalid range",
    );
    assert_err(
        "impl_foreach! { (0..4) { A[0..#, Off]; } }",
        "exactly one range pattern",
    );
    assert_err(
        "impl_foreach! { (0..4) { Name[0..1..2]; } }",
        "exactly one range pattern",
    );
    assert_err(
        "impl_foreach! { (0..4) { Name[0..2::]; } }",
        "expected a step",
    );
    assert_err(
        "impl_foreach! { (0..4) { Name[0..4::0]; } }",
        "step must be a positive integer",
    );
    assert_err(
        "impl_foreach! { (1..2) { Name[0..=#::#-1]; } }",
        "step must be a positive integer",
    );
    assert_err(
        "impl_foreach! { (0..2) { Name[#-1..=#]; } }",
        "outside the domain",
    );
    assert_err(
        "impl_foreach! { (0..4) { Name[#+]; } }",
        "expected an integer literal",
    );
}
