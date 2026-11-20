//! Case groups read from a directory layout.
//!
//! Group folders name the runs. Marker file names name the cases. `.rs` files
//! next to those folders are the source.

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

struct TemplateFile {
    path: PathBuf,
}

struct Templates {
    files: Vec<TemplateFile>,
}

#[derive(Default)]
struct GroupPaths {
    warmup: Option<PathBuf>,
    debug: Option<PathBuf>,
    incremental: Option<PathBuf>,
    release: Option<PathBuf>,
}

struct Marker {
    name: String,
    /// `name` is `{kind}_{count}`. `kind` is the borrowed prefix.
    kind_len: usize,
    count: u32,
}

impl Marker {
    fn kind(&self) -> &str {
        &self.name[..self.kind_len]
    }

    fn parse(path: &Path) -> Result<Self> {
        let stem = file_stem(path).ok_or_else(|| unexpected(path))?;
        let Some((kind, count_text)) = stem.rsplit_once('_') else {
            return Err(Error::unknown_case(stem));
        };
        if kind.is_empty() {
            return Err(Error::unknown_case(stem));
        }
        let Ok(count) = count_text.parse::<u32>() else {
            return Err(Error::unknown_case(stem));
        };
        let kind_len = kind.len();
        Ok(Self {
            name: case_name(kind, count),
            kind_len,
            count,
        })
    }
}

/// Reads `directory` into warmup, debug, incremental, and release cases.
///
/// # Errors
///
/// Returns an error when the directory is not a case layout, a marker or
/// template is missing or duplicated, or a template file cannot be read.
pub(crate) fn load(directory: &Path) -> Result<Groups> {
    if !directory.is_dir() {
        return Err(ErrorKind::NotACaseDirectory {
            path: directory.display().to_string(),
        }
        .into());
    }
    let (templates, groups) = split(directory)?;
    let mut discovered = Groups::default();
    push_group(&mut discovered.warmup, groups.warmup.as_deref(), &templates)?;
    push_group(&mut discovered.debug, groups.debug.as_deref(), &templates)?;
    push_group(
        &mut discovered.incremental,
        groups.incremental.as_deref(),
        &templates,
    )?;
    push_group(
        &mut discovered.release,
        groups.release.as_deref(),
        &templates,
    )?;
    if discovered.warmup.is_empty() {
        return Err(ErrorKind::MissingWarmup.into());
    }
    Ok(discovered)
}

fn split(directory: &Path) -> Result<(Templates, GroupPaths)> {
    let paths = list(directory)?;
    let mut files = Vec::with_capacity(paths.len());
    let mut groups = GroupPaths::default();
    for path in paths {
        if path.is_dir() {
            assign(&mut groups, path)?;
        } else if path.is_file() && is_rust_source(&path) && file_stem(&path).is_some() {
            files.push(TemplateFile { path });
        } else {
            return Err(unexpected(&path));
        }
    }
    Ok((Templates { files }, groups))
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

fn push_group(out: &mut Vec<Case>, directory: Option<&Path>, templates: &Templates) -> Result<()> {
    let Some(directory) = directory else {
        return Ok(());
    };
    let paths = list(directory)?;
    let mut markers = Vec::with_capacity(paths.len());
    for path in paths {
        if !path.is_file() {
            return Err(unexpected(&path));
        }
        markers.push(Marker::parse(&path)?);
    }
    markers.sort_by(|left, right| {
        left.kind()
            .cmp(right.kind())
            .then(left.count.cmp(&right.count))
    });
    out.reserve(markers.len());
    for marker in markers {
        if out.iter().any(|case| case.name == marker.name) {
            return Err(ErrorKind::DuplicateCase { case: marker.name }.into());
        }
        let source = templates.source(marker.kind(), marker.count)?;
        out.push(Case {
            name: marker.name,
            source,
        });
    }
    Ok(())
}

impl Templates {
    fn source(&self, kind: &str, count: u32) -> Result<String> {
        let Some(header) = self
            .files
            .iter()
            .find(|file| file_stem(&file.path) == Some(kind))
        else {
            return Err(ErrorKind::MissingTemplate {
                case: case_name(kind, count),
                template: format!("{kind}.rs"),
            }
            .into());
        };
        if let Some(item) = singular(&self.files, kind) {
            let header_text = read_exact(&header.path)?;
            let item_text = read_exact(&item.path)?;
            let next = item_text.contains(NEXT_KEY).then_some(NEXT_KEY);
            return Ok(render(&header_text, &item_text, count, next));
        }
        let text = read_exact(&header.path)?;
        Ok(fill_count(&text, CASE_LEN, count))
    }
}

/// `{kind}` without a trailing `s`, when that stem is one of the templates.
fn singular<'a>(files: &'a [TemplateFile], kind: &str) -> Option<&'a TemplateFile> {
    let stem = kind.strip_suffix('s').filter(|stem| !stem.is_empty())?;
    files
        .iter()
        .find(|file| file_stem(&file.path) == Some(stem))
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
        tree.file("flat_structs.rs", "len = CASE_LEN;\n");
        tree.file("hand_structs.rs", "use crate;\n");
        tree.file("hand_struct.rs", "struct IxINDEX;\n");
        tree.file("many_calls.rs", "use crate;\n\n");
        tree.file("many_call.rs", "range = INDEX..NEXT;\n");
        tree.file("fan_rows.rs", "rows = CASE_LEN;\n");
        tree.file("fan_take.rs", "take = CASE_LEN;\n");
        tree.file("warmup/hand_structs_1", "IGNORE");
        tree.file("warmup/flat_structs_1", "IGNORE");
        tree.file("debug/flat_structs_1024", "IGNORE");
        tree.file("debug/hand_structs_2", "IGNORE");
        tree.file("debug/flat_structs_64", "IGNORE");
        tree.file("debug/many_calls_2", "IGNORE");
        tree.file("debug/fan_take_16", "IGNORE");
        tree.file("debug/fan_rows_4", "IGNORE");
        tree.file("incremental/flat_structs_1024", "");
        tree.file("release/hand_structs_2", "");
        tree.file("release/fan_rows_4", "");

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
        assert!(!groups.debug[2].source.contains("IGNORE"));
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
    }

    #[test]
    fn absent_groups_are_empty_when_warmup_exists() {
        let tree = Tree::new();
        tree.file("flat_structs.rs", "len = CASE_LEN;\n");
        tree.file("warmup/flat_structs_1", "");
        let groups = load(&tree.0).unwrap();
        assert!(groups.debug.is_empty());
        assert!(groups.incremental.is_empty());
        assert!(groups.release.is_empty());
    }

    #[test]
    fn missing_warmup_is_an_error() {
        let tree = Tree::new();
        tree.file("flat_structs.rs", "len = CASE_LEN;\n");
        tree.file("debug/flat_structs_1", "");
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
    fn unknown_marker_is_an_error() {
        let tree = Tree::new();
        tree.file("flat_structs.rs", "len = CASE_LEN;\n");
        tree.file("warmup/flat_structs_1", "");
        tree.file("debug/flat", "");
        let error = load(&tree.0).unwrap_err().to_string();
        assert!(error.contains("unknown case"), "{error}");
    }

    #[test]
    fn missing_template_names_the_kind() {
        let tree = Tree::new();
        tree.file("flat_structs.rs", "len = CASE_LEN;\n");
        tree.file("warmup/flat_structs_1", "");
        tree.file("debug/one_call_256", "");
        let error = load(&tree.0).unwrap_err().to_string();
        assert!(error.contains("one_call.rs"), "{error}");
    }

    #[test]
    fn duplicate_counts_are_an_error() {
        let tree = Tree::new();
        tree.file("flat_structs.rs", "len = CASE_LEN;\n");
        tree.file("warmup/flat_structs_1", "");
        tree.file("debug/flat_structs_64", "");
        tree.file("debug/flat_structs_064", "");
        let error = load(&tree.0).unwrap_err().to_string();
        assert!(error.contains("duplicate case"), "{error}");
    }

    #[test]
    fn unexpected_entries_are_an_error() {
        let tree = Tree::new();
        tree.file("flat_structs.rs", "len = CASE_LEN;\n");
        tree.file("readme.md", "notes");
        let error = load(&tree.0).unwrap_err().to_string();
        assert!(error.contains("unexpected entry"), "{error}");

        let nested = Tree::new();
        nested.file("flat_structs.rs", "len = CASE_LEN;\n");
        nested.file("warmup/flat_structs_1", "");
        nested.dir("extra");
        let error = load(&nested.0).unwrap_err().to_string();
        assert!(error.contains("unexpected entry"), "{error}");

        let marker_dir = Tree::new();
        marker_dir.file("flat_structs.rs", "len = CASE_LEN;\n");
        marker_dir.dir("debug/flat_structs_1");
        let error = load(&marker_dir.0).unwrap_err().to_string();
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
