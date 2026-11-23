//! Times directory cases of a downstream crate that depends on `ph_util_foreach`.
//!
//! Scale series are the `ph_util_foreach_series` bench.
//! `fan_take` and `fan_rows` stay files: `fan_take` grows its `repeat` list
//! with the index, and `fan_rows` spans only four times.

use std::process::ExitCode;

use ph_bench_build::{Bench, Result};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<()> {
    Bench::new("ph_util_foreach").run()
}
