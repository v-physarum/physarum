//! Workspace root, member crates, and `use` paths for a measured build.
//!
//! A test does not pass the workspace root, the macro crate, the user package,
//! or the output directory. Those are read from the `Cargo.toml` files above
//! `CARGO_MANIFEST_DIR` and from `use` paths in the rendered source.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::Dependency;
use crate::error::{ErrorKind, Result};
use crate::growth::Group;

const CARGO_TOML: &str = "Cargo.toml";
const BENCH_ROOT: &str = "benches";
const CRATE_ROOT: &str = "crates";
const CASE_ROOT: &str = "cases";
const TARGET_ROOT: &str = "target";
const SERIES_ROOT: &str = "ph_bench_build";
const USER_SUFFIX: &str = "_user";
const EXPAND_SUFFIX: &str = "-expand";

/// One workspace member package.
struct Member {
    name: String,
    directory: PathBuf,
}

/// The workspace that contains the running test or bench.
pub(crate) struct Workspace {
    root: PathBuf,
    members: Vec<Member>,
}

/// Paths [`crate::Bench`] fills in when the caller does not.
pub(crate) struct BenchLayout {
    pub(crate) cases: PathBuf,
    pub(crate) dependency: Dependency,
    pub(crate) package: String,
    pub(crate) output: String,
}

/// Where one [`crate::Series`] writes its user crate.
pub(crate) struct SeriesHome {
    pub(crate) output: PathBuf,
    pub(crate) package: String,
    pub(crate) dependencies: Vec<Dependency>,
}

impl Workspace {
    fn member(&self, name: &str) -> Option<&Member> {
        self.members.iter().find(|member| member.name == name)
    }

    fn require(&self, name: &str) -> Result<&Member> {
        self.member(name)
            .ok_or_else(|| ErrorKind::MemberMissing { package: name.to_owned() }.into())
    }
}

/// The workspace that owns the crate currently being compiled.
///
/// # Errors
///
/// Returns an error when `CARGO_MANIFEST_DIR` is unset or no ancestor manifest
/// lists workspace members.
pub(crate) fn discover_from_manifest_dir() -> Result<Workspace> {
    let start = env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .ok_or(ErrorKind::MissingManifestDir)?;
    discover(&start)
}

/// Walks ancestors of `start` until a manifest lists workspace members.
///
/// # Errors
///
/// Returns an error when no ancestor is a workspace, or a member manifest
/// cannot be read.
pub(crate) fn discover(start: &Path) -> Result<Workspace> {
    let mut dir = start.to_path_buf();
    loop {
        let manifest_path = dir.join(CARGO_TOML);
        if manifest_path.is_file() {
            let parsed = read_manifest::<RootManifest>(&manifest_path)?;
            if let Some(members) = parsed.workspace.and_then(|workspace| workspace.members) {
                return load_members(dir, &members);
            }
        }
        if !dir.pop() {
            break;
        }
    }
    Err(ErrorKind::WorkspaceMissing { start: start.display().to_string() }.into())
}

/// Case directory, path dependency, user package, and output name for `name`.
///
/// # Errors
///
/// Returns an error when `name` is not a workspace member.
pub(crate) fn bench_layout(name: &str) -> Result<BenchLayout> {
    let workspace = discover_from_manifest_dir()?;
    let member = workspace.require(name)?;
    let cases = workspace
        .root
        .join(BENCH_ROOT)
        .join(CRATE_ROOT)
        .join(name)
        .join(CASE_ROOT);
    Ok(BenchLayout {
        cases,
        dependency: Dependency::path(member.name.clone(), member.directory.clone()),
        package: format!("{name}{USER_SUFFIX}"),
        output: format!("{name}{EXPAND_SUFFIX}"),
    })
}

/// Output directory, user package, and path dependencies for one series.
///
/// `use` paths in `sources` that name a workspace member become path
/// dependencies. `std`, `core`, `alloc`, and `proc_macro` do not.
///
/// # Errors
///
/// Returns an error when a used crate is not a workspace member.
pub(crate) fn series_home(name: &str, group: Group, sources: &[String]) -> Result<SeriesHome> {
    let workspace = discover_from_manifest_dir()?;
    let base = env::var_os("CARGO_TARGET_DIR")
        .map_or_else(|| workspace.root.join(TARGET_ROOT), PathBuf::from);
    let mut names = Vec::new();
    for source in sources {
        for crate_name in crate_uses(source) {
            if !names.iter().any(|seen: &String| seen == &crate_name) {
                names.push(crate_name);
            }
        }
    }
    let mut dependencies = Vec::with_capacity(names.len());
    for crate_name in names {
        let Some(member) = workspace.member(&crate_name) else {
            return Err(ErrorKind::SeriesCrate { series: name.to_owned(), crate_name }.into());
        };
        dependencies.push(Dependency::path(
            member.name.clone(),
            member.directory.clone(),
        ));
    }
    Ok(SeriesHome {
        output: base.join(SERIES_ROOT).join(format!("{name}-{group}")),
        package: format!("{name}{USER_SUFFIX}"),
        dependencies,
    })
}

fn load_members(root: PathBuf, members: &[String]) -> Result<Workspace> {
    let mut found = Vec::with_capacity(members.len());
    for member in members {
        let directory = root.join(member);
        let manifest_path = directory.join(CARGO_TOML);
        let parsed = read_manifest::<MemberManifest>(&manifest_path)?;
        let Some(package) = parsed.package else {
            continue;
        };
        found.push(Member { name: package.name, directory });
    }
    Ok(Workspace { root, members: found })
}

fn read_manifest<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let text = fs::read_to_string(path)?;
    toml::from_str(&text).map_err(|source| {
        ErrorKind::ManifestRead { path: path.display().to_string(), source }.into()
    })
}

/// Crate names in `use` items, skipping the standard prelude crates.
fn crate_uses(source: &str) -> Vec<String> {
    let code = code_only(source);
    let bytes = code.as_bytes();
    let mut names = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if !is_ident_start(bytes[index]) {
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        while index < bytes.len() && is_ident_continue(bytes[index]) {
            index += 1;
        }
        if &code[start..index] != "use" {
            continue;
        }
        let Some(name) = following_ident(&code[index..]) else {
            continue;
        };
        if is_builtin(&name) || names.iter().any(|seen| seen == &name) {
            continue;
        }
        names.push(name);
    }
    names
}

fn following_ident(text: &str) -> Option<String> {
    let mut rest = text.trim_start();
    if let Some(stripped) = rest.strip_prefix("::") {
        rest = stripped.trim_start();
    }
    let mut chars = rest.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphabetic() && first != '_' {
        return None;
    }
    let mut name = String::new();
    name.push(first);
    for ch in chars {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            name.push(ch);
        } else {
            break;
        }
    }
    Some(name)
}

fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "alloc" | "core" | "crate" | "proc_macro" | "self" | "std" | "super" | "test"
    )
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_ident_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Source with comments and string literals replaced by spaces.
fn code_only(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '/' if chars.peek() == Some(&'/') => {
                chars.next();
                out.push_str("  ");
                for next in chars.by_ref() {
                    if next == '\n' {
                        out.push('\n');
                        break;
                    }
                    push_space(&mut out, next);
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                out.push_str("  ");
                skip_block(&mut chars, &mut out);
            }
            '"' => {
                out.push(' ');
                skip_string(&mut chars, &mut out);
            }
            'r' if matches!(chars.peek(), Some('#' | '"')) => {
                out.push(' ');
                skip_raw(&mut chars, &mut out);
            }
            '\'' => {
                out.push(' ');
                skip_char_or_lifetime(&mut chars, &mut out);
            }
            _ => out.push(ch),
        }
    }
    out
}

fn skip_block(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, out: &mut String) {
    let mut depth = 1u32;
    while let Some(ch) = chars.next() {
        match ch {
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                depth += 1;
                out.push_str("  ");
            }
            '*' if chars.peek() == Some(&'/') => {
                chars.next();
                depth -= 1;
                out.push_str("  ");
                if depth == 0 {
                    break;
                }
            }
            _ => push_space(out, ch),
        }
    }
}

fn skip_string(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, out: &mut String) {
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            out.push(' ');
            if let Some(escaped) = chars.next() {
                push_space(out, escaped);
            }
            continue;
        }
        push_space(out, ch);
        if ch == '"' {
            break;
        }
    }
}

fn skip_raw(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, out: &mut String) {
    let mut hashes = 0usize;
    while chars.peek() == Some(&'#') {
        chars.next();
        hashes += 1;
        out.push(' ');
    }
    if chars.peek() != Some(&'"') {
        return;
    }
    chars.next();
    out.push(' ');
    while let Some(ch) = chars.next() {
        if ch == '"' {
            let mut matched = 0usize;
            while matched < hashes && chars.peek() == Some(&'#') {
                chars.next();
                matched += 1;
            }
            push_space(out, ch);
            for _ in 0..matched {
                out.push(' ');
            }
            if matched == hashes {
                break;
            }
            continue;
        }
        push_space(out, ch);
    }
}

fn skip_char_or_lifetime(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, out: &mut String) {
    let Some(first) = chars.next() else {
        return;
    };
    if first == '\\' {
        push_space(out, first);
        if let Some(escaped) = chars.next() {
            push_space(out, escaped);
        }
        if chars.peek() == Some(&'\'') {
            chars.next();
            out.push(' ');
        }
        return;
    }
    push_space(out, first);
    if chars.peek() == Some(&'\'') {
        chars.next();
        out.push(' ');
    }
}

fn push_space(out: &mut String, ch: char) {
    if ch == '\n' {
        out.push('\n');
    } else {
        for _ in 0..ch.len_utf8() {
            out.push(' ');
        }
    }
}

#[derive(Deserialize)]
struct RootManifest {
    workspace: Option<WorkspaceTable>,
}

#[derive(Deserialize)]
struct WorkspaceTable {
    members: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct MemberManifest {
    package: Option<PackageName>,
}

#[derive(Deserialize)]
struct PackageName {
    name: String,
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{crate_uses, discover, discover_from_manifest_dir};

    #[test]
    fn use_paths_skip_builtins_comments_and_strings() {
        let source = r#"
            use ph_util_foreach::impl_foreach;
            use std::fmt::Write;
            // use hidden::nope;
            const TEXT: &str = "use string_crate::Item";
            /* use block::nope; */
        "#;
        assert_eq!(crate_uses(source), ["ph_util_foreach"]);
    }

    #[test]
    fn this_workspace_contains_the_foreach_crate() {
        let workspace = discover_from_manifest_dir().unwrap();
        let member = workspace.member("ph_util_foreach").unwrap();
        assert!(member.directory.join("Cargo.toml").is_file());
        assert!(workspace.member("missing_crate").is_none());
    }

    #[test]
    fn a_temp_workspace_is_found_from_a_member() {
        let tree = Tree::new();
        fs::write(
            tree.0.join("Cargo.toml"),
            "[workspace]\nmembers = [\"demo\"]\n",
        )
        .unwrap();
        fs::create_dir_all(tree.0.join("demo")).unwrap();
        fs::write(
            tree.0.join("demo/Cargo.toml"),
            "[package]\nname = \"demo_macro\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let workspace = discover(&tree.0.join("demo")).unwrap();
        assert_eq!(
            workspace.member("demo_macro").unwrap().directory,
            tree.0.join("demo")
        );
    }

    struct Tree(PathBuf);

    impl Tree {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("ph_bench_ws_{}_{id}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}
