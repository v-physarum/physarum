//! `rustc` invocation for one release probe.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{ErrorKind, Result};
use crate::ir::Assembly;

const EDITION: &str = "2024";
const CRATE_NAME: &str = "probe";
const OPT_LEVEL: &str = "3";
const DEBUGINFO: &str = "0";
const CODEGEN_UNITS: &str = "1";

/// One Rust probe compiled to LLVM IR and assembly.
///
/// [`Probe::emit`] runs only after [`Probe::release`]. There is no debug profile.
#[must_use]
#[derive(Debug)]
pub struct Probe {
    source: PathBuf,
    target: Option<String>,
    released: bool,
}

impl Probe {
    /// A probe read from `source`.
    pub fn new(source: impl Into<PathBuf>) -> Self {
        Self {
            source: source.into(),
            target: None,
            released: false,
        }
    }

    /// Compiles the probe as release: `opt-level=3`, no debug info.
    pub fn release(mut self) -> Self {
        self.released = true;
        self
    }

    /// Passes `--target` to `rustc`.
    pub fn target(mut self, triple: impl Into<String>) -> Self {
        self.target = Some(triple.into());
        self
    }

    /// Emits LLVM IR and assembly.
    ///
    /// # Errors
    ///
    /// Returns an error when [`Probe::release`] was skipped, `rustc` fails,
    /// or the output cannot be read.
    pub fn emit(self) -> Result<Assembly> {
        let Self {
            source,
            target,
            released,
        } = self;
        if !released {
            return Err(ErrorKind::NotRelease.into());
        }
        let request = EmitRequest {
            source: &source,
            target: target.as_deref(),
        };
        let emitted = request.run()?;
        Ok(Assembly::parse(emitted.ir, emitted.asm))
    }
}

struct EmitRequest<'a> {
    source: &'a Path,
    target: Option<&'a str>,
}

struct Emitted {
    ir: String,
    asm: String,
}

impl EmitRequest<'_> {
    fn run(self) -> Result<Emitted> {
        let dir = scratch_dir()?;
        let _guard = TempDir(dir.clone());
        let mut command = Command::new(rustc_program());
        command.arg("--edition").arg(EDITION);
        command.arg("--crate-type").arg("lib");
        command.arg("--crate-name").arg(CRATE_NAME);
        command.arg("-C").arg(format!("opt-level={OPT_LEVEL}"));
        command.arg("-C").arg(format!("debuginfo={DEBUGINFO}"));
        command.arg("-C").arg("debug-assertions=off");
        command.arg("-C").arg("overflow-checks=off");
        command
            .arg("-C")
            .arg(format!("codegen-units={CODEGEN_UNITS}"));
        command.arg("--emit=llvm-ir,asm");
        command.arg("--out-dir").arg(&dir);
        if let Some(target) = self.target {
            command.arg("--target").arg(target);
        }
        command.arg(self.source);
        let output = command
            .output()
            .map_err(|source| ErrorKind::RustcStart { source })?;
        if !output.status.success() {
            let stderr = String::from_utf8(output.stderr).map_err(|source| ErrorKind::Utf8 {
                what: "rustc stderr",
                source,
            })?;
            return Err(ErrorKind::RustcFailed {
                path: self.source.display().to_string(),
                stderr,
            }
            .into());
        }
        Ok(Emitted {
            ir: read_emit(&dir, "ll", "LLVM IR")?,
            asm: read_emit(&dir, "s", "assembly")?,
        })
    }
}

fn rustc_program() -> String {
    env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned())
}

fn scratch_dir() -> Result<PathBuf> {
    static TICK: AtomicU64 = AtomicU64::new(0);
    let n = TICK.fetch_add(1, Ordering::Relaxed);
    let dir = env::temp_dir().join(format!("ph-build-test-{}-{n}", std::process::id()));
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
    let mut text = String::new();
    for path in paths {
        text.push_str(&fs::read_to_string(path)?);
        text.push('\n');
    }
    Ok(text)
}
