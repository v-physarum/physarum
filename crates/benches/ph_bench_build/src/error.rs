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
    /// A required [`crate::Bench`] setter was skipped.
    #[error("{label} is unset")]
    Unset { label: &'static str },
    /// `case` is not a name this bench knows.
    #[error("unknown case {case}")]
    UnknownCase { case: String },
    /// `path` is not a directory of case groups.
    #[error("{path} is not a case directory")]
    NotACaseDirectory { path: String },
    /// `path` is not a template or a group folder.
    #[error("unexpected entry {path}")]
    UnexpectedEntry { path: String },
    /// `case` names a kind with no template file.
    #[error("case {case} has no template {template}")]
    MissingTemplate { case: String, template: String },
    /// Two count directories name the same case.
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
        let error = Error::from(ErrorKind::RuntimeBuild {
            stderr: "missing crate".to_owned(),
        });
        assert_eq!(
            error.to_string(),
            "release runtime bench failed to build\nmissing crate"
        );
    }
}
