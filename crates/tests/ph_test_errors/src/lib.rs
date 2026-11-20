//! Compile-fail asserts for debug probes.
//!
//! [`Probe`] builds one crate with `rustc` at `opt-level=0` and returns the
//! compiler diagnostics. [`ErrorTester`] checks those diagnostics. The probe is
//! its own crate. Pass a file or source text, not a module of the test crate.
//! The first assert compiles the probe. Later asserts reuse the parsed
//! diagnostics and do not allocate while walking them.
//!
//! This crate does not emit LLVM IR or assembly.

#![forbid(unsafe_code)]

mod diagnostics;
mod error;
mod probe;
mod tester;

pub use diagnostics::{Diagnostic, Diagnostics, Level, Span};
pub use error::{Error, Result};
pub use probe::Probe;
pub use tester::ErrorTester;
