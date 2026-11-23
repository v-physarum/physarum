//! Scale series for `ph_util_foreach`: debug and release build time versus `N`.
//!
//! Each series renders one source per scale, times `cargo build` with
//! hyperfine, and checks the log-log exponent. `hand_structs` and
//! `flat_structs` claim linear growth.

use std::process::ExitCode;

use ph_bench_build::{Group, Growth, Result, Scale, Series};

const FLAT_BEFORE: &str = "use ph_util_foreach::impl_foreach;\n\nimpl_foreach!(n in 0..";
const FLAT_AFTER: &str = " {\n    pub struct paste(Ix, n);\n});\n";
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
    Series::new("hand_structs")
        .group(Group::Debug)
        .scales([64, 1024, 8192])
        .expect(Growth::Linear)
        .render(hand_structs)
        .check()?;
    flat(Group::Debug, [64, 1024, 8192]).check()?;
    flat(Group::Release, [1024, 8192]).check()?;
    Ok(())
}

#[test]
fn an_incremental_edit_stays_linear() {
    Series::new("incremental_touch")
        .group(Group::Incremental)
        .scales([8, 32])
        .expect(Growth::Linear)
        .render(hand_structs)
        .edit(append_marker)
        .check()
        .unwrap();
}

fn append_marker(source: &str) -> String {
    let mut edited = String::with_capacity(source.len() + MARKER.len());
    edited.push_str(source);
    edited.push_str(MARKER);
    edited
}

const MARKER: &str = "pub fn marker() {}\n";

fn flat<const LEN: usize>(group: Group, scales: [u32; LEN]) -> Series {
    Series::new("flat_structs")
        .group(group)
        .scales(scales)
        .expect(Growth::Linear)
        .render(flat_source)
}

fn flat_source(scale: Scale) -> String {
    let mut source = String::with_capacity(FLAT_BEFORE.len() + DIGIT_CAP + FLAT_AFTER.len());
    source.push_str(FLAT_BEFORE);
    push_u32(&mut source, scale.count());
    source.push_str(FLAT_AFTER);
    source
}

fn hand_structs(scale: Scale) -> String {
    let count = scale.count();
    let mut source = String::with_capacity(HAND_LINE_CAP * count as usize);
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
