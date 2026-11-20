#![allow(missing_docs)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use ph_proc_macro_dylib::proc_macro_library;

#[test]
fn debug_library_uses_the_host_extension() {
    let mut fixture = Fixture::new();
    let name = "ph_dylib_debug";
    let path = fixture.write(&format!("debug/{}", file_name(name)), 10);
    let found = proc_macro_library(name);
    assert_eq!(found, path);
}

#[test]
fn target_root_library_uses_the_exact_file_name() {
    let mut fixture = Fixture::new();
    let name = "ph_dylib_root";
    let path = fixture.write(&file_name(name), 10);
    let found = proc_macro_library(name);
    assert_eq!(found, path);
}

#[test]
fn newest_deps_copy_wins() {
    let mut fixture = Fixture::new();
    let name = "ph_dylib_newest";
    let ext = extension();
    fixture.write(&format!("debug/{}", file_name(name)), 10);
    let hashed = fixture.write(&format!("debug/deps/lib{name}-hash.{ext}"), 40);
    let found = proc_macro_library(name);
    assert_eq!(found, hashed);
}

#[test]
fn build_out_directory_is_searched() {
    let mut fixture = Fixture::new();
    let name = "ph_dylib_build";
    let ext = extension();
    let path = fixture.write(
        &format!("debug/build/{name}/unit/out/lib{name}-hash.{ext}"),
        15,
    );
    let found = proc_macro_library(name);
    assert_eq!(found, path);
}

#[test]
fn workspace_ancestor_target_is_searched() {
    let mut fixture = Fixture::new();
    let name = "ph_dylib_workspace";
    let path = workspace_target().join(file_name(name));
    fixture.write_at(&path, 8);
    let found = proc_macro_library(name);
    assert_eq!(found, path);
}

#[test]
fn hyphenated_name_is_copied_into_the_file_name() {
    let mut fixture = Fixture::new();
    let name = "ph-dylib-hyphen";
    let path = fixture.write(&format!("debug/{}", file_name(name)), 12);
    let found = proc_macro_library(name);
    assert_eq!(found, path);
}

#[test]
#[should_panic(expected = "could not find libph_dylib_stem.")]
fn a_longer_crate_name_does_not_match() {
    let mut fixture = Fixture::new();
    let ext = extension();
    fixture.write(&format!("debug/deps/libph_dylib_stemextra.{ext}"), 40);
    let _ = proc_macro_library("ph_dylib_stem");
}

#[test]
#[should_panic(expected = "could not find libph_dylib_absent.")]
fn missing_library_panics() {
    let _ = proc_macro_library("ph_dylib_absent");
}

#[test]
#[should_panic(expected = "cargo package name")]
fn a_path_is_not_a_crate_name() {
    let _ = proc_macro_library("ph/util");
}

struct Fixture {
    files: Vec<PathBuf>,
}

impl Fixture {
    const fn new() -> Self {
        Self { files: Vec::new() }
    }

    fn write(&mut self, relative: &str, secs: u64) -> PathBuf {
        let path = target_root().join(relative);
        self.write_at(&path, secs);
        path
    }

    fn write_at(&mut self, path: &Path, secs: u64) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("fixture directory");
        }
        fs::write(path, b"proc-macro").expect("fixture file");
        fs::File::options()
            .write(true)
            .open(path)
            .expect("fixture file")
            .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(secs))
            .expect("fixture mtime");
        self.files.push(path.to_path_buf());
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for path in &self.files {
            let _ = fs::remove_file(path);
        }
    }
}

fn target_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target")
}

fn workspace_target() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(MANIFEST_PARENTS)
        .expect("workspace root")
        .join("target")
}

const MANIFEST_PARENTS: usize = 3;

const fn extension() -> &'static str {
    #[cfg(windows)]
    {
        "dll"
    }
    #[cfg(target_os = "macos")]
    {
        "dylib"
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        "so"
    }
}

fn file_name(name: &str) -> String {
    format!("lib{name}.{}", extension())
}
