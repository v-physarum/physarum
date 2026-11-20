//! [`ErrorTester`] checks one debug probe.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::diagnostics::{Diagnostic, Diagnostics, Level};
use crate::probe::Probe;

/// Compile-fail checks for one debug probe.
///
/// ```no_run
/// use ph_test_errors::ErrorTester;
///
/// ErrorTester::file("fail.rs")
///     .assert_code("E0308")
///     .assert_description("mismatched types");
/// ErrorTester::source("fn main() { missing; }")
///     .assert_code("E0425")
///     .assert_description("cannot find");
/// ErrorTester::file("ok.rs").assert_pass();
/// ```
///
/// The chain compiles the probe once. Later asserts reuse that result.
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

    /// The probe emits error `code`.
    ///
    /// # Panics
    ///
    /// Panics when the code is absent, or when the probe cannot be built.
    pub fn assert_code(mut self, code: &str) -> Self {
        let ok = self.view().error_codes().any(|found| found == code);
        if !ok {
            self.mismatch(&format!("error {code}"));
        }
        self
    }

    /// An error message contains `text`.
    ///
    /// # Panics
    ///
    /// Panics when no error message contains `text`, or when the probe cannot be built.
    pub fn assert_description(mut self, text: &str) -> Self {
        let ok = self
            .view()
            .error_messages()
            .any(|message| message.contains(text));
        if !ok {
            self.mismatch(&format!("description {text}"));
        }
        self
    }

    /// One error has `code` and its message contains `text`.
    ///
    /// # Panics
    ///
    /// Panics when no error matches both, or when the probe cannot be built.
    pub fn assert_code_description(mut self, code: &str, text: &str) -> Self {
        let ok = errors(self.view())
            .any(|item| item.code() == Some(code) && item.message().contains(text));
        if !ok {
            self.mismatch(&format!("error {code} description {text}"));
        }
        self
    }

    /// The probe builds and emits no errors. Warnings are allowed.
    ///
    /// # Panics
    ///
    /// Panics when the probe fails to build or emits an error.
    pub fn assert_pass(mut self) -> Self {
        let ok = self.view().succeeded();
        if !ok {
            self.mismatch("probe compiles");
        }
        self
    }

    /// The probe emits an error or fails to build.
    ///
    /// # Panics
    ///
    /// Panics when the probe builds without errors.
    pub fn assert_fail(mut self) -> Self {
        let ok = !self.view().succeeded();
        if !ok {
            self.mismatch("probe fails");
        }
        self
    }

    /// A note contains `text`.
    ///
    /// # Panics
    ///
    /// Panics when no note contains `text`, or when the probe cannot be built.
    pub fn assert_note(mut self, text: &str) -> Self {
        let ok = any(self.view(), |item| {
            item.level() == Level::Note && item.message().contains(text)
        });
        if !ok {
            self.mismatch(&format!("note {text}"));
        }
        self
    }

    /// A help message contains `text`.
    ///
    /// # Panics
    ///
    /// Panics when no help message contains `text`, or when the probe cannot be built.
    pub fn assert_help(mut self, text: &str) -> Self {
        let ok = any(self.view(), |item| {
            item.level() == Level::Help && item.message().contains(text)
        });
        if !ok {
            self.mismatch(&format!("help {text}"));
        }
        self
    }

    /// The probe emits no warnings.
    ///
    /// # Panics
    ///
    /// Panics when a warning is present, or when the probe cannot be built.
    pub fn assert_no_warning(mut self) -> Self {
        let ok = !any(self.view(), |item| item.level() == Level::Warning);
        if !ok {
            self.mismatch("no warnings");
        }
        self
    }

    /// A warning has `code`.
    ///
    /// # Panics
    ///
    /// Panics when the warning code is absent, or when the probe cannot be built.
    pub fn assert_warning_code(mut self, code: &str) -> Self {
        let ok = any(self.view(), |item| {
            item.level() == Level::Warning && item.code() == Some(code)
        });
        if !ok {
            self.mismatch(&format!("warning {code}"));
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
        match self.cached.as_ref() {
            Some(diagnostics) => diagnostics,
            None => panic!("debug probe was not built"),
        }
    }

    fn mismatch(mut self, expectation: &str) -> ! {
        let detail = render(self.view());
        panic!("assertion failed: {expectation}\n{detail}");
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

fn errors(diagnostics: &Diagnostics) -> impl Iterator<Item = &Diagnostic> {
    diagnostics
        .messages()
        .iter()
        .filter(|item| item.level() == Level::Error)
}

fn any(diagnostics: &Diagnostics, mut pred: impl FnMut(&Diagnostic) -> bool) -> bool {
    fn walk(items: &[Diagnostic], pred: &mut impl FnMut(&Diagnostic) -> bool) -> bool {
        for item in items {
            if pred(item) || walk(item.children(), pred) {
                return true;
            }
        }
        false
    }
    walk(diagnostics.messages(), &mut pred)
}

fn render(diagnostics: &Diagnostics) -> String {
    let mut out = String::new();
    for item in diagnostics.messages() {
        let text = if item.rendered().is_empty() {
            item.message()
        } else {
            item.rendered()
        };
        if text.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(text);
    }
    if out.is_empty() {
        "(no diagnostics)".to_owned()
    } else {
        out
    }
}
