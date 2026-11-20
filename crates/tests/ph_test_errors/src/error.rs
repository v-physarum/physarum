//! Failures from starting `rustc` or reading its JSON diagnostics.
//!
//! `thiserror` writes each message. Call sites pass the values, not a formatted string.

use std::io;
use std::string::FromUtf8Error;

use thiserror::Error;

/// A probe failure. The message is the text shown to the caller.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct Error(#[from] ErrorKind);

/// Result of building one probe.
pub type Result<T> = std::result::Result<T, Error>;

/// Why a probe step failed.
#[derive(Debug, Error)]
pub(crate) enum ErrorKind {
    /// A filesystem call failed.
    #[error(transparent)]
    Io(#[from] io::Error),
    /// `rustc` did not start.
    #[error("rustc failed to start: {source}")]
    RustcStart {
        /// Spawn error.
        #[source]
        source: io::Error,
    },
    /// Compiler text was not UTF-8.
    #[error("{what} is not utf-8: {source}")]
    Utf8 {
        /// `rustc stderr`.
        what: &'static str,
        /// Decode error.
        #[source]
        source: FromUtf8Error,
    },
    /// A JSON diagnostic line could not be parsed.
    #[error("rustc JSON diagnostic is invalid: {source}")]
    Json {
        /// Decode error.
        #[source]
        source: serde_json::Error,
    },
    /// `rustc` failed and wrote no diagnostic.
    #[error("debug probe failed for {path}\n{stderr}")]
    ProbeFailed {
        /// Probe path passed to `rustc`.
        path: String,
        /// Compiler stderr.
        stderr: String,
    },
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        ErrorKind::Io(error).into()
    }
}
