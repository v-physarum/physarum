//! [`ErrorTester`] checks one debug probe.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::diagnostics::Diagnostics;
use crate::probe::Probe;

/// Checks that a debug probe emits an error code and a description.
///
/// ```no_run
/// use ph_test_errors::ErrorTester;
///
/// ErrorTester::file("fail.rs").assert_code_description("E0308", "mismatched types");
/// ErrorTester::source("fn main() { missing; }").assert_code_description("E0425", "cannot find");
/// ```
///
/// The check compiles the probe once. A match does not allocate. A mismatch
/// panics with the codes and messages the probe emitted.
#[derive(Debug)]
pub struct ErrorTester {
    input: Input,
    cached: Option<Diagnostics>,
}

#[derive(Debug)]
enum Input {
    File(PathBuf),
    Text(String),
}

#[allow(clippy::must_use_candidate, clippy::return_self_not_must_use)]
impl ErrorTester {
    /// A probe read from `path`.
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self {
            input: Input::File(path.into()),
            cached: None,
        }
    }

    /// A probe whose source is `text`. `rustc` sees it as `probe.rs`.
    pub fn source(text: impl Into<String>) -> Self {
        Self {
            input: Input::Text(text.into()),
            cached: None,
        }
    }

    /// One error has `code` and its message contains `description`.
    ///
    /// # Panics
    ///
    /// Panics when no error matches both, or when the probe cannot be built.
    /// The panic names `code`, `description`, and the codes and messages that
    /// arrived.
    pub fn assert_code_description(mut self, code: &str, description: &str) -> Self {
        let ok =
            self.view().messages.iter().any(|item| {
                item.code.as_deref() == Some(code) && item.message.contains(description)
            });
        if !ok {
            self.mismatch(code, description);
        }
        self
    }

    fn view(&mut self) -> &Diagnostics {
        if self.cached.is_none() {
            let (path, _guard) = materialize(&self.input);
            let diagnostics = Probe::new(path).diagnostics().unwrap_or_else(|error| {
                panic!("{error}");
            });
            self.cached = Some(diagnostics);
        }
        self.cached.as_ref().expect("debug probe was not built")
    }

    fn mismatch(mut self, code: &str, description: &str) -> ! {
        let mut arrived = String::new();
        for item in &self.view().messages {
            if !arrived.is_empty() {
                arrived.push('\n');
            }
            arrived.push_str(item.code.as_deref().unwrap_or("(none)"));
            arrived.push_str(": ");
            arrived.push_str(&item.message);
        }
        if arrived.is_empty() {
            arrived.push_str("(none)");
        }
        panic!("assertion failed: error {code} description {description}\n{arrived}");
    }
}

fn materialize(input: &Input) -> (PathBuf, Option<TempDir>) {
    match input {
        Input::File(path) => (path.clone(), None),
        Input::Text(text) => {
            let dir = scratch_dir();
            let path = dir.join("probe.rs");
            fs::write(&path, text).unwrap_or_else(|error| {
                panic!("failed to write probe: {error}");
            });
            (path, Some(TempDir(dir)))
        }
    }
}

fn scratch_dir() -> PathBuf {
    static TICK: AtomicU64 = AtomicU64::new(0);
    let n = TICK.fetch_add(1, Ordering::Relaxed);
    let dir = env::temp_dir().join(format!("ph-test-errors-{}-{n}", std::process::id()));
    fs::create_dir_all(&dir).unwrap_or_else(|error| {
        panic!("failed to create probe dir: {error}");
    });
    dir
}

struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
