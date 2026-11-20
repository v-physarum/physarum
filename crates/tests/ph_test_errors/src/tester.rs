//! [`ErrorTester`] checks one debug probe.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::diagnostics::{Diagnostic, Diagnostics};
use crate::probe::{Extern, Probe};

/// Checks that a debug probe emits an error code or a message.
///
/// ```no_run
/// use ph_test_errors::ErrorTester;
///
/// ErrorTester::file("fail.rs").assert_code("E0308");
/// ErrorTester::source("fn main() { missing; }").assert_description("cannot find");
/// ErrorTester::source("compile_error!(\"stop\");")
///     .extern_crate("demo", "libdemo.rlib")
///     .assert_description("stop");
/// ```
///
/// The check compiles the probe once. A match does not allocate. A mismatch
/// panics with the codes and messages the probe emitted.
#[derive(Debug)]
pub struct ErrorTester {
    input: Input,
    externs: Vec<Extern>,
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
            externs: Vec::new(),
            cached: None,
        }
    }

    /// A probe whose source is `text`. `rustc` sees it as `probe.rs`.
    pub fn source(text: impl Into<String>) -> Self {
        Self {
            input: Input::Text(text.into()),
            externs: Vec::new(),
            cached: None,
        }
    }

    /// Passes `--extern name=path` to `rustc` for this probe.
    ///
    /// Call again for another crate. Set every extern before the first assert.
    /// The check compiles the probe once and reuses those diagnostics.
    pub fn extern_crate(mut self, name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        self.cached = None;
        self.externs.push(Extern::new(name, path));
        self
    }

    /// One error has `code`.
    ///
    /// # Panics
    ///
    /// Panics when no error has `code`, or when the probe cannot be built.
    /// The panic names `code` and the codes and messages that arrived.
    pub fn assert_code(mut self, code: &str) -> Self {
        let ok = self
            .view()
            .messages
            .iter()
            .any(|item| item.matches_code(code));
        if !ok {
            let header = format!("error {code}");
            self.fail(&header);
        }
        self
    }

    /// One error message contains `description`.
    ///
    /// A diagnostic whose code is null matches on its message. `compile_error!`
    /// is that case.
    ///
    /// # Panics
    ///
    /// Panics when no error message contains `description`, or when the probe
    /// cannot be built. The panic names `description` and the codes and
    /// messages that arrived.
    pub fn assert_description(mut self, description: &str) -> Self {
        let ok = self
            .view()
            .messages
            .iter()
            .any(|item| item.matches_message(description));
        if !ok {
            let header = format!("description {description}");
            self.fail(&header);
        }
        self
    }

    fn view(&mut self) -> &Diagnostics {
        if self.cached.is_none() {
            let (path, _guard) = self.input.materialize();
            let diagnostics = Probe::new(path)
                .diagnostics(&self.externs)
                .unwrap_or_else(|error| {
                    panic!("{error}");
                });
            self.cached = Some(diagnostics);
        }
        self.cached.as_ref().expect("debug probe was not built")
    }

    fn fail(mut self, header: &str) -> ! {
        let messages = &self.view().messages;
        let mut arrived = String::with_capacity(Diagnostic::arrived_len(messages));
        for (index, item) in messages.iter().enumerate() {
            if index > 0 {
                arrived.push('\n');
            }
            item.push_line(&mut arrived);
        }
        if arrived.is_empty() {
            arrived.push_str(Diagnostic::NONE);
        }
        panic!("assertion failed: {header}\n{arrived}");
    }
}

impl Input {
    fn materialize(&self) -> (PathBuf, Option<TempDir>) {
        match self {
            Self::File(path) => (path.clone(), None),
            Self::Text(text) => {
                let dir = TempDir::scratch();
                let path = dir.0.join("probe.rs");
                fs::write(&path, text).unwrap_or_else(|error| {
                    panic!("failed to write probe: {error}");
                });
                (path, Some(dir))
            }
        }
    }
}

const SCRATCH_DIR_PREFIX: &str = "ph-test-errors";

struct TempDir(PathBuf);

impl TempDir {
    fn scratch() -> Self {
        static TICK: AtomicU64 = AtomicU64::new(0);
        let n = TICK.fetch_add(1, Ordering::Relaxed);
        let dir = env::temp_dir().join(format!("{SCRATCH_DIR_PREFIX}-{}-{n}", std::process::id()));
        fs::create_dir_all(&dir).unwrap_or_else(|error| {
            panic!("failed to create probe dir: {error}");
        });
        Self(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
