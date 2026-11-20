//! Release probe compiler used by `ph_test_asm`.
//!
//! [`Probe`] invokes `rustc` and emits LLVM IR plus assembly. [`Assembly`]
//! parses that text into [`Function`], [`Loop`], and [`Check`] values.
//! The crate does not choose a profile other than release.

#![forbid(unsafe_code)]

mod error;
mod ir;
mod probe;

pub use error::{Error, Result};
pub use ir::{Assembly, Check, CheckKind, Function, Loop};
pub use probe::Probe;
