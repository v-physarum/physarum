//! Times `impl_foreach!` in a downstream crate with hyperfine 2.
//!
//! The user-crate sources are the files in `benches/crates/ph_util_foreach/cases`.
//!
//! The proc-macro is compiled once and is not part of the case times.
//! Debug and release builds are full compiles. Incremental time is debug only.
//! Runtime time is the release bench.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use ph_bench_build::{Bench, CaseName, Error, Fill, Repeat, Result};

const MACRO_CRATE: &str = "crates/util/ph_util_foreach";
const CASES_DIR: &str = "benches/crates/ph_util_foreach/cases";
const FLAT_STRUCTS_RS: &str = "flat_structs.rs";
const HAND_STRUCTS_RS: &str = "hand_structs.rs";
const HAND_STRUCT_RS: &str = "hand_struct.rs";
const MANY_CALLS_RS: &str = "many_calls.rs";
const MANY_CALL_RS: &str = "many_call.rs";
const FAN_TAKE_RS: &str = "fan_take.rs";
const FAN_ROWS_RS: &str = "fan_rows.rs";
const DEBUG_CASES: &[&str] = &[
    "flat_structs_64",
    "flat_structs_1024",
    "flat_structs_8192",
    "hand_structs_64",
    "hand_structs_1024",
    "hand_structs_8192",
    "one_call_256",
    "many_calls_256",
    "fan_take_16",
    "fan_take_64",
    "fan_rows_64",
    "fan_rows_256",
];
const RELEASE_CASES: &[&str] = &[
    "flat_structs_1024",
    "hand_structs_1024",
    "flat_structs_8192",
    "fan_take_64",
    "fan_rows_256",
];

fn main() -> ExitCode {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let outcome = Bench::new("ph_util_foreach")
        .output("ph-util-foreach-expand")
        .user_package("ph_util_foreach_user")
        .dependency_path("ph_util_foreach", root.join(MACRO_CRATE))
        .render(render_case)
        .warmup("flat_structs_1")
        .debug(DEBUG_CASES)
        .incremental("flat_structs_1024")
        .release(RELEASE_CASES)
        .runtime("ph_util_foreach_runtime")
        .run();
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn render_case(case: &str) -> Result<String> {
    let cases = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(CASES_DIR);
    let parsed = CaseName::parse(case)?;
    let len = parsed.count().to_string();
    match parsed.kind() {
        "flat_structs" | "one_call" => cases
            .join(FLAT_STRUCTS_RS)
            .fill(&[("CASE_LEN", len.as_str())]),
        "hand_structs" => repeat(
            &cases,
            (HAND_STRUCTS_RS, HAND_STRUCT_RS),
            parsed.count(),
            None,
        ),
        "many_calls" => repeat(
            &cases,
            (MANY_CALLS_RS, MANY_CALL_RS),
            parsed.count(),
            Some("NEXT"),
        ),
        "fan_take" => cases.join(FAN_TAKE_RS).fill(&[("CASE_LEN", len.as_str())]),
        "fan_rows" => cases.join(FAN_ROWS_RS).fill(&[("CASE_LEN", len.as_str())]),
        _ => Err(Error::unknown_case(case)),
    }
}

fn repeat(cases: &Path, files: (&str, &str), count: u32, next: Option<&str>) -> Result<String> {
    let header = cases.join(files.0);
    let item = cases.join(files.1);
    Repeat::new(&header, &item, count, next).load()
}
