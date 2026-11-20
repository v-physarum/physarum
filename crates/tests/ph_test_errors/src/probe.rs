//! `rustc` invocation for one debug probe.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::diagnostics::{self, Diagnostics};
use crate::error::{ErrorKind, Result};

const EDITION: &str = "2024";
const CRATE_NAME: &str = "probe";

/// One Rust probe built for compiler diagnostics.
///
/// [`Probe::diagnostics`] runs `rustc` at `opt-level=0` and reads JSON
/// diagnostics. It does not emit LLVM IR or assembly.
#[must_use]
#[derive(Debug)]
pub struct Probe {
    source: PathBuf,
}

impl Probe {
    /// A probe read from `source`.
    pub fn new(source: impl Into<PathBuf>) -> Self {
        Self {
            source: source.into(),
        }
    }

    /// Builds the probe and returns the compiler diagnostics.
    ///
    /// An error diagnostic is part of the returned value. [`Err`] means `rustc`
    /// did not start, or it failed without a JSON diagnostic.
    ///
    /// # Errors
    ///
    /// Returns an error when `rustc` cannot start, its stderr is not UTF-8,
    /// a diagnostic line is not JSON, or `rustc` fails without a diagnostic.
    pub fn diagnostics(self) -> Result<Diagnostics> {
        let dir = scratch_dir()?;
        let _guard = TempDir(dir.clone());
        let mut command = Command::new(rustc_program());
        command.arg("--edition").arg(EDITION);
        command.arg("--crate-type").arg("lib");
        command.arg("--crate-name").arg(CRATE_NAME);
        command.arg("-C").arg("opt-level=0");
        command.arg("--error-format").arg("json");
        command.arg("--color").arg("never");
        command.arg("--emit=metadata");
        command.arg("--out-dir").arg(&dir);
        command.arg(&self.source);
        let output = command
            .output()
            .map_err(|source| ErrorKind::RustcStart { source })?;
        let stderr = String::from_utf8(output.stderr).map_err(|source| ErrorKind::Utf8 {
            what: "rustc stderr",
            source,
        })?;
        let parsed = diagnostics::parse(output.status.success(), &stderr)?;
        if !output.status.success() && parsed.is_empty() {
            return Err(ErrorKind::ProbeFailed {
                path: self.source.display().to_string(),
                stderr,
            }
            .into());
        }
        Ok(parsed)
    }
}

fn rustc_program() -> String {
    env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned())
}

fn scratch_dir() -> Result<PathBuf> {
    static TICK: AtomicU64 = AtomicU64::new(0);
    let n = TICK.fetch_add(1, Ordering::Relaxed);
    let dir = env::temp_dir().join(format!("ph-test-errors-out-{}-{n}", std::process::id()));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
