//! Assembly assertions for tests.
//!
//! [`AsmTester`] checks one release probe. [`AsmCompare`] checks that two
//! release probes match after normalization. This crate compiles the probe
//! with `rustc` and reads assembly. There is no debug profile.
//! The public entry point is this crate.

#![forbid(unsafe_code)]

mod assembly;
mod compare;
mod diff;
mod emit;
mod error;
mod normalize;
mod scan;
mod tester;

pub use compare::{AsmCompare, FunctionCompare, LoopCompare};
pub use tester::{AsmTester, Asserts, FunctionScope, Rules, TargetAssert};
