//! Scale series measured by building the rendered source.

use ph_bench_build::{Group, Growth, Scale, Series};

const FLAT_BEFORE: &str = "use ph_util_foreach::impl_foreach;\n\nimpl_foreach!(n in 0..";
const FLAT_AFTER: &str = " {\n    pub struct paste(Ix, n);\n});\n";
const HAND_PREFIX: &str = "pub struct Ix";
const HAND_SUFFIX: &str = ";\n";
/// Decimal digits of a `u32`.
const DIGIT_CAP: usize = 10;
/// Bytes reserved for one hand-written struct line, including a `u32`'s digits.
const HAND_LINE_CAP: usize = HAND_PREFIX.len() + DIGIT_CAP + HAND_SUFFIX.len();

#[test]
fn hand_structs_grow_linearly() {
    Series::new("hand_structs")
        .group(Group::Debug)
        .scales([64, 1024, 8192])
        .expect(Growth::Linear)
        .render(hand_structs)
        .check()
        .unwrap();
}

#[test]
fn flat_structs_grow_linearly_in_debug() {
    flat(Group::Debug, [64, 1024, 8192]).check().unwrap();
}

#[test]
fn flat_structs_grow_linearly_in_release() {
    flat(Group::Release, [1024, 8192]).check().unwrap();
}

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
