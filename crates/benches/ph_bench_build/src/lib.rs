//! Runs hyperfine scenarios against a downstream crate.
//!
//! [`Bench`] reads directory cases. The workspace root, the path dependency,
//! the user package, and the output directory are taken from the workspace
//! when the bench does not set them. [`Series`] renders a generator at several
//! scales and checks the measured builds from a test.
#![forbid(unsafe_code)]

mod cases;
mod engine;
mod error;
mod growth;
mod json;
mod manifest;
mod passes;
mod source;
mod workspace;

pub use error::{Error, Result};
pub use growth::{
    Character, Fit, Group, Growth, LINEAR_EXPONENT, LINEAR_LIMIT, MIN_SCALE, MIN_SCALES,
    NOISE_ALLOWANCE, PassCeiling, PassSample, Point, QUADRATIC_EXPONENT, QUADRATIC_LIMIT,
    RustcPass, Scale, Series, Solver, TYPE_CHECK_CRATE_LIMIT, Verdict,
};

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
        Self { name: name.into(), path: path.into() }
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
            repeats: engine::DEFAULT_REPEATS,
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

    /// Directory whose folders and Rust files select the cases.
    ///
    /// `warmup`, `debug`, `incremental`, and `release` are the group
    /// directories. Under a group, each kind is a directory. Each count is a
    /// non-empty `{count}.rs` file in that kind directory, for example
    /// `debug/flat_structs/64.rs`. That file's text is the source. Kinds run
    /// in name order. Counts run in numeric order. Every count file is
    /// launched.
    ///
    /// `CASE_LEN` in the count file is replaced with the count. When `kind`
    /// ends in `s` and the kind directory also contains that name without the
    /// final `s` as a `.rs` file, the shorter file is the item and the count
    /// file is the header. The item is pasted once per count. `INDEX` is the
    /// paste index. `NEXT` in the item is the following index.
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
    /// Unset paths come from the workspace: the case directory is
    /// `benches/crates/{name}/cases`, the path dependency is the member named
    /// `name`, the user package is `{name}_user`, and the output directory is
    /// `{name}-expand`.
    ///
    /// # Errors
    ///
    /// Returns an error when `name` is not a workspace member, the case
    /// directory cannot be read, or a measured command fails.
    pub fn run(self) -> Result<()> {
        let layout = workspace::bench_layout(&self.name)?;
        let directory = self.cases.unwrap_or(layout.cases);
        let cases = cases::load(&directory)?;
        let plan = Plan {
            summary: format!("{} user compile time", self.name),
            output: self.output.unwrap_or(layout.output),
            package: self.package.unwrap_or(layout.package),
            dependency: self.dependency.unwrap_or(layout.dependency),
            cases,
            runtime: self.runtime,
            repeats: self.repeats,
        };
        engine::execute(&plan)
    }
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
