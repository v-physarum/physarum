//! Normalization for asm and LLVM IR equivalence.

use crate::ir::{Assembly, Function, Loop};

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

/// One function, labels and alloc symbols canonicalized.
pub(crate) fn function_text(function: &Function) -> String {
    format!(
        "define @{name}\n{ir}\nasm:\n{asm}",
        name = function.name(),
        ir = normalize_ir(function.ir()),
        asm = normalize_asm(function.asm())
    )
}

/// Hot loop text for one function.
pub(crate) fn loop_text(function: &Function, hot: &Loop) -> String {
    format!(
        "loop @{name}\n{ir}\nasm:\n{asm}",
        name = function.name(),
        ir = normalize_ir(hot.ir()),
        asm = normalize_asm(hot.asm())
    )
}

fn normalize_ir(ir: &str) -> String {
    let mut lines = Vec::new();
    for line in ir.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(';') {
            continue;
        }
        let tidy = tidy(&strip_metadata(trimmed));
        if !tidy.is_empty() {
            lines.push(tidy);
        }
    }
    canonicalize_labels(&canonicalize_allocs(&lines.join("\n")))
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

fn strip_metadata(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::new();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '!' {
            index = skip_metadata(&chars, index + 1);
            continue;
        }
        if chars[index] == '#' {
            let next = index + 1;
            if next < chars.len() && chars[next].is_ascii_digit() {
                index = next;
                while index < chars.len() && chars[index].is_ascii_digit() {
                    index += 1;
                }
                continue;
            }
        }
        out.push(chars[index]);
        index += 1;
    }
    out
}

fn skip_metadata(chars: &[char], mut index: usize) -> usize {
    if index < chars.len() && chars[index] == '{' {
        let mut depth = 1;
        index += 1;
        while index < chars.len() && depth > 0 {
            match chars[index] {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
            index += 1;
        }
        return index;
    }
    while index < chars.len() && is_meta_char(chars[index]) {
        index += 1;
    }
    index
}

fn is_meta_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '$')
}

fn tidy(line: &str) -> String {
    let mut text = String::new();
    let mut pending_space = false;
    for ch in line.chars() {
        if ch.is_whitespace() {
            pending_space = !text.is_empty();
            continue;
        }
        if pending_space {
            text.push(' ');
            pending_space = false;
        }
        text.push(ch);
    }
    while text.ends_with(',') {
        text.pop();
    }
    text
}

fn canonicalize_allocs(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut names = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if text[index..].starts_with("@alloc_") {
            let start = index;
            index += "@alloc_".len();
            let hex = index;
            while index < bytes.len() && bytes[index].is_ascii_hexdigit() {
                index += 1;
            }
            let boundary = index == bytes.len() || !is_name_byte(bytes[index]);
            if index > hex && boundary {
                let name = text[start..index].to_owned();
                if !names.contains(&name) {
                    names.push(name);
                }
                continue;
            }
        }
        index += 1;
    }
    replace_in_order(text, &names, "@alloc_")
}

fn canonicalize_labels(text: &str) -> String {
    let mut names = Vec::new();
    for line in text.lines() {
        let Some(name) = line.strip_suffix(':') else {
            continue;
        };
        if name.is_empty()
            || name.contains(char::is_whitespace)
            || names.iter().any(|item| item == name)
        {
            continue;
        }
        names.push(name.to_owned());
    }
    let mut out = text.to_owned();
    let order = longest_first(&names);
    for (ordinal, name) in &order {
        let marker = marker(*ordinal);
        out = replace_exact_line(&out, &format!("{name}:"), &format!("{marker}:"));
        out = replace_bounded(&out, &format!("%{name}"), &format!("%{marker}"));
    }
    finish_markers(&out, names.len())
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

fn finish_markers(text: &str, count: usize) -> String {
    let mut out = text.to_owned();
    for ordinal in 0..count {
        out = out.replace(&marker(ordinal), &format!("bb{ordinal}"));
    }
    out
}

fn longest_first(names: &[String]) -> Vec<(usize, &String)> {
    let mut order: Vec<(usize, &String)> = names.iter().enumerate().collect();
    order.sort_by_key(|(_, name)| std::cmp::Reverse(name.len()));
    order
}

fn marker(ordinal: usize) -> String {
    format!("\u{E000}{ordinal}\u{E001}")
}

fn replace_exact_line(text: &str, from: &str, to: &str) -> String {
    text.lines()
        .map(|line| if line == from { to } else { line })
        .collect::<Vec<_>>()
        .join("\n")
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
