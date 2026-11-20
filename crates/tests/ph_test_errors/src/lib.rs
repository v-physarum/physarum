//! Compile-fail asserts for debug probes.
//!
//! [`ErrorTester`] is the public API. It checks that a probe emits an error
//! code and a description. The probe is its own crate. Pass a file or source
//! text, not a module of the test crate. The check compiles the probe once
//! and does not allocate while matching the parsed diagnostics.
//!
//! This crate does not emit LLVM IR or assembly.

#![forbid(unsafe_code)]

mod diagnostics;
mod error;
mod probe;
mod tester;

pub use tester::ErrorTester;
