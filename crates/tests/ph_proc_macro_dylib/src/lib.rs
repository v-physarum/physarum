//! Path of a built proc-macro dynamic library.
//!
//! [`proc_macro_library`] looks through the usual cargo target layout and
//! returns the newest copy. The file is `lib<name>.dll` on Windows,
//! `lib<name>.dylib` on macOS, and `lib<name>.so` elsewhere.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

#[cfg(windows)]
const LIBRARY_EXTENSION: &str = "dll";
#[cfg(target_os = "macos")]
const LIBRARY_EXTENSION: &str = "dylib";
#[cfg(not(any(windows, target_os = "macos")))]
const LIBRARY_EXTENSION: &str = "so";

const LIBRARY_PREFIX: &str = "lib";
const HASH_SEPARATOR: &str = "-";
const DEBUG_DIRECTORY: &str = "debug";
const DEPS_DIRECTORY: &str = "deps";
const BUILD_DIRECTORY: &str = "build";
const OUT_DIRECTORY: &str = "out";
const TARGET_DIRECTORY: &str = "target";
const EXE_PARENTS: usize = 8;
const MANIFEST_ANCESTORS: usize = 4;

/// Newest built proc-macro library for `name`.
///
/// `name` is the cargo package name as it appears in the file name
/// (`ph_util_foreach`, hyphens included, not a path). The file is
/// `lib<name>.dll` on Windows, `lib<name>.dylib` on macOS, and `lib<name>.so`
/// elsewhere. Cargo's hashed copy `lib<name>-<hash>.<ext>` under `deps` is the
/// same library. A longer name such as `lib<name>extra.<ext>` is not.
///
/// Each root is searched at the root itself, `debug`, `deps`, `debug/deps`,
/// `build/<name>/*/out`, and `debug/build/<name>/*/out`. Roots are
/// `CARGO_TARGET_DIR` when it is set, eight ancestors of the running
/// executable, and `target` next to this crate's manifest directory and its
/// three parent directories. From `crates/tests/<crate>` those parents include
/// the workspace root, where cargo writes the library. When several copies
/// exist, the newest modification time wins.
///
/// # Panics
///
/// Panics when `name` is empty or is not a cargo package name, and when no
/// library file is found. The missing-file panic names the file and lists the
/// directories that were searched.
///
/// ```no_run
/// use ph_proc_macro_dylib::proc_macro_library;
///
/// let path = proc_macro_library("ph_util_foreach");
/// let _ = path.file_name();
/// ```
#[must_use]
#[track_caller]
pub fn proc_macro_library(name: &str) -> PathBuf {
    assert!(
        valid_name(name),
        "proc-macro crate name must be one cargo package name, got {name}"
    );
    let file = library_file(name);
    let roots = search_roots();
    let found = roots
        .iter()
        .filter_map(|root| find_library(root, name))
        .collect();
    newest_path(found).unwrap_or_else(|| panic!("could not find {file}; looked in {roots:?}"))
}

fn valid_name(name: &str) -> bool {
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    if !first.is_ascii_alphanumeric() && first != '_' {
        return false;
    }
    characters
        .all(|character| character.is_ascii_alphanumeric() || character == '_' || character == '-')
}

fn library_file(name: &str) -> String {
    format!("{LIBRARY_PREFIX}{name}.{LIBRARY_EXTENSION}")
}

fn library_prefix(name: &str) -> String {
    format!("{LIBRARY_PREFIX}{name}")
}

fn search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(target) = std::env::var_os("CARGO_TARGET_DIR") {
        roots.push(PathBuf::from(target));
    }
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(Path::to_path_buf);
        for _ in 0..EXE_PARENTS {
            let Some(current) = dir else {
                break;
            };
            roots.push(current.clone());
            dir = current.parent().map(Path::to_path_buf);
        }
    }
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    roots.push(manifest.join(TARGET_DIRECTORY));
    for ancestor in manifest.ancestors().take(MANIFEST_ANCESTORS) {
        roots.push(ancestor.join(TARGET_DIRECTORY));
    }
    roots
}

fn find_library(dir: &Path, name: &str) -> Option<PathBuf> {
    let file = library_file(name);
    let mut found = Vec::new();
    for candidate in [dir.join(&file), dir.join(DEBUG_DIRECTORY).join(&file)] {
        if candidate.is_file() {
            found.push(candidate);
        }
    }
    for deps in [
        dir.join(DEPS_DIRECTORY),
        dir.join(DEBUG_DIRECTORY).join(DEPS_DIRECTORY),
    ] {
        if let Some(library) = newest_named(&deps, name) {
            found.push(library);
        }
    }
    for build in [
        dir.join(BUILD_DIRECTORY).join(name),
        dir.join(DEBUG_DIRECTORY).join(BUILD_DIRECTORY).join(name),
    ] {
        let Ok(entries) = std::fs::read_dir(&build) else {
            continue;
        };
        for entry in entries.flatten() {
            if let Some(library) = newest_named(&entry.path().join(OUT_DIRECTORY), name) {
                found.push(library);
            }
        }
    }
    newest_path(found)
}

fn newest_named(dir: &Path, name: &str) -> Option<PathBuf> {
    let prefix = library_prefix(name);
    let entries = std::fs::read_dir(dir).ok()?;
    let matches = entries.flatten().filter_map(|entry| {
        let path = entry.path();
        let file_name = path.file_name()?.to_str()?;
        if is_library_file(file_name, &prefix) {
            Some(path)
        } else {
            None
        }
    });
    newest_path(matches.collect())
}

fn is_library_file(file_name: &str, prefix: &str) -> bool {
    let Some((stem, extension)) = file_name.rsplit_once('.') else {
        return false;
    };
    if !extension.eq_ignore_ascii_case(LIBRARY_EXTENSION) {
        return false;
    }
    stem == prefix || hashed_stem(stem, prefix)
}

fn hashed_stem(stem: &str, prefix: &str) -> bool {
    stem.strip_prefix(prefix)
        .is_some_and(|rest| rest.starts_with(HASH_SEPARATOR))
}

fn newest_path(paths: Vec<PathBuf>) -> Option<PathBuf> {
    paths.into_iter().max_by_key(|path| {
        path.metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
    })
}
