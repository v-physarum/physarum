//! Failures from reading a bench report or running a scenario.
//!
//! `thiserror` writes each message. Call sites pass the values, not a formatted string.

use std::io;
use std::string::FromUtf8Error;

use thiserror::Error;

use crate::json::ReadError;

/// A bench failure. The message is the text shown to the operator.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct Error(#[from] ErrorKind);

/// Result of one bench step.
pub type Result<T> = std::result::Result<T, Error>;

/// Why a bench step failed.
#[derive(Debug, Error)]
pub(crate) enum ErrorKind {
    /// `CARGO_MANIFEST_DIR` was not set.
    #[error("CARGO_MANIFEST_DIR is unset; run cargo bench")]
    MissingManifestDir,
    /// A filesystem call failed.
    #[error(transparent)]
    Io(#[from] io::Error),
    /// A hyperfine or cargo JSON document could not be read.
    #[error(transparent)]
    Read(#[from] ReadError),
    /// The report document could not be written.
    #[error(transparent)]
    Report(#[from] serde_json::Error),
    /// The downstream manifest could not be written.
    #[error(transparent)]
    Manifest(#[from] toml::ser::Error),
    /// `case` is not a name this bench knows.
    #[error("unknown case {case}")]
    UnknownCase { case: String },
    /// `path` is not a directory of case groups.
    #[error("{path} is not a case directory")]
    NotACaseDirectory { path: String },
    /// `path` is not a group folder, a kind folder, or a source file.
    #[error("unexpected entry {path}")]
    UnexpectedEntry { path: String },
    /// Two count files name the same case.
    #[error("duplicate case {case}")]
    DuplicateCase { case: String },
    /// The case directory has no warmup case.
    #[error("warmup case is missing")]
    MissingWarmup,
    /// A measured command did not start.
    #[error("{label} failed to start: {source}")]
    CommandStart {
        label: String,
        #[source]
        source: io::Error,
    },
    /// A measured command exited with a failure.
    #[error("{label} failed\n{output}")]
    CommandFailed { label: String, output: String },
    /// The release runtime bench did not build.
    #[error("release runtime bench failed to build\n{stderr}")]
    RuntimeBuild { stderr: String },
    /// `program` is not on `PATH`.
    #[error("{program} is required")]
    ProgramMissing { program: &'static str },
    /// `program` exited with a failure.
    #[error("{program} failed")]
    ProgramFailed { program: &'static str },
    /// Bytes that should have been text were not UTF-8.
    #[error("{what} is not utf-8: {source}")]
    Utf8 {
        what: &'static str,
        #[source]
        source: FromUtf8Error,
    },
    /// Output from `program` was not UTF-8.
    #[error("{program} output is not utf-8: {source}")]
    ProgramUtf8 {
        program: &'static str,
        #[source]
        source: FromUtf8Error,
    },
    /// One run of the release runtime bench failed.
    #[error("runtime bench run {run} failed")]
    RuntimeRun { run: u32 },
    /// The runtime bench printed `runtime_seconds` with a bad number.
    #[error("runtime_seconds is not a number")]
    RuntimeSeconds,
    /// The runtime bench printed no `runtime_seconds` line.
    #[error("runtime bench did not print runtime_seconds")]
    MissingRuntimeSeconds,
    /// A growth series has an empty name.
    #[error("series name is empty")]
    SeriesName,
    /// A growth series has no group.
    #[error("series {series} group is unset")]
    SeriesGroup { series: String },
    /// A growth series has too few scales to show growth.
    #[error("{series} needs at least {min} scales")]
    SeriesTooFew { series: String, min: usize },
    /// A scale is zero, so it has no logarithm.
    #[error("{series} scale {scale} is not a positive size")]
    SeriesScale { series: String, scale: u32 },
    /// The same scale was declared twice.
    #[error("{series} scale {scale} is duplicated")]
    SeriesDuplicate { series: String, scale: u32 },
    /// A growth series has no claim.
    #[error("series {series} expectation is unset")]
    SeriesExpect { series: String },
    /// The generator returned an empty source.
    #[error("series {series} scale {scale} rendered an empty source")]
    SeriesSource { series: String, scale: u32 },
    /// [`crate::Series::render`] was not called, or the scales changed after it.
    #[error("series {series} has no rendered sources")]
    SeriesSources { series: String },
    /// An incremental series has no [`crate::Series::edit`].
    #[error("series {series} incremental build has no edit")]
    SeriesEdit { series: String },
    /// [`crate::Series::edit`] returned an empty source.
    #[error("series {series} scale {scale} edit is empty")]
    SeriesEditEmpty { series: String, scale: u32 },
    /// [`crate::Series::edit`] returned the rendered source unchanged.
    #[error("series {series} scale {scale} edit does not change the source")]
    SeriesEditSame { series: String, scale: u32 },
    /// The time slice length does not match the scales.
    #[error("{series} has {scales} scales but {times} times")]
    SeriesTimeCount {
        series: String,
        scales: usize,
        times: usize,
    },
    /// A measured time is not a positive finite number.
    #[error("{series} scale {scale} time is not a positive finite number")]
    SeriesTime { series: String, scale: u32 },
    /// The log-log fit is not a finite exponent.
    #[error("{series} scales are not distinguishable on a log axis")]
    SeriesLog { series: String },
    /// A linear claim missed [`crate::LINEAR_LIMIT`].
    #[error("{series} is not linear: exponent {exponent}, linear limit {limit}")]
    NotLinear {
        series: String,
        exponent: f64,
        limit: f64,
    },
    /// A not-quadratic claim hit [`crate::QUADRATIC_LIMIT`].
    #[error("{series} is quadratic: exponent {exponent}, quadratic limit {limit}")]
    QuadraticGrowth {
        series: String,
        exponent: f64,
        limit: f64,
    },
    /// No workspace `Cargo.toml` was found walking up from `start`.
    #[error("workspace root was not found from {start}")]
    WorkspaceMissing { start: String },
    /// `path` is not valid TOML.
    #[error("{path} is not a readable manifest: {source}")]
    ManifestRead {
        path: String,
        #[source]
        source: toml::de::Error,
    },
    /// `package` is not a member of the workspace.
    #[error("package {package} is not a workspace member")]
    MemberMissing { package: String },
    /// The rendered source uses a crate that is not in the workspace.
    #[error("{series} uses {crate_name}, which is not a workspace member")]
    SeriesCrate { series: String, crate_name: String },
    /// A series sets both a growth claim and a pass ceiling.
    #[error("{series} sets both a growth claim and a pass ceiling")]
    SeriesBothClaims { series: String },
    /// The ceiling scale is not one of the series scales.
    #[error("{series} ceiling scale {scale} is not in the series")]
    PassScale { series: String, scale: u32 },
    /// Nightly rustc is required for `-Ztime-passes`.
    #[error("nightly rustc is required for -Ztime-passes")]
    NightlyMissing,
    /// The user crate's `-Ztime-passes` log has no line for this pass.
    #[error("no {pass} time after compiling {package}")]
    PassMissing { pass: String, package: String },
    /// A pass line did not contain a finite number of seconds.
    #[error("{pass} time is not a number of seconds")]
    PassText { pass: String },
    /// A pass time is not a finite non-negative number.
    #[error("{pass} at scale {scale} time is not a finite non-negative number")]
    PassTime { pass: String, scale: u32 },
    /// A pass took longer than its ceiling.
    #[error("{pass} at scale {scale} took {elapsed_ms} ms, limit is {limit_ms} ms")]
    PassSlow {
        pass: String,
        scale: u32,
        elapsed_ms: u128,
        limit_ms: u128,
    },
}

impl Error {
    /// `case` is not a name this bench knows.
    #[must_use]
    pub(crate) fn unknown_case(case: impl Into<String>) -> Self {
        ErrorKind::UnknownCase { case: case.into() }.into()
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        ErrorKind::Io(error).into()
    }
}

impl From<ReadError> for Error {
    fn from(error: ReadError) -> Self {
        ErrorKind::Read(error).into()
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        ErrorKind::Report(error).into()
    }
}

impl From<toml::ser::Error> for Error {
    fn from(error: toml::ser::Error) -> Self {
        ErrorKind::Manifest(error).into()
    }
}

#[cfg(test)]
mod tests {
    use super::{Error, ErrorKind};

    #[test]
    fn runtime_build_failure_includes_stderr() {
        let error = Error::from(ErrorKind::RuntimeBuild { stderr: "missing crate".to_owned() });
        assert_eq!(
            error.to_string(),
            "release runtime bench failed to build\nmissing crate"
        );
    }
}
