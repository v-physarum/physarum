//! Case groups read from a directory layout.
//!
//! Group and kind are folders. Each count is a non-empty `.rs` file in the
//! kind folder, and that file is the source.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, ErrorKind, Result};
use crate::source::{case_name, fill_count, read_exact, render};

const WARMUP: &str = "warmup";
const DEBUG: &str = "debug";
const INCREMENTAL: &str = "incremental";
const RELEASE: &str = "release";
const CASE_LEN: &str = "CASE_LEN";
const NEXT_KEY: &str = "NEXT";

/// One case, already rendered into downstream source.
#[derive(Debug)]
pub(crate) struct Case {
    /// `{kind}_{count}`, used in the report.
    pub(crate) name: String,
    /// Downstream `src/lib.rs`.
    pub(crate) source: String,
}

/// Cases in the order they run.
#[derive(Debug, Default)]
pub(crate) struct Groups {
    /// Compiled before the measured runs so the dependency is built.
    pub(crate) warmup: Vec<Case>,
    /// Full debug builds.
    pub(crate) debug: Vec<Case>,
    /// Incremental debug builds. The package is not cleaned first.
    pub(crate) incremental: Vec<Case>,
    /// Full release builds.
    pub(crate) release: Vec<Case>,
}

#[derive(Default)]
struct GroupPaths {
    warmup: Option<PathBuf>,
    debug: Option<PathBuf>,
    incremental: Option<PathBuf>,
    release: Option<PathBuf>,
}

struct KindDir {
    path: PathBuf,
    counts: Vec<CountFile>,
    item: Option<PathBuf>,
}

struct CountFile {
    count: u32,
    path: PathBuf,
}

/// Reads `directory` into warmup, debug, incremental, and release cases.
///
/// # Errors
///
/// Returns an error when the directory is not a case layout, a count file is
/// missing or duplicated, or a source file cannot be read.
pub(crate) fn load(directory: &Path) -> Result<Groups> {
    if !directory.is_dir() {
        return Err(ErrorKind::NotACaseDirectory {
            path: directory.display().to_string(),
        }
        .into());
    }
    let groups = split(directory)?;
    let mut discovered = Groups::default();
    push_group(&mut discovered.warmup, groups.warmup.as_deref())?;
    push_group(&mut discovered.debug, groups.debug.as_deref())?;
    push_group(&mut discovered.incremental, groups.incremental.as_deref())?;
    push_group(&mut discovered.release, groups.release.as_deref())?;
    if discovered.warmup.is_empty() {
        return Err(ErrorKind::MissingWarmup.into());
    }
    Ok(discovered)
}

fn split(directory: &Path) -> Result<GroupPaths> {
    let paths = list(directory)?;
    let mut groups = GroupPaths::default();
    for path in paths {
        if path.is_dir() {
            assign(&mut groups, path)?;
        } else {
            return Err(unexpected(&path));
        }
    }
    Ok(groups)
}

fn assign(groups: &mut GroupPaths, path: PathBuf) -> Result<()> {
    let group = match file_name(&path)? {
        WARMUP => Group::Warmup,
        DEBUG => Group::Debug,
        INCREMENTAL => Group::Incremental,
        RELEASE => Group::Release,
        _ => return Err(unexpected(&path)),
    };
    let slot = match group {
        Group::Warmup => &mut groups.warmup,
        Group::Debug => &mut groups.debug,
        Group::Incremental => &mut groups.incremental,
        Group::Release => &mut groups.release,
    };
    *slot = Some(path);
    Ok(())
}

enum Group {
    Warmup,
    Debug,
    Incremental,
    Release,
}

fn push_group(out: &mut Vec<Case>, directory: Option<&Path>) -> Result<()> {
    let Some(directory) = directory else {
        return Ok(());
    };
    let paths = list(directory)?;
    let mut kinds = Vec::with_capacity(paths.len());
    for path in paths {
        if !path.is_dir() {
            return Err(unexpected(&path));
        }
        kinds.push(read_kind(path)?);
    }
    let total = kinds.iter().map(|kind| kind.counts.len()).sum();
    out.reserve(total);
    for kind in &kinds {
        let name = file_name(&kind.path)?;
        reject_duplicates(name, &kind.counts)?;
        for count in &kind.counts {
            out.push(Case {
                name: case_name(name, count.count),
                source: source_for(count, kind.item.as_deref())?,
            });
        }
    }
    Ok(())
}

fn read_kind(path: PathBuf) -> Result<KindDir> {
    let children = list(&path)?;
    if children.is_empty() {
        return Err(unexpected(&path));
    }
    let kind = file_name(&path)?;
    let mut counts = Vec::with_capacity(children.len());
    let mut item = None;
    for child in children {
        match classify(&child, kind)? {
            Class::Count(count) => counts.push(CountFile { count, path: child }),
            Class::Item => {
                if item.is_some() {
                    return Err(unexpected(&child));
                }
                item = Some(child);
            }
        }
    }
    if counts.is_empty() {
        return Err(unexpected(&path));
    }
    counts.sort_unstable_by_key(|file| file.count);
    Ok(KindDir { path, counts, item })
}

enum Class {
    Count(u32),
    Item,
}

fn classify(path: &Path, kind: &str) -> Result<Class> {
    if !path.is_file() || !is_rust_source(path) {
        return Err(unexpected(path));
    }
    if fs::metadata(path)?.len() == 0 {
        return Err(unexpected(path));
    }
    let stem = file_stem(path).ok_or_else(|| unexpected(path))?;
    if let Some(count) = parse_count(stem) {
        return Ok(Class::Count(count));
    }
    if singular_stem(kind) == Some(stem) {
        return Ok(Class::Item);
    }
    Err(Error::unknown_case(stem))
}

fn source_for(count: &CountFile, item: Option<&Path>) -> Result<String> {
    let header = read_exact(&count.path)?;
    let Some(item) = item else {
        return Ok(fill_count(&header, CASE_LEN, count.count));
    };
    let item_text = read_exact(item)?;
    let next = item_text.contains(NEXT_KEY).then_some(NEXT_KEY);
    Ok(render(&header, &item_text, count.count, next))
}

fn parse_count(stem: &str) -> Option<u32> {
    if stem.is_empty() || !stem.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    stem.parse().ok()
}

/// `{kind}` without a trailing `s`.
fn singular_stem(kind: &str) -> Option<&str> {
    kind.strip_suffix('s').filter(|stem| !stem.is_empty())
}

fn reject_duplicates(kind: &str, counts: &[CountFile]) -> Result<()> {
    for pair in counts.windows(2) {
        if pair[0].count == pair[1].count {
            return Err(ErrorKind::DuplicateCase {
                case: case_name(kind, pair[0].count),
            }
            .into());
        }
    }
    Ok(())
}

fn list(directory: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(directory)? {
        paths.push(entry?.path());
    }
    paths.sort_by(|left, right| left.file_name().cmp(&right.file_name()));
    Ok(paths)
}

fn is_rust_source(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("rs"))
}

fn file_name(path: &Path) -> Result<&str> {
    path.file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| unexpected(path))
}

fn file_stem(path: &Path) -> Option<&str> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
}

fn unexpected(path: &Path) -> Error {
    ErrorKind::UnexpectedEntry {
        path: path.display().to_string(),
    }
    .into()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::load;

    #[test]
    fn directory_selects_groups_and_renders_source() {
        let tree = Tree::new();
        tree.file("warmup/hand_structs/1.rs", "use crate;\n");
        tree.file("warmup/hand_structs/hand_struct.rs", "struct IxINDEX;\n");
        tree.file("warmup/flat_structs/1.rs", "len = CASE_LEN;\n");
        tree.file("debug/flat_structs/1024.rs", "len = CASE_LEN;\n");
        tree.file("debug/hand_structs/2.rs", "use crate;\n");
        tree.file("debug/hand_structs/hand_struct.rs", "struct IxINDEX;\n");
        tree.file("debug/flat_structs/64.rs", "len = CASE_LEN;\n");
        tree.file("debug/many_calls/2.rs", "use crate;\n\n");
        tree.file("debug/many_calls/many_call.rs", "range = INDEX..NEXT;\n");
        tree.file("debug/fan_take/16.rs", "take = CASE_LEN;\n");
        tree.file("debug/fan_rows/4.rs", "rows = CASE_LEN;\n");
        tree.file("incremental/flat_structs/1024.rs", "len = CASE_LEN;\n");
        tree.file("release/hand_structs/2.rs", "use crate;\n");
        tree.file("release/hand_structs/hand_struct.rs", "struct IxINDEX;\n");
        tree.file("release/fan_rows/4.rs", "rows = CASE_LEN;\n");

        let groups = load(&tree.0).unwrap();
        assert_eq!(names(&groups.warmup), ["flat_structs_1", "hand_structs_1"]);
        assert_eq!(groups.warmup[0].source, "len = 1;\n");
        assert_eq!(groups.warmup[1].source, "use crate;\n\nstruct Ix0;\n");
        assert_eq!(
            names(&groups.debug),
            [
                "fan_rows_4",
                "fan_take_16",
                "flat_structs_64",
                "flat_structs_1024",
                "hand_structs_2",
                "many_calls_2",
            ]
        );
        assert_eq!(groups.debug[2].source, "len = 64;\n");
        assert_eq!(
            groups.debug[4].source,
            "use crate;\n\nstruct Ix0;\nstruct Ix1;\n"
        );
        assert_eq!(
            groups.debug[5].source,
            "use crate;\n\nrange = 0..1;\nrange = 1..2;\n"
        );
        assert_eq!(names(&groups.incremental), ["flat_structs_1024"]);
        assert_eq!(groups.incremental[0].source, "len = 1024;\n");
        assert_eq!(names(&groups.release), ["fan_rows_4", "hand_structs_2"]);
        assert_eq!(groups.release[0].source, "rows = 4;\n");
        assert_eq!(
            groups.release[1].source,
            "use crate;\n\nstruct Ix0;\nstruct Ix1;\n"
        );
    }

    #[test]
    fn absent_groups_are_empty_when_warmup_exists() {
        let tree = Tree::new();
        tree.file("warmup/flat_structs/1.rs", "len = CASE_LEN;\n");
        let groups = load(&tree.0).unwrap();
        assert!(groups.debug.is_empty());
        assert!(groups.incremental.is_empty());
        assert!(groups.release.is_empty());
    }

    #[test]
    fn missing_warmup_is_an_error() {
        let tree = Tree::new();
        tree.file("debug/flat_structs/1.rs", "len = CASE_LEN;\n");
        let error = load(&tree.0).unwrap_err().to_string();
        assert!(error.contains("warmup"), "{error}");
    }

    #[test]
    fn missing_directory_is_an_error() {
        let tree = Tree::new();
        let error = load(&tree.0.join("missing")).unwrap_err().to_string();
        assert!(error.contains("not a case directory"), "{error}");
    }

    #[test]
    fn a_file_is_not_a_case_directory() {
        let tree = Tree::new();
        tree.file("cases.txt", "nope");
        let error = load(&tree.0.join("cases.txt")).unwrap_err().to_string();
        assert!(error.contains("not a case directory"), "{error}");
    }

    #[test]
    fn unknown_count_is_an_error() {
        let tree = Tree::new();
        tree.file("warmup/flat_structs/1.rs", "len = CASE_LEN;\n");
        tree.file("debug/flat_structs/flat.rs", "len = CASE_LEN;\n");
        let error = load(&tree.0).unwrap_err().to_string();
        assert!(error.contains("unknown case"), "{error}");
    }

    #[test]
    fn duplicate_counts_are_an_error() {
        let tree = Tree::new();
        tree.file("warmup/flat_structs/1.rs", "len = CASE_LEN;\n");
        tree.file("debug/flat_structs/64.rs", "len = CASE_LEN;\n");
        tree.file("debug/flat_structs/064.rs", "len = 64;\n");
        let error = load(&tree.0).unwrap_err().to_string();
        assert!(error.contains("duplicate case"), "{error}");
    }

    #[test]
    fn empty_files_and_directories_are_an_error() {
        let empty_file = Tree::new();
        empty_file.file("warmup/flat_structs/1.rs", "len = CASE_LEN;\n");
        empty_file.file("debug/flat_structs/64.rs", "");
        let error = load(&empty_file.0).unwrap_err().to_string();
        assert!(error.contains("unexpected entry"), "{error}");

        let gitkeep = Tree::new();
        gitkeep.file("warmup/flat_structs/1.rs", "len = CASE_LEN;\n");
        gitkeep.file("debug/flat_structs/.gitkeep", "keep\n");
        let error = load(&gitkeep.0).unwrap_err().to_string();
        assert!(error.contains("unexpected entry"), "{error}");

        let count_dir = Tree::new();
        count_dir.file("warmup/flat_structs/1.rs", "len = CASE_LEN;\n");
        count_dir.dir("debug/flat_structs/64");
        let error = load(&count_dir.0).unwrap_err().to_string();
        assert!(error.contains("unexpected entry"), "{error}");

        let empty_kind = Tree::new();
        empty_kind.file("warmup/flat_structs/1.rs", "len = CASE_LEN;\n");
        empty_kind.dir("debug/flat_structs");
        let error = load(&empty_kind.0).unwrap_err().to_string();
        assert!(error.contains("unexpected entry"), "{error}");
    }

    #[test]
    fn unexpected_entries_are_an_error() {
        let tree = Tree::new();
        tree.file("warmup/flat_structs/1.rs", "len = CASE_LEN;\n");
        tree.file("readme.md", "notes");
        let error = load(&tree.0).unwrap_err().to_string();
        assert!(error.contains("unexpected entry"), "{error}");

        let nested = Tree::new();
        nested.file("warmup/flat_structs/1.rs", "len = CASE_LEN;\n");
        nested.dir("extra");
        let error = load(&nested.0).unwrap_err().to_string();
        assert!(error.contains("unexpected entry"), "{error}");

        let root_template = Tree::new();
        root_template.file("flat_structs.rs", "len = CASE_LEN;\n");
        root_template.file("warmup/flat_structs/1.rs", "len = CASE_LEN;\n");
        let error = load(&root_template.0).unwrap_err().to_string();
        assert!(error.contains("unexpected entry"), "{error}");

        let marker_file = Tree::new();
        marker_file.file("warmup/flat_structs_1", "len = CASE_LEN;\n");
        let error = load(&marker_file.0).unwrap_err().to_string();
        assert!(error.contains("unexpected entry"), "{error}");
    }

    fn names(cases: &[super::Case]) -> Vec<&str> {
        cases.iter().map(|case| case.name.as_str()).collect()
    }

    struct Tree(PathBuf);

    impl Tree {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("ph_bench_cases_{}_{id}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn file(&self, rel: &str, body: &str) {
            let path = self.0.join(rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, body).unwrap();
        }

        fn dir(&self, rel: &str) {
            fs::create_dir_all(self.0.join(rel)).unwrap();
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}
