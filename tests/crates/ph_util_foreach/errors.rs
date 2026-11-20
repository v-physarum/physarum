#![allow(missing_docs, clippy::must_use_candidate)]

use std::path::{Path, PathBuf};

use ph_test_errors::ErrorTester;

#[cfg(windows)]
const LIBRARY_EXTENSION: &str = "dll";
#[cfg(target_os = "macos")]
const LIBRARY_EXTENSION: &str = "dylib";
#[cfg(not(any(windows, target_os = "macos")))]
const LIBRARY_EXTENSION: &str = "so";

const LIBRARY_PREFIX: &str = "libph_util_foreach";

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
    roots.push(manifest.join("target"));
    for ancestor in manifest.ancestors().take(4) {
        roots.push(ancestor.join("target"));
    }
    let found = roots.iter().filter_map(|root| find_dylib(root)).collect();
    newest_path(found).unwrap_or_else(|| {
        panic!("could not find {LIBRARY_PREFIX}.{LIBRARY_EXTENSION}; looked in {roots:?}")
    })
}

fn library_file() -> String {
    format!("{LIBRARY_PREFIX}.{LIBRARY_EXTENSION}")
}

fn find_dylib(dir: &Path) -> Option<PathBuf> {
    let file = library_file();
    let mut found = Vec::new();
    for candidate in [dir.join(&file), dir.join("debug").join(&file)] {
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
        let is_shared = path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case(LIBRARY_EXTENSION));
        if name.starts_with(LIBRARY_PREFIX) && is_shared {
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

fn assert_err(input: &str, description: &str) {
    let source = format!("use ph_util_foreach::impl_foreach;\n{input}\n");
    ErrorTester::source(source)
        .extern_crate("ph_util_foreach", proc_macro_library())
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
        "impl_foreach!(0..2 { K; });",
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
