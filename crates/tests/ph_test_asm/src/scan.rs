//! Instruction scans over one function or loop.

const LIBM: &[&str] = &[
    "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "sinh", "cosh", "tanh", "exp", "exp2",
    "exp10", "log", "log2", "log10", "pow", "fmod", "cbrt", "hypot", "sqrt",
];

/// Byte span of the backedge region with the most instructions.
///
/// The span is into `asm`. Nothing is allocated.
pub(crate) fn hot_span(asm: &str) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize, usize)> = None;
    for (start, line) in LineIter::new(asm) {
        let Some(target) = branch_target(line) else {
            continue;
        };
        let Some(label_at) = label_before(asm, start, target) else {
            continue;
        };
        let end = start + line.len();
        let weight = asm_instructions(&asm[label_at..end]);
        if best.is_none_or(|(_, _, best_weight)| weight > best_weight) {
            best = Some((label_at, end, weight));
        }
    }
    best.map(|(start, end, _)| (start, end))
}

/// Lines of `text` with the byte offset of each line.
pub(crate) struct LineIter<'a> {
    text: &'a str,
    offset: usize,
}

impl<'a> LineIter<'a> {
    pub(crate) fn new(text: &'a str) -> Self {
        Self { text, offset: 0 }
    }
}

impl<'a> Iterator for LineIter<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.text.len() {
            return None;
        }
        let rest = &self.text[self.offset..];
        let (line, next) = if let Some(at) = rest.find('\n') {
            let mut end = at;
            if end > 0 && rest.as_bytes()[end - 1] == b'\r' {
                end -= 1;
            }
            (&rest[..end], self.offset + at + 1)
        } else {
            (rest, self.text.len())
        };
        let start = self.offset;
        self.offset = next;
        Some((start, line))
    }
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
    instructions(asm).any(|line| contains_ascii_ignore_case(line, "gather"))
}

pub(crate) fn has_libm(asm: &str) -> bool {
    instructions(asm).any(|line| LIBM.iter().any(|name| contains_symbol_at(line, name)))
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
    every_parameter(signature, |param| {
        !has_type_word(param, "ptr") || has_type_word(param, "noalias")
    })
}

fn has_backedge(asm: &str) -> bool {
    hot_span(asm).is_some()
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
        let (load, store) = memory_operands(operands)?;
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

fn label_before(asm: &str, before: usize, name: &str) -> Option<usize> {
    for (start, line) in LineIter::new(asm) {
        if start >= before {
            break;
        }
        if label_name(strip_comment(line).trim()) == Some(name) {
            return Some(start);
        }
    }
    None
}

fn label_name(trimmed: &str) -> Option<&str> {
    let name = trimmed.strip_suffix(':')?;
    if name.is_empty() || name.contains(char::is_whitespace) {
        return None;
    }
    Some(name)
}

fn branch_target(line: &str) -> Option<&str> {
    if !is_instruction(line) {
        return None;
    }
    let token = strip_comment(line).split_whitespace().last()?;
    let token = token.trim_matches(',');
    if token.starts_with('.')
        || token.starts_with('_')
        || token.starts_with(|ch: char| ch.is_ascii_alphabetic())
    {
        Some(token)
    } else {
        None
    }
}

fn every_parameter(signature: &str, mut accept: impl FnMut(&str) -> bool) -> bool {
    let mut depth = 0i32;
    let mut start = 0usize;
    for (index, ch) in signature.char_indices() {
        match ch {
            '(' | '[' | '<' | '{' => depth += 1,
            ')' | ']' | '>' | '}' => depth -= 1,
            ',' if depth == 0 => {
                if !accept(signature[start..index].trim()) {
                    return false;
                }
                start = index + 1;
            }
            _ => {}
        }
    }
    let last = signature[start..].trim();
    last.is_empty() || accept(last)
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

fn memory_operands(text: &str) -> Option<(bool, bool)> {
    let mut depth = 0i32;
    let mut start = 0usize;
    let mut saw = false;
    let mut earlier_load = false;
    let mut last_memory = false;
    for (index, ch) in text.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                note_operand(
                    text[start..index].trim(),
                    &mut saw,
                    &mut earlier_load,
                    &mut last_memory,
                );
                start = index + 1;
            }
            _ => {}
        }
    }
    note_operand(
        text[start..].trim(),
        &mut saw,
        &mut earlier_load,
        &mut last_memory,
    );
    if !saw || (!earlier_load && !last_memory) {
        return None;
    }
    Some((earlier_load, last_memory))
}

fn note_operand(part: &str, saw: &mut bool, earlier_load: &mut bool, last_memory: &mut bool) {
    if part.is_empty() {
        return;
    }
    if *saw {
        *earlier_load |= *last_memory;
    }
    *saw = true;
    *last_memory = part.contains('(');
}

fn contains_ascii_ignore_case(haystack: &str, needle: &str) -> bool {
    let haystack = haystack.as_bytes();
    let needle = needle.as_bytes();
    if needle.is_empty() || haystack.len() < needle.len() {
        return false;
    }
    haystack
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle))
}

fn contains_symbol_at(line: &str, name: &str) -> bool {
    let bytes = line.as_bytes();
    let name = name.as_bytes();
    if name.is_empty() || bytes.len() <= name.len() {
        return false;
    }
    bytes
        .windows(name.len() + 1)
        .any(|window| window[..name.len()] == *name && window[name.len()] == b'@')
}
