//! Normalization for assembly equivalence.

use crate::assembly::{Assembly, Function};
use crate::span::ByteSpan;

const NEWLINE_LEN: usize = 1;
const LOCAL_PREFIX: &str = ".L";
const LOCAL_LABEL_MIN_BYTES: usize = 3;
const DECIMAL_DIGITS: usize = 20;

/// Whole module, functions sorted by symbol.
pub(crate) fn module_text(assembly: &Assembly) -> String {
    let mut functions: Vec<&Function> = assembly.functions().iter().collect();
    functions.sort_by(|left, right| left.name().cmp(right.name()));
    let mut cap = 0usize;
    for function in &functions {
        cap = cap
            .saturating_add(function.asm().len())
            .saturating_add(NEWLINE_LEN);
    }
    let mut labels = LabelTable::new();
    let mut out = String::with_capacity(cap);
    for (index, function) in functions.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        append_normalized(function.asm(), &mut labels, &mut out);
    }
    out
}

/// One function, local labels canonicalized.
pub(crate) fn function_text(function: &Function) -> String {
    let mut out = String::with_capacity(function.asm().len());
    let mut labels = LabelTable::new();
    append_normalized(function.asm(), &mut labels, &mut out);
    out
}

/// Hot loop text for one function.
pub(crate) fn loop_text(hot: &str) -> String {
    let mut out = String::with_capacity(hot.len());
    let mut labels = LabelTable::new();
    append_normalized(hot, &mut labels, &mut out);
    out
}

struct LabelTable {
    spans: Vec<ByteSpan>,
}

impl LabelTable {
    fn new() -> Self {
        Self { spans: Vec::new() }
    }

    fn fill(&mut self, asm: &str) {
        self.spans.clear();
        let bound = asm.len() / LOCAL_LABEL_MIN_BYTES;
        let extra = bound.saturating_sub(self.spans.capacity());
        self.spans.reserve(extra);
        for line in kept_lines(asm) {
            discover_line(asm, line, &mut self.spans);
        }
    }

    fn ordinal(&self, asm: &str, name: &str) -> usize {
        self.spans
            .iter()
            .position(|span| span.slice(asm) == name)
            .expect("local label")
    }
}

fn append_normalized(asm: &str, labels: &mut LabelTable, out: &mut String) {
    labels.fill(asm);
    let mut first = true;
    for line in kept_lines(asm) {
        if !first {
            out.push('\n');
        }
        first = false;
        write_canonical(asm, line, labels, out);
    }
}

fn kept_lines(asm: &str) -> impl Iterator<Item = &str> {
    asm.lines().filter_map(|line| {
        let trimmed = line.split('#').next().unwrap_or(line).trim();
        if trimmed.is_empty() || trimmed.starts_with('.') || trimmed.starts_with('#') {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn discover_line(asm: &str, line: &str, spans: &mut Vec<ByteSpan>) {
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if is_local_label_at(bytes, index) {
            let start = index;
            index += LOCAL_PREFIX.len();
            while index < bytes.len() && is_name_byte(bytes[index]) {
                index += 1;
            }
            let name = &line[start..index];
            if name.len() > LOCAL_PREFIX.len() {
                if spans.iter().all(|span| span.slice(asm) != name) {
                    spans.push(ByteSpan::covering(asm, name));
                }
                continue;
            }
            index = start;
        }
        let ch = line[index..].chars().next().expect("index is in range");
        index += ch.len_utf8();
    }
}

fn write_canonical(asm: &str, line: &str, labels: &LabelTable, out: &mut String) {
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if is_local_label_at(bytes, index) {
            let start = index;
            index += LOCAL_PREFIX.len();
            while index < bytes.len() && is_name_byte(bytes[index]) {
                index += 1;
            }
            let name = &line[start..index];
            if name.len() > LOCAL_PREFIX.len() {
                out.push('L');
                push_usize(out, labels.ordinal(asm, name));
                continue;
            }
            index = start;
        }
        let ch = line[index..].chars().next().expect("index is in range");
        out.push(ch);
        index += ch.len_utf8();
    }
}

fn is_local_label_at(bytes: &[u8], index: usize) -> bool {
    let prefix = LOCAL_PREFIX.as_bytes();
    index + prefix.len() <= bytes.len()
        && bytes[index..].starts_with(prefix)
        && (index == 0 || !is_name_byte(bytes[index - 1]))
}

fn push_usize(out: &mut String, value: usize) {
    let mut buf = [0u8; DECIMAL_DIGITS];
    let mut n = value;
    let mut index = buf.len();
    loop {
        index -= 1;
        buf[index] = b'0' + u8::try_from(n % 10).unwrap_or(0);
        n /= 10;
        if n == 0 {
            break;
        }
    }
    let digits = std::str::from_utf8(&buf[index..]).unwrap_or("0");
    out.push_str(digits);
}

fn is_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'$' | b'@' | b'%')
}
