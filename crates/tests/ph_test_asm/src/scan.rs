//! Instruction scans over one function or loop.

const LIBM: &[&str] = &[
    "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "sinh", "cosh", "tanh", "exp", "exp2",
    "exp10", "log", "log2", "log10", "pow", "fmod", "cbrt", "hypot", "sqrt",
];

pub(crate) fn hot_region(asm: &str) -> Option<String> {
    let lines: Vec<&str> = asm.lines().collect();
    let mut labels = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if let Some(name) = label_name(strip_comment(line).trim()) {
            labels.push((name, index));
        }
    }
    let mut best: Option<(usize, usize, usize)> = None;
    for (index, line) in lines.iter().enumerate() {
        let Some(target) = branch_target(line) else {
            continue;
        };
        let Some((_, label_at)) = labels.iter().find(|(name, _)| name == &target) else {
            continue;
        };
        if *label_at >= index {
            continue;
        }
        let region = lines[*label_at..=index].join("\n");
        let weight = region.lines().filter(|line| is_instruction(line)).count();
        let replace = best.is_none_or(|(_, _, best_weight)| weight > best_weight);
        if replace {
            best = Some((*label_at, index, weight));
        }
    }
    best.map(|(start, end, _)| lines[start..=end].join("\n"))
}

pub(crate) fn has_simd(asm: &str) -> bool {
    instructions(asm)
        .any(|line| line.contains("%xmm") || line.contains("%ymm") || line.contains("%zmm"))
}

pub(crate) fn conditional_branches(asm: &str) -> usize {
    instructions(asm)
        .filter(|line| mnemonic(line).is_some_and(is_conditional_jump))
        .count()
}

pub(crate) fn has_indirect_call(asm: &str) -> bool {
    instructions(asm).any(|line| {
        mnemonic(line).is_some_and(|name| name.starts_with("call") || name.starts_with("jmp"))
            && line.contains("*%")
    })
}

pub(crate) fn has_conditional_move(asm: &str) -> bool {
    instructions(asm).any(|line| mnemonic(line).is_some_and(|name| name.starts_with("cmov")))
}

pub(crate) fn has_div(asm: &str) -> bool {
    instructions(asm).any(|line| {
        mnemonic(line).is_some_and(|name| name.starts_with("idiv") || name.starts_with("div"))
    })
}

pub(crate) fn has_unwind(asm: &str) -> bool {
    asm.contains("_Unwind_Resume")
}

pub(crate) fn has_alloc(asm: &str) -> bool {
    asm.contains("__rust_alloc")
}

pub(crate) fn has_atomic(asm: &str) -> bool {
    instructions(asm).any(|line| {
        line.split_whitespace().next() == Some("lock")
            || mnemonic(line)
                .is_some_and(|name| name.starts_with("xadd") || name.starts_with("cmpxchg"))
    })
}

pub(crate) fn has_memcpy(asm: &str) -> bool {
    instructions(asm)
        .any(|line| line.contains("memcpy") || line.contains("memmove") || line.contains("memset"))
}

pub(crate) fn has_panic(asm: &str) -> bool {
    asm.contains("panic_")
}

pub(crate) fn has_bounds(asm: &str) -> bool {
    asm.contains("panic_bounds_check")
        || asm.contains("slice_index_len_fail")
        || asm.contains("slice_start_index_len_fail")
        || asm.contains("slice_end_index_len_fail")
}

pub(crate) fn count_loads(asm: &str) -> usize {
    memory_ops(asm).filter(|op| op.load).count()
}

pub(crate) fn count_stores(asm: &str) -> usize {
    memory_ops(asm).filter(|op| op.store).count()
}

pub(crate) fn is_counted_loop(asm: &str) -> bool {
    if !has_backedge(asm) {
        return false;
    }
    let mut adds = false;
    let mut compares = false;
    for line in instructions(asm) {
        let Some(name) = mnemonic(line) else {
            continue;
        };
        if name.starts_with("add")
            || name.starts_with("sub")
            || name.starts_with("inc")
            || name.starts_with("dec")
            || name.starts_with("lea")
        {
            adds = true;
        }
        if name.starts_with("cmp") || name.starts_with("test") {
            compares = true;
        }
    }
    adds && compares
}

pub(crate) fn has_stack_address(asm: &str) -> bool {
    instructions(asm).any(|line| line.contains("%rsp") || line.contains("%rbp"))
}

pub(crate) fn has_gather(asm: &str) -> bool {
    instructions(asm).any(|line| line.to_ascii_lowercase().contains("gather"))
}

pub(crate) fn has_libm(asm: &str) -> bool {
    instructions(asm).any(|line| LIBM.iter().any(|name| line.contains(&format!("{name}@"))))
}

pub(crate) fn has_fma(asm: &str) -> bool {
    instructions(asm).any(|line| {
        line.contains("fma@")
            || mnemonic(line)
                .is_some_and(|name| name.starts_with("fmadd") || name.starts_with("vfmadd"))
    })
}

pub(crate) fn asm_instructions(asm: &str) -> usize {
    asm.lines().filter(|line| is_instruction(line)).count()
}

/// Every pointer parameter in an LLVM IR signature is `noalias`.
///
/// A signature with no pointer parameter passes.
pub(crate) fn params_are_noalias(signature: &str) -> bool {
    parameters(signature)
        .iter()
        .all(|param| !has_type_word(param, "ptr") || has_type_word(param, "noalias"))
}

fn has_backedge(asm: &str) -> bool {
    hot_region(asm).is_some()
}

struct MemoryOp {
    load: bool,
    store: bool,
}

fn memory_ops(asm: &str) -> impl Iterator<Item = MemoryOp> + '_ {
    instructions(asm).filter_map(|line| {
        let name = mnemonic(line)?;
        if name.starts_with("lea")
            || name.starts_with("call")
            || name.starts_with("jmp")
            || is_conditional_jump(name)
        {
            return None;
        }
        let operands = line[line.find(name)? + name.len()..].trim();
        let parts = split_operands(operands);
        if parts.is_empty() {
            return None;
        }
        let store = parts.last().is_some_and(|part| part.contains('('));
        let load = parts[..parts.len() - 1]
            .iter()
            .any(|part| part.contains('('));
        if !load && !store {
            return None;
        }
        Some(MemoryOp { load, store })
    })
}

fn instructions(asm: &str) -> impl Iterator<Item = &str> {
    asm.lines()
        .map(strip_comment)
        .map(str::trim)
        .filter(|line| is_instruction(line))
}

fn mnemonic(line: &str) -> Option<&str> {
    let mut tokens = line.split_whitespace();
    let first = tokens.next()?;
    if first == "lock" {
        tokens.next()
    } else {
        Some(first)
    }
}

fn is_conditional_jump(name: &str) -> bool {
    name.starts_with('j') && !name.starts_with("jmp")
}

pub(crate) fn is_instruction(line: &str) -> bool {
    let trimmed = strip_comment(line).trim();
    !trimmed.is_empty()
        && !trimmed.ends_with(':')
        && !trimmed.starts_with('.')
        && !trimmed.starts_with('#')
}

fn label_name(trimmed: &str) -> Option<String> {
    let name = trimmed.strip_suffix(':')?;
    if name.is_empty() || name.contains(char::is_whitespace) {
        return None;
    }
    Some(name.to_owned())
}

fn branch_target(line: &str) -> Option<String> {
    if !is_instruction(line) {
        return None;
    }
    let token = strip_comment(line).split_whitespace().last()?;
    let token = token.trim_matches(',');
    if token.starts_with('.')
        || token.starts_with('_')
        || token.starts_with(|ch: char| ch.is_ascii_alphabetic())
    {
        Some(token.to_owned())
    } else {
        None
    }
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

fn strip_comment(line: &str) -> &str {
    line.split('#').next().unwrap_or(line)
}

fn split_operands(text: &str) -> Vec<&str> {
    let mut depth = 0i32;
    let mut start = 0usize;
    let mut parts = Vec::new();
    for (index, ch) in text.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(text[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    let last = text[start..].trim();
    if !last.is_empty() {
        parts.push(last);
    }
    parts
}
