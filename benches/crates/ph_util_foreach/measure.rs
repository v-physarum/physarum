//! Times a downstream crate that depends on `ph_util_foreach`.
//!
//! This bench points `ph_bench_build` at the case directory. That crate
//! discovers the cases and renders their source. Debug and release builds are
//! full compiles. Incremental time is debug only.

use std::path::PathBuf;
use std::process::ExitCode;

use ph_bench_build::Bench;

const MACRO_CRATE: &str = "crates/util/ph_util_foreach";
const CASES_DIR: &str = "benches/crates/ph_util_foreach/cases";

fn main() -> ExitCode {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let outcome = Bench::new("ph_util_foreach")
        .output("ph-util-foreach-expand")
        .user_package("ph_util_foreach_user")
        .dependency_path("ph_util_foreach", root.join(MACRO_CRATE))
        .cases(root.join(CASES_DIR))
        .run();
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}
