//! Debug probe driver.
//!
//! [`Probe`] builds one crate with `rustc` at `opt-level=0` and returns the
//! compiler diagnostics. Compile errors are [`Diagnostics`]. [`Error`] is
//! reserved for a `rustc` run that does not start or whose output cannot be read.

#![forbid(unsafe_code)]

mod diagnostics;
mod error;
mod probe;

pub use diagnostics::{Diagnostic, Diagnostics, Level, Span};
pub use error::{Error, Result};
pub use probe::Probe;
