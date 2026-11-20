//! Instruction scans over one function or loop.

pub(crate) fn opcode(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with(';') {
        return None;
    }
    let code = match trimmed.split_once('=') {
        Some((lhs, rhs)) if !lhs.contains(';') => rhs.trim(),
        _ => trimmed,
    };
    let mut rest = code;
    loop {
        let token = rest.split_whitespace().next()?;
        if matches!(token, "tail" | "musttail" | "notail" | "volatile") {
            rest = rest[token.len()..].trim_start();
            continue;
        }
        return Some(token);
    }
}

pub(crate) fn callee(line: &str) -> Option<&str> {
    let bytes = line.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if *byte != b'(' || index == 0 {
            continue;
        }
        let mut start = index;
        while start > 0 && is_symbol_byte(bytes[start - 1]) {
            start -= 1;
        }
        if start == index {
            continue;
        }
        let token = &line[start..index];
        if token.starts_with('@') || token.starts_with('%') {
            return Some(token);
        }
    }
    None
}

pub(crate) fn code_lines(ir: &str) -> impl Iterator<Item = &str> {
    ir.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with(';'))
}

pub(crate) fn has_simd(ir: &str) -> bool {
    code_lines(ir).any(|line| {
        matches!(
            opcode(line),
            Some("insertelement" | "extractelement" | "shufflevector")
        ) || has_vector_type(line)
    })
}

pub(crate) fn conditional_branches(ir: &str) -> usize {
    code_lines(ir)
        .filter(|line| is_conditional_branch(line))
        .count()
}

pub(crate) fn has_indirect_call(ir: &str) -> bool {
    code_lines(ir).any(|line| {
        matches!(opcode(line), Some("call" | "invoke"))
            && callee(line).is_some_and(|name| name.starts_with('%'))
    })
}

pub(crate) fn has_opcode(ir: &str, expected: &str) -> bool {
    code_lines(ir).any(|line| opcode(line) == Some(expected))
}

pub(crate) fn count_opcode(ir: &str, expected: &str) -> usize {
    code_lines(ir)
        .filter(|line| opcode(line) == Some(expected))
        .count()
}

pub(crate) fn has_div(ir: &str) -> bool {
    code_lines(ir).any(|line| {
        matches!(
            opcode(line),
            Some("sdiv" | "udiv" | "fdiv" | "srem" | "urem" | "frem")
        )
    })
}

pub(crate) fn has_alloc(ir: &str) -> bool {
    code_lines(ir).any(|line| {
        callee(line).is_some_and(|name| {
            name.contains("__rust_alloc")
                || name.contains("__rust_dealloc")
                || name.contains("__rust_realloc")
        })
    })
}

pub(crate) fn has_atomic(ir: &str) -> bool {
    code_lines(ir).any(|line| match opcode(line) {
        Some("atomicrmw" | "cmpxchg" | "fence") => true,
        Some("load" | "store") => line.contains(" atomic"),
        _ => false,
    })
}

pub(crate) fn has_memcpy(ir: &str) -> bool {
    code_lines(ir).any(|line| {
        callee(line).is_some_and(|name| {
            name.contains("llvm.memcpy")
                || name.contains("llvm.memmove")
                || name.contains("llvm.memset")
        })
    })
}

pub(crate) fn has_i128(ir: &str) -> bool {
    code_lines(ir).any(|line| has_type_word(line, "i128"))
}

pub(crate) fn has_fma(ir: &str) -> bool {
    code_lines(ir).any(|line| {
        callee(line)
            .is_some_and(|name| name.contains("llvm.fma.") || name.contains("llvm.fmuladd."))
    })
}

pub(crate) fn has_libm(ir: &str) -> bool {
    const NAMES: &[&str] = &[
        "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "sinh", "cosh", "tanh", "exp",
        "exp2", "exp10", "log", "log2", "log10", "pow", "fmod", "cbrt", "hypot", "sqrt",
    ];
    code_lines(ir).any(|line| {
        callee(line).is_some_and(|name| {
            NAMES
                .iter()
                .any(|item| name.contains(&format!("llvm.{item}.")))
        })
    })
}

pub(crate) fn has_gather(ir: &str) -> bool {
    code_lines(ir)
        .any(|line| callee(line).is_some_and(|name| name.to_ascii_lowercase().contains("gather")))
}

pub(crate) fn has_alloca_in_loop(ir: &str) -> bool {
    code_lines(ir).any(|line| {
        opcode(line) == Some("alloca")
            || callee(line).is_some_and(|name| {
                name.contains("llvm.lifetime.start") || name.contains("llvm.stacksave")
            })
    })
}

pub(crate) fn is_counted_loop(ir: &str) -> bool {
    let phis = phi_names(ir);
    if phis.is_empty() || !has_opcode(ir, "icmp") {
        return false;
    }
    phis.iter().any(|phi| has_constant_step(ir, phi))
}

pub(crate) fn basic_blocks(ir: &str) -> usize {
    code_lines(ir).filter(|line| is_label(line)).count()
}

pub(crate) fn asm_instructions(asm: &str) -> usize {
    asm.lines().filter(|line| is_asm_instruction(line)).count()
}

pub(crate) fn params_are_noalias(signature: &str) -> bool {
    parameters(signature)
        .iter()
        .all(|param| !has_type_word(param, "ptr") || has_type_word(param, "noalias"))
}

fn is_conditional_branch(line: &str) -> bool {
    match opcode(line) {
        Some("switch" | "indirectbr") => true,
        Some("br") => {
            let Some(token) = opcode(line) else {
                return false;
            };
            let rest = line.trim();
            let Some(at) = rest.find(token) else {
                return false;
            };
            rest[at + token.len()..].trim_start().starts_with("i1")
        }
        _ => false,
    }
}

fn has_vector_type(line: &str) -> bool {
    let bytes = line.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if *byte != b'<' {
            continue;
        }
        let rest = line[index + 1..].trim_start();
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if digits.is_empty() {
            continue;
        }
        if rest[digits.len()..].trim_start().starts_with('x') {
            return true;
        }
    }
    false
}

fn phi_names(ir: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in code_lines(ir) {
        if opcode(line) != Some("phi") {
            continue;
        }
        let Some((lhs, _)) = line.split_once('=') else {
            continue;
        };
        let name = lhs.trim();
        if name.starts_with('%') && line.contains("phi i") {
            names.push(name.to_owned());
        }
    }
    names
}

fn has_constant_step(ir: &str, phi: &str) -> bool {
    code_lines(ir).any(|line| {
        matches!(opcode(line), Some("add" | "sub"))
            && has_token(line, phi)
            && has_integer_token(line)
    })
}

fn has_integer_token(line: &str) -> bool {
    line.split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | '(' | ')' | '[' | ']'))
        .any(is_integer)
}

fn is_integer(token: &str) -> bool {
    let digits = token.strip_prefix('-').unwrap_or(token);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn has_token(line: &str, token: &str) -> bool {
    let bytes = line.as_bytes();
    let needle = token.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let Some(found) = line[index..].find(token) else {
            return false;
        };
        let at = index + found;
        let after = at + needle.len();
        if after >= bytes.len() || !is_symbol_byte(bytes[after]) {
            return true;
        }
        index = at + 1;
    }
    false
}

fn is_label(line: &str) -> bool {
    let Some(name) = line.strip_suffix(':') else {
        return false;
    };
    !name.is_empty() && !name.contains(char::is_whitespace)
}

fn is_asm_instruction(line: &str) -> bool {
    let trimmed = line.split('#').next().unwrap_or(line).trim();
    !trimmed.is_empty()
        && !trimmed.ends_with(':')
        && !trimmed.starts_with('.')
        && !trimmed.starts_with('#')
}

fn parameters(signature: &str) -> Vec<&str> {
    let mut depth = 0i32;
    let mut start = 0usize;
    let mut params = Vec::new();
    for (index, ch) in signature.char_indices() {
        match ch {
            '(' | '[' | '<' | '{' => depth += 1,
            ')' | ']' | '>' | '}' => depth -= 1,
            ',' if depth == 0 => {
                params.push(signature[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    let last = signature[start..].trim();
    if !last.is_empty() {
        params.push(last);
    }
    params
}

fn has_type_word(text: &str, word: &str) -> bool {
    let bytes = text.as_bytes();
    let needle = word.as_bytes();
    if needle.is_empty() {
        return false;
    }
    let mut index = 0;
    while index + needle.len() <= bytes.len() {
        if &bytes[index..index + needle.len()] == needle {
            let before_ok = index == 0 || !is_symbol_byte(bytes[index - 1]);
            let after = index + needle.len();
            let after_ok = after == bytes.len() || !is_symbol_byte(bytes[after]);
            let not_sigil = index == 0 || (bytes[index - 1] != b'%' && bytes[index - 1] != b'@');
            if before_ok && after_ok && not_sigil {
                return true;
            }
        }
        index += 1;
    }
    false
}

fn is_symbol_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'$' | b'@' | b'%')
}
