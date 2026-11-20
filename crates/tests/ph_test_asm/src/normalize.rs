//! Normalization for assembly equivalence.

use crate::assembly::{Assembly, Function};

/// Whole module, functions sorted by symbol.
pub(crate) fn module_text(assembly: &Assembly) -> String {
    let mut functions: Vec<&Function> = assembly.functions().iter().collect();
    functions.sort_by(|left, right| left.name().cmp(right.name()));
    functions
        .into_iter()
        .map(function_text)
        .collect::<Vec<_>>()
        .join("\n")
}

/// One function, local labels canonicalized.
pub(crate) fn function_text(function: &Function) -> String {
    normalize_asm(function.asm())
}

/// Hot loop text for one function.
pub(crate) fn loop_text(hot: &str) -> String {
    normalize_asm(hot)
}

fn normalize_asm(asm: &str) -> String {
    let mut lines = Vec::new();
    for line in asm.lines() {
        let trimmed = line.split('#').next().unwrap_or(line).trim();
        if trimmed.is_empty() || trimmed.starts_with('.') || trimmed.starts_with('#') {
            continue;
        }
        lines.push(trimmed.to_owned());
    }
    canonicalize_local_labels(&lines.join("\n"))
}

fn canonicalize_local_labels(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut names = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if text[index..].starts_with(".L") {
            let start = index;
            index += 2;
            while index < bytes.len() && is_name_byte(bytes[index]) {
                index += 1;
            }
            let name = text[start..index].to_owned();
            if name.len() > 2 && !names.contains(&name) {
                names.push(name);
            }
            continue;
        }
        index += 1;
    }
    replace_in_order(text, &names, "L")
}

fn replace_in_order(text: &str, names: &[String], prefix: &str) -> String {
    let mut out = text.to_owned();
    let order = longest_first(names);
    for (ordinal, name) in &order {
        out = replace_bounded(&out, name, &marker(*ordinal));
    }
    let mut finished = out;
    for ordinal in 0..names.len() {
        finished = finished.replace(&marker(ordinal), &format!("{prefix}{ordinal}"));
    }
    finished
}

fn longest_first(names: &[String]) -> Vec<(usize, &String)> {
    let mut order: Vec<(usize, &String)> = names.iter().enumerate().collect();
    order.sort_by_key(|(_, name)| std::cmp::Reverse(name.len()));
    order
}

fn marker(ordinal: usize) -> String {
    format!("\u{E000}{ordinal}\u{E001}")
}

fn replace_bounded(text: &str, from: &str, to: &str) -> String {
    let bytes = text.as_bytes();
    let needle = from.as_bytes();
    if needle.is_empty() {
        return text.to_owned();
    }
    let mut out = String::new();
    let mut index = 0;
    while index < bytes.len() {
        let matched =
            index + needle.len() <= bytes.len() && &bytes[index..index + needle.len()] == needle;
        let before_ok = index == 0 || !is_name_byte(bytes[index - 1]);
        let after = index + needle.len();
        let after_ok = after >= bytes.len() || !is_name_byte(bytes[after]);
        if matched && before_ok && after_ok {
            out.push_str(to);
            index = after;
            continue;
        }
        out.push(char::from(bytes[index]));
        index += 1;
    }
    out
}

fn is_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'$' | b'@' | b'%')
}
