//! Compare two release probes.

use std::path::{Path, PathBuf};

use crate::assembly::{Assembly, Function};
use crate::diff::unified;
use crate::emit::{Extern, Probe};
use crate::normalize::{function_text, loop_text, module_text};

/// Compares two probe files compiled as release.
#[must_use]
#[derive(Debug)]
pub struct AsmCompare {
    left: Probe,
    right: Probe,
}

impl AsmCompare {
    /// Both paths are Rust probes.
    pub fn files(left: impl Into<PathBuf>, right: impl Into<PathBuf>) -> Self {
        Self { left: Probe::new(left.into()), right: Probe::new(right.into()) }
    }

    /// Passes `--extern name=path` to `rustc` for both probes.
    ///
    /// The directory of `path` is passed as `-L dependency=dir`. Call again
    /// for another crate.
    pub fn extern_crate(mut self, name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        let item = Extern::new(name.into(), path.into());
        self.left.push_extern(item.clone());
        self.right.push_extern(item);
        self
    }

    /// The normalized modules are the same.
    ///
    /// # Panics
    ///
    /// Panics with a unified diff when the modules differ, or when a probe fails to compile.
    pub fn assert_equivalent(&self) {
        let left = emit(&self.left);
        let right = emit(&self.right);
        require_same(
            self.left.source(),
            self.right.source(),
            &module_text(&left),
            &module_text(&right),
        );
    }

    /// Narrows the comparison to one function.
    pub fn function(self, name: impl Into<String>) -> FunctionCompare {
        FunctionCompare { left: self.left, right: self.right, name: name.into() }
    }
}

/// One symbol in two probes.
#[must_use]
#[derive(Debug)]
pub struct FunctionCompare {
    left: Probe,
    right: Probe,
    name: String,
}

impl FunctionCompare {
    /// The normalized function text is the same.
    ///
    /// # Panics
    ///
    /// Panics with a unified diff when the functions differ, or when a probe fails to compile.
    pub fn assert_equivalent(&self) {
        let left = emit(&self.left);
        let right = emit(&self.right);
        let left_fn = function_or_panic(&left, &self.name);
        let right_fn = function_or_panic(&right, &self.name);
        require_same(
            self.left.source(),
            self.right.source(),
            &function_text(left_fn, left.target()),
            &function_text(right_fn, right.target()),
        );
    }

    /// Narrows the comparison to the hot loop.
    pub fn hot_loop(self) -> LoopCompare {
        LoopCompare { left: self.left, right: self.right, name: self.name }
    }
}

/// Hot loop of one symbol in two probes.
#[must_use]
#[derive(Debug)]
pub struct LoopCompare {
    left: Probe,
    right: Probe,
    name: String,
}

impl LoopCompare {
    /// The normalized hot loops are the same.
    ///
    /// # Panics
    ///
    /// Panics with a unified diff when the loops differ, or when a probe has no hot loop.
    pub fn assert_equivalent(&self) {
        let left = emit(&self.left);
        let right = emit(&self.right);
        let left_fn = function_or_panic(&left, &self.name);
        let right_fn = function_or_panic(&right, &self.name);
        let left_loop = left_fn.hot_loop().unwrap_or_else(|error| panic!("{error}"));
        let right_loop = right_fn
            .hot_loop()
            .unwrap_or_else(|error| panic!("{error}"));
        require_same(
            self.left.source(),
            self.right.source(),
            &loop_text(left_loop, left.target()),
            &loop_text(right_loop, right.target()),
        );
    }
}

fn emit(probe: &Probe) -> Assembly {
    probe
        .compile(None)
        .unwrap_or_else(|error| panic!("{error}"))
}

fn function_or_panic<'a>(assembly: &'a Assembly, name: &str) -> &'a Function {
    assembly
        .function(name)
        .unwrap_or_else(|error| panic!("{error}"))
}

fn require_same(left: &Path, right: &Path, left_text: &str, right_text: &str) {
    if left_text == right_text {
        return;
    }
    let diff = unified(
        &left.display().to_string(),
        &right.display().to_string(),
        left_text,
        right_text,
    );
    panic!("probes are not equivalent\n{diff}");
}
