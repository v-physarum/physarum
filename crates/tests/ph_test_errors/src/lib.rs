//! Compile-fail asserts for debug probes.
//!
//! [`ErrorTester`] is the public API. It checks that a probe emits an error
//! code, or that an error message contains a description. The message check
//! includes a diagnostic whose code is null. The probe is its own crate.
//! Pass a file or source text, not a module of the test crate. Pass each
//! extern crate as a name and a path. The check compiles the probe once and
//! does not allocate while matching the parsed diagnostics.

#![forbid(unsafe_code)]

mod diagnostics;
mod error;
mod probe;
mod tester;

pub use tester::ErrorTester;
