//! Release `rustc` invocation that emits assembly and LLVM IR.
//!
//! Assembly feeds the asserts. LLVM IR is read only for `noalias` on
//! function parameters.
//!
//! These asserts compile the probe in this crate.

use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::assembly::Assembly;
use crate::error::{ErrorKind, Result};

const EDITION: &str = "2024";
const CRATE_NAME: &str = "probe";
const EMIT_DIR_PREFIX: &str = "ph-test-asm-emit";

pub(crate) fn compile(source: &Path, target: Option<&str>) -> Result<Assembly> {
    let dir = scratch_dir()?;
    let _guard = TempDir(dir.clone());
    let mut command = Command::new(rustc_program());
    command.arg("--edition").arg(EDITION);
    command.arg("--crate-type").arg("lib");
    command.arg("--crate-name").arg(CRATE_NAME);
    command.arg("-C").arg("opt-level=3");
    command.arg("-C").arg("debuginfo=0");
    command.arg("-C").arg("debug-assertions=off");
    command.arg("-C").arg("overflow-checks=off");
    command.arg("-C").arg("codegen-units=1");
    command.arg("--emit=llvm-ir,asm");
    command.arg("--out-dir").arg(&dir);
    if let Some(triple) = target {
        command.arg("--target").arg(triple);
    }
    command.arg(source);
    let output = command
        .output()
        .map_err(|source| ErrorKind::RustcStart { source })?;
    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr).map_err(|source| ErrorKind::Utf8 {
            what: "rustc stderr",
            source,
        })?;
        return Err(ErrorKind::RustcFailed {
            path: source.display().to_string(),
            stderr,
        }
        .into());
    }
    let ir = read_emit(&dir, "ll", "LLVM IR")?;
    let asm = read_emit(&dir, "s", "assembly")?;
    Ok(Assembly::parse(&ir, &asm))
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
        total = total.saturating_add(len).saturating_add(1);
    }
    let mut text = String::with_capacity(total);
    for path in paths {
        fs::File::open(path)?.read_to_string(&mut text)?;
        text.push('\n');
    }
    Ok(text)
}
