//! Times a downstream crate that depends on `ph_util_foreach`.
//!
//! This bench points `ph_bench_build` at the case directory and at the scale
//! series whose size is `N` items. Debug and release builds are full compiles.
//! Incremental time is debug only. A single scale stays a directory case.
//! `fan_take` stays too: each step's `repeat` list grows with the index.
//! `fan_rows` spans only four times (64 and 256), which is too short for the
//! noise allowance.

use std::path::PathBuf;
use std::process::ExitCode;

use ph_bench_build::{Bench, Group, Growth, Result, Scale, Series};

const MACRO_CRATE: &str = "crates/util/ph_util_foreach";
const CASES_DIR: &str = "benches/crates/ph_util_foreach/cases";
const FLAT_BEFORE: &str = "use ph_util_foreach::impl_foreach;\n\nimpl_foreach!(n in 0..";
const FLAT_AFTER: &str = " {\n    pub struct paste(Ix, n);\n});\n";
const HAND_HEADER: &str = "#![allow(unused_imports)]\nuse ph_util_foreach::impl_foreach;\n\n";
const HAND_PREFIX: &str = "pub struct Ix";
const HAND_SUFFIX: &str = ";\n";
/// Decimal digits of a `u32`.
const DIGIT_CAP: usize = 10;
/// Bytes reserved for one hand-written struct line, including a `u32`'s digits.
const HAND_LINE_CAP: usize = HAND_PREFIX.len() + DIGIT_CAP + HAND_SUFFIX.len();

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
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    Bench::new("ph_util_foreach")
        .output("ph-util-foreach-expand")
        .user_package("ph_util_foreach_user")
        .dependency_path("ph_util_foreach", root.join(MACRO_CRATE))
        .cases(root.join(CASES_DIR))
        .series(flat_structs(
            Group::Debug,
            [64, 1024, 8192],
            Growth::Linear,
        )?)
        .series(flat_structs(Group::Release, [1024, 8192], Growth::Linear)?)
        .series(hand_structs()?)
        .run()
}

fn flat_structs<const LEN: usize>(
    group: Group,
    scales: [u32; LEN],
    expect: Growth,
) -> Result<Series> {
    Series::new("flat_structs")
        .group(group)
        .scales(scales)
        .expect(expect)
        .render(flat_struct_source)
}

fn hand_structs() -> Result<Series> {
    Series::new("hand_structs")
        .group(Group::Debug)
        .scales([64, 1024, 8192])
        .expect(Growth::Linear)
        .render(hand_struct_source)
}

fn flat_struct_source(scale: Scale) -> String {
    let mut source = String::with_capacity(FLAT_BEFORE.len() + DIGIT_CAP + FLAT_AFTER.len());
    source.push_str(FLAT_BEFORE);
    push_u32(&mut source, scale.count());
    source.push_str(FLAT_AFTER);
    source
}

fn hand_struct_source(scale: Scale) -> String {
    let count = scale.count();
    let mut source = String::with_capacity(HAND_HEADER.len() + HAND_LINE_CAP * count as usize);
    source.push_str(HAND_HEADER);
    for index in 0..count {
        source.push_str(HAND_PREFIX);
        push_u32(&mut source, index);
        source.push_str(HAND_SUFFIX);
    }
    source
}

fn push_u32(out: &mut String, mut value: u32) {
    let mut bytes = [0u8; DIGIT_CAP];
    let mut at = bytes.len();
    loop {
        at -= 1;
        bytes[at] = b"0123456789"[(value % 10) as usize];
        value /= 10;
        if value == 0 {
            break;
        }
    }
    for byte in &bytes[at..] {
        out.push(char::from(*byte));
    }
}
