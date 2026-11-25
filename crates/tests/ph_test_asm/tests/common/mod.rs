use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TICK: AtomicU64 = AtomicU64::new(0);

pub fn write_probe(name: &str, source: &str) -> PathBuf {
    let path = scratch_dir().join(name);
    fs::write(&path, source).expect("probe");
    path
}

pub fn host_triple() -> String {
    let program = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
    let output = Command::new(program)
        .arg("-vV")
        .output()
        .expect("rustc -vV");
    let text = String::from_utf8(output.stdout).expect("utf-8");
    text.lines()
        .find_map(|line| line.strip_prefix("host: "))
        .expect("host triple")
        .to_owned()
}

pub const SHAPES: &str = r#"
#[unsafe(no_mangle)]
pub fn add(left: i32, right: i32) -> i32 {
    left.wrapping_add(right)
}

#[unsafe(no_mangle)]
pub fn sum(values: &[i32]) -> i32 {
    let mut total = 0i32;
    let mut index = 0usize;
    while index < values.len() {
        total = total.wrapping_add(values[index]);
        index = index.wrapping_add(1);
    }
    total
}

#[unsafe(no_mangle)]
pub fn at(values: &[i32], index: usize) -> i32 {
    values[index]
}

#[unsafe(no_mangle)]
pub fn pick(flag: bool, left: i32, right: i32) -> i32 {
    if flag { left } else { right }
}

#[unsafe(no_mangle)]
pub fn call_fn(func: fn(i32) -> i32, value: i32) -> i32 {
    func(value)
}

#[inline(never)]
fn double(value: i32) -> i32 {
    value.wrapping_mul(2)
}

#[unsafe(no_mangle)]
pub fn call_direct(value: i32) -> i32 {
    double(value)
}

#[unsafe(no_mangle)]
pub fn quotient(left: i32, right: i32) -> i32 {
    left / right
}

#[unsafe(no_mangle)]
pub fn sine(value: f64) -> f64 {
    value.sin()
}

#[unsafe(no_mangle)]
pub fn fused(left: f64, mid: f64, right: f64) -> f64 {
    left.mul_add(mid, right)
}

#[unsafe(no_mangle)]
pub fn raw(ptr: *const i32) -> *const i32 {
    ptr
}

#[unsafe(no_mangle)]
pub fn length(values: &[i32]) -> usize {
    values.len()
}

#[unsafe(no_mangle)]
pub fn bump(cell: &std::sync::atomic::AtomicU32) -> u32 {
    cell.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[unsafe(no_mangle)]
pub fn duplicate(source: &[u8; 64], dest: &mut [u8; 64]) {
    unsafe {
        memcpy_bytes(dest.as_mut_ptr(), source.as_ptr(), 64);
    }
}

unsafe extern "C" {
    fn memcpy_bytes(dest: *mut u8, src: *const u8, n: usize);
}

#[unsafe(no_mangle)]
pub fn heap() -> Vec<i32> {
    vec![1, 2, 3]
}

#[inline(never)]
fn observe(value: i32) {
    std::hint::black_box(value);
}

struct Guard(i32);

impl Drop for Guard {
    fn drop(&mut self) {
        observe(self.0);
    }
}

#[unsafe(no_mangle)]
pub fn may_invoke(flag: bool) -> i32 {
    let guard = Guard(1);
    if flag {
        panic!("boom");
    }
    std::hint::black_box(guard.0)
}

#[unsafe(no_mangle)]
pub fn with_alloca(limit: usize) -> i32 {
    let mut total = 0i32;
    let mut index = 0usize;
    while index < limit {
        let mut buffer = [0i32; 4];
        buffer[0] = index as i32;
        total = total.wrapping_add(std::hint::black_box(buffer)[0]);
        index = index.wrapping_add(1);
    }
    total
}

#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
pub fn gather(base: *const i32, index: core::arch::x86_64::__m128i) -> core::arch::x86_64::__m128i {
    unsafe { core::arch::x86_64::_mm_i32gather_epi32(base, index, 4) }
}
"#;

pub const ADD_THEN_SUB: &str = r"
#[unsafe(no_mangle)]
pub fn add(left: i32, right: i32) -> i32 {
    left.wrapping_add(right)
}

#[unsafe(no_mangle)]
pub fn sub(left: i32, right: i32) -> i32 {
    left.wrapping_sub(right)
}
";

pub const SUB_THEN_ADD: &str = r"
#[unsafe(no_mangle)]
pub fn sub(left: i32, right: i32) -> i32 {
    left.wrapping_sub(right)
}

#[unsafe(no_mangle)]
pub fn add(left: i32, right: i32) -> i32 {
    left.wrapping_add(right)
}
";

pub fn step_loop(step: i32) -> String {
    format!(
        r"
#[unsafe(no_mangle)]
pub fn k(values: &[i32]) -> i32 {{
    let mut total = 0i32;
    for value in values {{
        total = total.wrapping_add(*value).wrapping_add({step});
    }}
    total
}}

#[unsafe(no_mangle)]
pub fn other() -> i32 {{
    {step}
}}
"
    )
}

const RLIB_SOURCE_NAME: &str = "lib.rs";
const RLIB_FILE_PREFIX: &str = "lib";
const RLIB_EXTENSION: &str = ".rlib";

/// Builds `source` as the release rlib `lib{name}.rlib` inside `dir`.
///
/// Each `(name, path)` in `externs` is passed as `--extern name=path`.
pub fn write_rlib(dir: &Path, name: &str, source: &str, externs: &[(&str, &Path)]) -> PathBuf {
    let source_path = dir.join(format!("{name}_{RLIB_SOURCE_NAME}"));
    fs::write(&source_path, source).expect("rlib source");
    let rlib = dir.join(format!("{RLIB_FILE_PREFIX}{name}{RLIB_EXTENSION}"));
    let program = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
    let mut command = Command::new(program);
    command
        .arg("--edition")
        .arg("2024")
        .arg("--crate-type")
        .arg("rlib")
        .arg("--crate-name")
        .arg(name)
        .arg("-C")
        .arg("opt-level=3")
        .arg("-L")
        .arg(dir);
    for (extern_name, path) in externs {
        command
            .arg("--extern")
            .arg(format!("{extern_name}={}", path.display()));
    }
    let output = command
        .arg(&source_path)
        .arg("-o")
        .arg(&rlib)
        .output()
        .expect("rustc");
    assert!(
        output.status.success(),
        "rlib `{name}` failed to build\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    rlib
}

/// A fresh empty directory for one test.
pub fn scratch_dir() -> PathBuf {
    let n = TICK.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("ph-test-asm-{n}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    dir
}
