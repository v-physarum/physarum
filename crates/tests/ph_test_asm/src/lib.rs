//! Assembly and LLVM IR assertions for tests.
//!
//! [`AsmTester`] checks one release probe. [`AsmCompare`] checks that two
//! release probes match after normalization. This crate compiles the probe
//! with `rustc` and reads LLVM IR and assembly. There is no debug profile.
//! The public entry point is this crate.

#![forbid(unsafe_code)]

mod compare;
mod diff;
mod emit;
mod error;
mod ir;
mod normalize;
mod scan;
mod tester;

pub use compare::{AsmCompare, FunctionCompare, LoopCompare};
pub use tester::{AsmTester, Asserts, FunctionScope, Rules, TargetAssert};
