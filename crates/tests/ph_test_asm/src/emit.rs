//! Release `rustc` invocation that emits assembly and LLVM IR.
//!
//! Assembly feeds the asserts. LLVM IR is read only for `noalias` on
//! function parameters.
//!
//! These asserts compile the probe in this crate.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::assembly::Assembly;
use crate::error::{ErrorKind, Result};
use crate::target::Target;

const EDITION: &str = "2024";
const CRATE_NAME: &str = "probe";
const CRATE_TYPE: &str = "lib";
const EMIT_DIR_PREFIX: &str = "ph-test-asm-emit";
const EMIT_FLAG: &str = "--emit=llvm-ir,asm";
const OPT_LEVEL: &str = "opt-level=3";
const DEBUGINFO: &str = "debuginfo=0";
const DEBUG_ASSERTIONS: &str = "debug-assertions=off";
const OVERFLOW_CHECKS: &str = "overflow-checks=off";
const CODEGEN_UNITS: &str = "codegen-units=1";
const EMIT_SEPARATOR_LEN: usize = 1;
const EXTERN_FLAG: &str = "--extern";
const EXTERN_SEPARATOR: &str = "=";
const SEARCH_FLAG: &str = "-L";
/// `-L` kind that only resolves crates other crates depend on.
const DEPENDENCY_SEARCH_PREFIX: &str = "dependency=";
const CURRENT_DIR: &str = ".";

/// One release probe: a Rust source file and the crates it links.
#[derive(Debug, Clone)]
pub(crate) struct Probe {
    source: PathBuf,
    externs: Vec<Extern>,
}

impl Probe {
    /// A probe read from `source` with no extern crate.
    pub(crate) fn new(source: PathBuf) -> Self {
        Self { source, externs: Vec::new() }
    }

    /// Adds `--extern name=path` and `-L dependency=<dir of path>`.
    pub(crate) fn push_extern(&mut self, item: Extern) {
        self.externs.push(item);
    }

    /// The probe file.
    #[must_use]
    pub(crate) fn source(&self) -> &Path {
        &self.source
    }

    /// Compiles the probe as release for `target`. `None` is the host.
    ///
    /// # Errors
    ///
    /// Returns an error when `rustc` cannot start, fails, or writes no output.
    pub(crate) fn compile(&self, target: Option<&str>) -> Result<Assembly> {
        compile(&self.source, &self.externs, target)
    }
}

/// One extern crate passed to `rustc` as `--extern name=path`.
///
/// The directory of `path` is also passed as `-L dependency=dir`, so the
/// crates that library depends on resolve from the same directory.
#[derive(Debug, Clone)]
pub(crate) struct Extern {
    name: String,
    path: PathBuf,
}

impl Extern {
    /// An extern crate named `name` whose library file is `path`.
    pub(crate) fn new(name: String, path: PathBuf) -> Self {
        Self { name, path }
    }

    fn extern_arg(&self) -> OsString {
        let mut arg = OsString::with_capacity(
            self.name.len() + EXTERN_SEPARATOR.len() + self.path.as_os_str().len(),
        );
        arg.push(&self.name);
        arg.push(EXTERN_SEPARATOR);
        arg.push(&self.path);
        arg
    }

    fn search_arg(&self) -> OsString {
        let dir = match self.path.parent() {
            Some(dir) if !dir.as_os_str().is_empty() => dir,
            _ => Path::new(CURRENT_DIR),
        };
        let mut arg =
            OsString::with_capacity(DEPENDENCY_SEARCH_PREFIX.len() + dir.as_os_str().len());
        arg.push(DEPENDENCY_SEARCH_PREFIX);
        arg.push(dir);
        arg
    }
}

fn compile(source: &Path, externs: &[Extern], target: Option<&str>) -> Result<Assembly> {
    let dir = scratch_dir()?;
    let _guard = TempDir(dir.clone());
    let mut command = Command::new(rustc_program());
    command.arg("--edition").arg(EDITION);
    command.arg("--crate-type").arg(CRATE_TYPE);
    command.arg("--crate-name").arg(CRATE_NAME);
    command.arg("-C").arg(OPT_LEVEL);
    command.arg("-C").arg(DEBUGINFO);
    command.arg("-C").arg(DEBUG_ASSERTIONS);
    command.arg("-C").arg(OVERFLOW_CHECKS);
    command.arg("-C").arg(CODEGEN_UNITS);
    command.arg(EMIT_FLAG);
    command.arg("--out-dir").arg(&dir);
    if let Some(triple) = target {
        command.arg("--target").arg(triple);
    }
    for item in externs {
        command.arg(EXTERN_FLAG).arg(item.extern_arg());
        command.arg(SEARCH_FLAG).arg(item.search_arg());
    }
    command.arg(source);
    let output = command
        .output()
        .map_err(|source| ErrorKind::RustcStart { source })?;
    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr)
            .map_err(|source| ErrorKind::Utf8 { what: "rustc stderr", source })?;
        return Err(ErrorKind::RustcFailed { path: source.display().to_string(), stderr }.into());
    }
    let ir = read_emit(&dir, "ll", "LLVM IR")?;
    let asm = read_emit(&dir, "s", "assembly")?;
    Ok(Assembly::parse(&ir, &asm, Target::for_triple(target)))
}

fn rustc_program() -> String {
    env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned())
}

fn scratch_dir() -> Result<PathBuf> {
    static TICK: AtomicU64 = AtomicU64::new(0);
    let n = TICK.fetch_add(1, Ordering::Relaxed);
    let dir = env::temp_dir().join(format!("{EMIT_DIR_PREFIX}-{}-{n}", std::process::id()));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn read_emit(dir: &Path, extension: &str, what: &'static str) -> Result<String> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some(extension) {
            paths.push(path);
        }
    }
    paths.sort();
    if paths.is_empty() {
        return Err(ErrorKind::EmptyEmit { what }.into());
    }
    let mut total = 0usize;
    for path in &paths {
        let len = usize::try_from(fs::metadata(path)?.len()).unwrap_or(usize::MAX);
        total = total.saturating_add(len).saturating_add(EMIT_SEPARATOR_LEN);
    }
    let mut text = String::with_capacity(total);
    for path in paths {
        fs::File::open(path)?.read_to_string(&mut text)?;
        text.push('\n');
    }
    Ok(text)
}
