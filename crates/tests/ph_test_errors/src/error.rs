//! Failures from starting `rustc` or reading its JSON diagnostics.
//!
//! `thiserror` writes each message. Call sites pass the values, not a formatted string.

use std::io;
use std::string::FromUtf8Error;

use thiserror::Error;

/// Why a probe step failed.
#[derive(Debug, Error)]
pub(crate) enum Error {
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
    #[error("rustc stderr is not utf-8: {source}")]
    Utf8 {
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

/// Result of building one probe.
pub(crate) type Result<T> = std::result::Result<T, Error>;
