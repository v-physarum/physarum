//! Runs hyperfine scenarios against a downstream crate.
//!
//! A bench selects cases with [`Bench`] and supplies the source for each case.
//! The dependency compile is paid once, before the case times.

mod engine;
mod error;
mod json;
mod manifest;
mod source;

pub use error::{Error, Result};
pub use source::{CaseName, Fill, Repeat};

use std::path::{Path, PathBuf};

/// Builds the downstream `src/lib.rs` for one case name.
pub(crate) type RenderCase = Box<dyn Fn(&str) -> Result<String>>;

/// Path dependency written into the downstream crate.
pub(crate) struct Dependency {
    name: String,
    path: PathBuf,
}

impl Dependency {
    /// A path dependency named `name`.
    #[must_use]
    pub(crate) fn path(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
        }
    }

    /// Dependency key in the downstream manifest.
    #[must_use]
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// Path written into the downstream manifest.
    #[must_use]
    pub(crate) fn directory(&self) -> &Path {
        &self.path
    }
}

/// Scenarios for one downstream crate, chosen before [`Bench::run`].
#[must_use]
pub struct Bench {
    name: String,
    output: Option<String>,
    package: Option<String>,
    dependency: Option<Dependency>,
    render: Option<RenderCase>,
    warmup: Option<String>,
    debug: Vec<String>,
    incremental: Option<String>,
    release: Vec<String>,
    runtime: Option<String>,
    repeats: u32,
}

impl Bench {
    /// Starts a plan whose report title uses `name`.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            output: None,
            package: None,
            dependency: None,
            render: None,
            warmup: None,
            debug: Vec::new(),
            incremental: None,
            release: Vec::new(),
            runtime: None,
            repeats: 3,
        }
    }

    /// Directory name under `target/` when cargo does not pass an output path.
    pub fn output(mut self, name: impl Into<String>) -> Self {
        self.output = Some(name.into());
        self
    }

    /// Package name of the downstream crate cargo builds.
    pub fn user_package(mut self, name: impl Into<String>) -> Self {
        self.package = Some(name.into());
        self
    }

    /// Path dependency of the downstream crate. `name` is the manifest key.
    pub fn dependency_path(mut self, name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        self.dependency = Some(Dependency::path(name, path));
        self
    }

    /// Renders one case name into the downstream `src/lib.rs`.
    pub fn render<F>(mut self, render: F) -> Self
    where
        F: Fn(&str) -> Result<String> + 'static,
    {
        self.render = Some(Box::new(render));
        self
    }

    /// Case built once so the dependency is compiled before the measured runs.
    pub fn warmup(mut self, case: impl Into<String>) -> Self {
        self.warmup = Some(case.into());
        self
    }

    /// Full debug builds, in the given order.
    pub fn debug(mut self, cases: &[&str]) -> Self {
        self.debug.extend(cases.iter().copied().map(str::to_owned));
        self
    }

    /// One incremental debug build. The package is not cleaned first.
    pub fn incremental(mut self, case: impl Into<String>) -> Self {
        self.incremental = Some(case.into());
        self
    }

    /// Full release builds, in the given order.
    pub fn release(mut self, cases: &[&str]) -> Self {
        self.release
            .extend(cases.iter().copied().map(str::to_owned));
        self
    }

    /// Release bench whose executable prints `runtime_seconds`.
    pub fn runtime(mut self, bench: impl Into<String>) -> Self {
        self.runtime = Some(bench.into());
        self
    }

    /// Builds the warmup case, then runs the selected scenarios.
    ///
    /// # Errors
    ///
    /// Returns an error when a required setter was skipped, a case fails to
    /// render, or a measured command fails.
    pub fn run(self) -> Result<()> {
        let plan = Plan {
            summary: format!("{} user compile time", self.name),
            output: required(self.output, "output directory")?,
            package: required(self.package, "user package")?,
            dependency: required(self.dependency, "dependency")?,
            render: required(self.render, "case renderer")?,
            warmup: required(self.warmup, "warmup case")?,
            debug: self.debug,
            incremental: self.incremental,
            release: self.release,
            runtime: self.runtime,
            repeats: self.repeats,
        };
        engine::execute(&plan)
    }
}

fn required<T>(value: Option<T>, label: &'static str) -> Result<T> {
    value.ok_or(crate::error::ErrorKind::Unset { label }.into())
}

/// One filled-in [`Bench`].
pub(crate) struct Plan {
    /// Directory name under `target/`.
    pub(crate) output: String,
    /// Heading written to the GitHub step summary.
    pub(crate) summary: String,
    /// Downstream package name.
    pub(crate) package: String,
    /// Path dependency of that package.
    pub(crate) dependency: Dependency,
    /// Renders a case name into source text.
    pub(crate) render: RenderCase,
    /// Case compiled before the measured runs.
    pub(crate) warmup: String,
    /// Full debug cases.
    pub(crate) debug: Vec<String>,
    /// Incremental debug case.
    pub(crate) incremental: Option<String>,
    /// Full release cases.
    pub(crate) release: Vec<String>,
    /// Release runtime bench name.
    pub(crate) runtime: Option<String>,
    /// Measured hyperfine runs per case.
    pub(crate) repeats: u32,
}
