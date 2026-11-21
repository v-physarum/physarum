//! Compile-fail pairs for the derive and for a missing provider.
//!
//! Each rejection has a probe here. The matching program that compiles is the
//! tree test: distinct beans, a bean in only one list, and a parent that
//! provides every inject.

#![forbid(unsafe_code)]
#![allow(missing_docs)]

use std::fs;
use std::path::{Path, PathBuf};

use ph_proc_macro_dylib::proc_macro_library;
use ph_test_errors::ErrorTester;

#[test]
fn duplicate_provider_is_rejected() {
    fail(
        "
        #[derive(Component)]
        #[provider(Count, Count)]
        struct Parent;
        ",
        "duplicate bean in `provider`",
    );
}

#[test]
fn duplicate_inject_is_rejected() {
    fail(
        "
        #[derive(Component)]
        #[inject(Count, Count)]
        struct Child;
        ",
        "duplicate bean in `inject`",
    );
}

#[test]
fn a_bean_in_both_lists_is_rejected() {
    fail(
        "
        #[derive(Component)]
        #[provider(Count)]
        #[inject(Count)]
        struct Child;
        ",
        "bean is in both `provider` and `inject`",
    );
}

#[test]
fn a_missing_provider_names_the_parent() {
    fail(
        "
        #[derive(Component, Default)]
        struct Count(u32);

        #[derive(Component)]
        #[inject(Count)]
        struct Child;

        impl ph_beans::Tick for Child {
            type Beans<'a> = ChildBeans<'a>;
            fn tick(_: Self::Beans<'_>) {}
        }

        #[derive(Component)]
        #[provider(Child)]
        struct Parent;

        impl ph_beans::Tick for Parent {
            type Beans<'a> = ParentBeans<'a>;
            fn tick(_: Self::Beans<'_>) {}
        }
        ",
        "does not provide",
    );
}

#[test]
fn five_providers_are_rejected() {
    fail(
        "
        #[derive(Component)]
        #[provider(A, B, C, D, E)]
        struct Parent;
        ",
        "accepts at most 4 beans",
    );
}

#[test]
fn a_generic_component_is_rejected() {
    fail(
        "
        #[derive(Component)]
        struct Holder<T>(T);
        ",
        "does not support generic",
    );
}

#[test]
fn four_injects_are_rejected() {
    fail(
        "
        #[derive(Component)]
        #[inject(A, B, C, D)]
        struct Child;
        ",
        "accepts at most 3 beans",
    );
}

fn fail(body: &str, description: &str) {
    // `ph_beans` is built with `impl_foreach`. Naming that proc-macro crate
    // here is what lets rustc load it for the probe; `--extern` alone does not.
    let source =
        format!("extern crate ph_util_foreach;\nuse ph_beans_derive::Component;\n{body}\n");
    let rlib = ph_beans_rlib();
    ErrorTester::source(source)
        .extern_crate("ph_beans", rlib.clone())
        .extern_crate("ph_beans_derive", proc_macro_library("ph_beans_derive"))
        .extern_crate("ph_util_foreach", foreach_dylib(&rlib))
        .assert_description(description);
}

fn ph_beans_rlib() -> PathBuf {
    let roots = target_roots();
    let mut found = Vec::new();
    for root in &roots {
        collect_rlibs(&root.join(profile()), &mut found);
        collect_rlibs(root, &mut found);
    }
    newest(found).unwrap_or_else(|| panic!("could not find libph_beans.rlib; looked in {roots:?}"))
}

/// Proc-macro dylib that this `ph_beans` rlib was built against.
///
/// Several `libph_util_foreach-*` copies can sit in the target directory.
/// The rlib records the extra filename of the copy it used.
fn foreach_dylib(rlib: &Path) -> PathBuf {
    let bytes = fs::read(rlib).unwrap_or_else(|error| {
        panic!("could not read {}: {error}", rlib.display());
    });
    let roots = target_roots();
    let mut found = Vec::new();
    for root in &roots {
        collect_foreach(&root.join(profile()), &mut found);
        collect_foreach(root, &mut found);
    }
    let matched = found
        .into_iter()
        .filter(|path| hash_is_in(path, &bytes))
        .collect();
    newest(matched).unwrap_or_else(|| {
        panic!(
            "could not find the ph_util_foreach dylib recorded in {}; looked in {roots:?}",
            rlib.display()
        )
    })
}

fn target_roots() -> Vec<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut roots = Vec::new();
    if let Some(dir) = std::env::var_os("CARGO_TARGET_DIR") {
        roots.push(PathBuf::from(dir));
    }
    roots.push(manifest.join("../../target"));
    roots
}

fn profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

fn collect_rlibs(root: &Path, found: &mut Vec<PathBuf>) {
    push_rlib(&root.join("libph_beans.rlib"), found);
    collect_dir(&root.join("deps"), found);
    let build = root.join("build").join("ph_beans");
    let Ok(entries) = fs::read_dir(&build) else {
        return;
    };
    for entry in entries.flatten() {
        collect_dir(&entry.path().join("out"), found);
    }
}

fn collect_dir(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        push_rlib(&entry.path(), found);
    }
}

fn collect_foreach(root: &Path, found: &mut Vec<PathBuf>) {
    let build = root.join("build").join("ph_util_foreach");
    let Ok(entries) = fs::read_dir(&build) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(files) = fs::read_dir(entry.path().join("out")) else {
            continue;
        };
        for file in files.flatten() {
            let path = file.path();
            let Some(name) = path.file_name().and_then(|item| item.to_str()) else {
                continue;
            };
            if is_dylib(&path) && name.starts_with("libph_util_foreach-") {
                found.push(path);
            }
        }
    }
}

fn is_dylib(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| matches!(ext, "so" | "dylib" | "dll"))
}

fn hash_is_in(path: &Path, bytes: &[u8]) -> bool {
    let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
        return false;
    };
    let Some((_, hash)) = name.rsplit_once('-') else {
        return false;
    };
    if hash.is_empty() {
        return false;
    }
    bytes
        .windows(hash.len())
        .any(|window| window == hash.as_bytes())
}

fn push_rlib(path: &Path, found: &mut Vec<PathBuf>) {
    let Some(name) = path.file_name().and_then(|file| file.to_str()) else {
        return;
    };
    let rlib = path.extension().is_some_and(|ext| ext == "rlib");
    if rlib && (name == "libph_beans.rlib" || name.starts_with("libph_beans-")) {
        found.push(path.to_path_buf());
    }
}

fn newest(found: Vec<PathBuf>) -> Option<PathBuf> {
    found.into_iter().max_by_key(|path| {
        path.metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
    })
}
