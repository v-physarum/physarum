//! `rustc` invocation for one debug probe.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::diagnostics::Diagnostics;
use crate::error::{Error, Result};

const EDITION: &str = "2024";
const CRATE_NAME: &str = "probe";
const OUT_DIR_PREFIX: &str = "ph-test-errors-out";
const DIR_NAME_SEPARATOR: &str = "-";
const OPT_LEVEL_PREFIX: &str = "opt-level=";
const OPT_LEVEL: &str = "0";
const EXTERN_SEPARATOR: &str = "=";
const RUSTC_PROGRAM: &str = "rustc";
const START_TICK: u64 = 0;
const TICK_STEP: u64 = 1;

/// One `--extern name=path` passed to `rustc`.
#[derive(Debug)]
pub(crate) struct Extern {
    name: String,
    path: PathBuf,
}

/// One Rust probe built for compiler diagnostics.
///
/// [`Probe::diagnostics`] runs `rustc` at `opt-level=0` and reads JSON
/// diagnostics. Each [`Extern`] is `--extern name=path`.
#[must_use]
#[derive(Debug)]
pub(crate) struct Probe {
    source: PathBuf,
}

impl Extern {
    /// An extern crate named `name` whose library file is `path`.
    pub(crate) fn new(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self { name: name.into(), path: path.into() }
    }

    fn arg(&self) -> OsString {
        let mut arg = OsString::with_capacity(
            self.name.len() + EXTERN_SEPARATOR.len() + self.path.as_os_str().len(),
        );
        arg.push(&self.name);
        arg.push(EXTERN_SEPARATOR);
        arg.push(&self.path);
        arg
    }
}

impl Probe {
    /// A probe read from `source`.
    pub(crate) fn new(source: impl Into<PathBuf>) -> Self {
        Self { source: source.into() }
    }

    /// Builds the probe and returns the compiler diagnostics.
    ///
    /// An error diagnostic is part of the returned value. [`Err`] means `rustc`
    /// did not start, or it failed without a JSON diagnostic. `externs` are
    /// passed as `--extern name=path`.
    ///
    /// # Errors
    ///
    /// Returns an error when `rustc` cannot start, its stderr is not UTF-8,
    /// a diagnostic line is not JSON, or `rustc` fails without a diagnostic.
    pub(crate) fn diagnostics(self, externs: &[Extern]) -> Result<Diagnostics> {
        let dir = TempDir::scratch()?;
        let mut command = Command::new(Self::rustc_program());
        command.arg("--edition").arg(EDITION);
        command.arg("--crate-type").arg("lib");
        command.arg("--crate-name").arg(CRATE_NAME);
        command
            .arg("-C")
            .arg(format!("{OPT_LEVEL_PREFIX}{OPT_LEVEL}"));
        command.arg("--error-format").arg("json");
        command.arg("--color").arg("never");
        command.arg("--emit=metadata");
        command.arg("--out-dir").arg(&dir.0);
        for item in externs {
            command.arg("--extern").arg(item.arg());
        }
        command.arg(&self.source);
        let output = command
            .output()
            .map_err(|source| Error::RustcStart { source })?;
        let stderr = String::from_utf8(output.stderr).map_err(|source| Error::Utf8 { source })?;
        let parsed = Diagnostics::parse(&stderr)?;
        if !output.status.success() && parsed.messages.is_empty() {
            return Err(Error::ProbeFailed { path: self.source.display().to_string(), stderr });
        }
        Ok(parsed)
    }

    fn rustc_program() -> String {
        env::var("RUSTC").unwrap_or_else(|_| RUSTC_PROGRAM.to_owned())
    }
}

struct TempDir(PathBuf);

impl TempDir {
    fn scratch() -> Result<Self> {
        static TICK: AtomicU64 = AtomicU64::new(START_TICK);
        let n = TICK.fetch_add(TICK_STEP, Ordering::Relaxed);
        let id = std::process::id();
        let dir = env::temp_dir().join(format!(
            "{OUT_DIR_PREFIX}{DIR_NAME_SEPARATOR}{id}{DIR_NAME_SEPARATOR}{n}"
        ));
        fs::create_dir_all(&dir)?;
        Ok(Self(dir))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
