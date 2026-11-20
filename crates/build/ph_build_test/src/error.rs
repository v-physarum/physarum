//! Failures from compiling a probe or reading its LLVM IR.
//!
//! `thiserror` writes each message. Call sites pass the values, not a formatted string.

use std::io;
use std::string::FromUtf8Error;

use thiserror::Error;

/// A probe failure. The message is the text shown to the caller.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct Error(#[from] ErrorKind);

/// Result of compiling or parsing one probe.
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
    /// `rustc` exited with a failure.
    #[error("rustc failed for {path}\n{stderr}")]
    RustcFailed {
        /// Probe path passed to `rustc`.
        path: String,
        /// Compiler stderr.
        stderr: String,
    },
    /// Emitted text was not UTF-8.
    #[error("{what} is not utf-8: {source}")]
    Utf8 {
        /// `LLVM IR` or assembly.
        what: &'static str,
        /// Decode error.
        #[source]
        source: FromUtf8Error,
    },
    /// [`crate::Probe::emit`] ran without [`crate::Probe::release`].
    #[error("Probe::release is required before emit")]
    NotRelease,
    /// `rustc` wrote no file of this kind.
    #[error("rustc wrote no {what}")]
    EmptyEmit {
        /// `LLVM IR` or assembly.
        what: &'static str,
    },
    /// The named function is not a `define` in the module.
    #[error("function `{name}` was not defined")]
    MissingFunction {
        /// Function name.
        name: String,
    },
    /// The function has no natural loop.
    #[error("function `{name}` has no loop")]
    NoHotLoop {
        /// Function name.
        name: String,
    },
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        ErrorKind::Io(error).into()
    }
}
