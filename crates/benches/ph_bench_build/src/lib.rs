//! Runs hyperfine scenarios against a downstream crate.
//!
//! A bench points [`Bench`] at a case directory. The directory's folders and
//! file names select the warmup, debug, incremental, and release cases and
//! supply each case's source. The dependency compile is paid once, before the
//! case times.

mod cases;
mod engine;
mod error;
mod json;
mod manifest;
mod source;

pub use error::{Error, Result};

use std::path::{Path, PathBuf};

use cases::Groups;

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
    cases: Option<PathBuf>,
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
            cases: None,
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

    /// Directory whose folders and files select the cases.
    ///
    /// `warmup`, `debug`, `incremental`, and `release` are the group folders.
    /// A file in a group folder names one case, `{kind}_{count}`, for example
    /// `flat_structs_64`. That file's bytes are not read. Kinds run in name
    /// order. Counts run in numeric order. Every file in a group is launched.
    ///
    /// `{kind}.rs` in the case directory is the source. `CASE_LEN` in that
    /// file is replaced with the count. When `kind` ends in `s` and the case
    /// directory also contains that name without the final `s` as a `.rs`
    /// file, the shorter file is the item and `{kind}.rs` is the header. The
    /// item is pasted once per count. `INDEX` is the paste index. `NEXT` in
    /// the item is the following index.
    ///
    /// At least one warmup case is required.
    pub fn cases(mut self, directory: impl Into<PathBuf>) -> Self {
        self.cases = Some(directory.into());
        self
    }

    /// Release bench whose executable prints `runtime_seconds`.
    pub fn runtime(mut self, bench: impl Into<String>) -> Self {
        self.runtime = Some(bench.into());
        self
    }

    /// Builds the warmup cases, then runs the selected scenarios.
    ///
    /// # Errors
    ///
    /// Returns an error when a required setter was skipped, the case directory
    /// cannot be read, or a measured command fails.
    pub fn run(self) -> Result<()> {
        let directory = required(self.cases, "case directory")?;
        let plan = Plan {
            summary: format!("{} user compile time", self.name),
            output: required(self.output, "output directory")?,
            package: required(self.package, "user package")?,
            dependency: required(self.dependency, "dependency")?,
            cases: cases::load(&directory)?,
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
    /// Cases discovered from the case directory.
    pub(crate) cases: Groups,
    /// Release runtime bench name.
    pub(crate) runtime: Option<String>,
    /// Measured hyperfine runs per case.
    pub(crate) repeats: u32,
}
