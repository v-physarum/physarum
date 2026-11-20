//! Failures from emitting or parsing a release probe.
//!
//! `thiserror` writes each message. Call sites pass the values, not a formatted string.

use std::io;
use std::string::FromUtf8Error;

use thiserror::Error;

/// A release-probe failure. The message is the text shown to the caller.
#[derive(Debug, Error)]
#[error(transparent)]
pub(crate) struct Error(#[from] ErrorKind);

/// Result of emitting or parsing one release probe.
pub(crate) type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub(crate) enum ErrorKind {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("rustc failed to start: {source}")]
    RustcStart {
        #[source]
        source: io::Error,
    },
    #[error("rustc failed for {path}\n{stderr}")]
    RustcFailed { path: String, stderr: String },
    #[error("{what} is not utf-8: {source}")]
    Utf8 {
        what: &'static str,
        #[source]
        source: FromUtf8Error,
    },
    #[error("rustc wrote no {what}")]
    EmptyEmit { what: &'static str },
    #[error("function `{name}` was not defined")]
    MissingFunction { name: String },
    #[error("function `{name}` has no loop")]
    NoHotLoop { name: String },
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        ErrorKind::Io(error).into()
    }
}
