//! Normalization for assembly equivalence.

use crate::assembly::{Assembly, Function};

/// Whole module, functions sorted by symbol.
pub(crate) fn module_text(assembly: &Assembly) -> String {
    let mut functions: Vec<&Function> = assembly.functions().iter().collect();
    functions.sort_by(|left, right| left.name().cmp(right.name()));
    let mut cap = 0usize;
    for function in &functions {
        cap = cap.saturating_add(function.asm().len()).saturating_add(1);
    }
    let mut out = String::with_capacity(cap);
    for (index, function) in functions.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        append_normalized(function.asm(), &mut out);
    }
    out
}

/// One function, local labels canonicalized.
pub(crate) fn function_text(function: &Function) -> String {
    let mut out = String::with_capacity(function.asm().len());
    append_normalized(function.asm(), &mut out);
    out
}

/// Hot loop text for one function.
pub(crate) fn loop_text(hot: &str) -> String {
    let mut out = String::with_capacity(hot.len());
    append_normalized(hot, &mut out);
    out
}

fn append_normalized(asm: &str, out: &mut String) {
    let mut names: Vec<&str> = Vec::new();
    let mut first = true;
    for line in asm.lines() {
        let trimmed = line.split('#').next().unwrap_or(line).trim();
        if trimmed.is_empty() || trimmed.starts_with('.') || trimmed.starts_with('#') {
            continue;
        }
        if !first {
            out.push('\n');
        }
        first = false;
        write_canonical(trimmed, &mut names, out);
    }
}

fn write_canonical<'a>(line: &'a str, names: &mut Vec<&'a str>, out: &mut String) {
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if is_local_label_at(bytes, index) {
            let start = index;
            index += 2;
            while index < bytes.len() && is_name_byte(bytes[index]) {
                index += 1;
            }
            let name = &line[start..index];
            if name.len() > 2 {
                let ordinal = ordinal_of(names, name);
                out.push('L');
                push_usize(out, ordinal);
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
    index + 2 <= bytes.len()
        && bytes[index] == b'.'
        && bytes[index + 1] == b'L'
        && (index == 0 || !is_name_byte(bytes[index - 1]))
}

fn ordinal_of<'a>(names: &mut Vec<&'a str>, name: &'a str) -> usize {
    if let Some(ordinal) = names.iter().position(|existing| *existing == name) {
        return ordinal;
    }
    let ordinal = names.len();
    names.push(name);
    ordinal
}

fn push_usize(out: &mut String, value: usize) {
    let mut buf = [0u8; 20];
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
