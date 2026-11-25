//! Instruction scans over one function or loop.
//!
//! Each scan reads the instruction set it is given. `x86_64` is AT&T syntax.
//! `AArch64` is the LLVM syntax for ELF and Mach-O.

use crate::bound::Count;
use crate::span::ByteSpan;
use crate::target::{Isa, MACH_O_SYMBOL_PREFIX, ObjectFormat, Target};

const LIBM: &[&str] = &[
    "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "sinh", "cosh", "tanh", "exp", "exp2",
    "exp10", "log", "log2", "log10", "pow", "fmod", "cbrt", "hypot", "sqrt",
];

/// `C` symbol of the fused multiply-add libm call.
const FMA_SYMBOL: &str = "fma";

/// `AArch64` branches on a condition or a register test.
const AARCH64_CONDITIONAL_BRANCH_PREFIXES: [&str; 2] = ["b.", "bc."];
const AARCH64_COMPARE_BRANCHES: [&str; 4] = ["cbz", "cbnz", "tbz", "tbnz"];
/// `AArch64` branches whose target is a register.
const AARCH64_INDIRECT_BRANCHES: [&str; 2] = ["br", "blr"];
/// Pointer-authenticated register branches (`braa`, `blraaz`, ...).
const AARCH64_AUTH_INDIRECT_PREFIXES: [&str; 2] = ["bra", "blra"];
const AARCH64_SELECTS: [&str; 10] = [
    "csel", "csinc", "csinv", "csneg", "cset", "csetm", "cinc", "cinv", "cneg", "fcsel",
];
const AARCH64_INTEGER_DIVS: [&str; 2] = ["sdiv", "udiv"];
const AARCH64_FLOAT_DIV_PREFIX: &str = "fdiv";
/// Read-modify-write atomics. Each one loads and stores.
const AARCH64_ATOMIC_RMW_PREFIXES: [&str; 18] = [
    "ldadd", "ldclr", "ldeor", "ldset", "ldsmax", "ldsmin", "ldumax", "ldumin", "stadd", "stclr",
    "steor", "stset", "stsmax", "stsmin", "stumax", "stumin", "swp", "cas",
];
/// Exclusive-monitor halves of an atomic loop.
const AARCH64_EXCLUSIVE_PREFIXES: [&str; 8] = [
    "ldxr", "ldaxr", "stxr", "stlxr", "ldxp", "ldaxp", "stxp", "stlxp",
];
const AARCH64_LOAD_PREFIX: &str = "ld";
const AARCH64_STORE_PREFIX: &str = "st";
const AARCH64_STEP_PREFIXES: [&str; 2] = ["add", "sub"];
/// Instructions that set flags or test a register for a loop exit.
const AARCH64_COMPARES: [&str; 11] = [
    "cmp", "cmn", "tst", "ccmp", "ccmn", "adds", "subs", "cbz", "cbnz", "tbz", "tbnz",
];
const AARCH64_STACK_REGISTERS: [&str; 4] = ["sp", "wsp", "x29", "fp"];
/// Vector register banks: `v0` (`NEON`), `q0` (128-bit), `z0` (`SVE`).
const AARCH64_VECTOR_BANKS: [char; 3] = ['v', 'q', 'z'];
const AARCH64_FMA_PREFIXES: [&str; 6] = ["fmadd", "fmsub", "fnmadd", "fnmsub", "fmla", "fmls"];

/// Byte span of the backedge region with the most instructions.
///
/// The span is into `asm`. Nothing is allocated.
pub(crate) fn hot_span(asm: &str, isa: Isa) -> Option<ByteSpan> {
    let mut best: Option<(ByteSpan, Count)> = None;
    for (start, line) in LineIter::new(asm) {
        let Some(target) = branch_target(line, isa) else {
            continue;
        };
        let Some(label_at) = label_before(asm, start, target, isa) else {
            continue;
        };
        let span = ByteSpan::new(label_at, start + line.len());
        let weight = asm_instructions(span.slice(asm), isa);
        if best.is_none_or(|(_, best_weight)| weight.get() > best_weight.get()) {
            best = Some((span, weight));
        }
    }
    best.map(|(span, _)| span)
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

pub(crate) fn has_simd(asm: &str, isa: Isa) -> bool {
    match isa {
        Isa::X86_64 => instructions(asm, isa)
            .any(|line| line.contains("%xmm") || line.contains("%ymm") || line.contains("%zmm")),
        Isa::Aarch64 => {
            instructions(asm, isa).any(|line| operand_words(line).any(is_aarch64_vector_register))
        }
    }
}

pub(crate) fn conditional_branches(asm: &str, isa: Isa) -> Count {
    Count::new(
        instructions(asm, isa)
            .filter(|line| mnemonic(line).is_some_and(|name| is_conditional_jump(name, isa)))
            .count(),
    )
}

pub(crate) fn has_indirect_call(asm: &str, isa: Isa) -> bool {
    instructions(asm, isa).any(|line| match isa {
        Isa::X86_64 => {
            mnemonic(line).is_some_and(|name| name.starts_with("call") || name.starts_with("jmp"))
                && line.contains("*%")
        }
        Isa::Aarch64 => mnemonic(line).is_some_and(|name| {
            AARCH64_INDIRECT_BRANCHES.contains(&name)
                || AARCH64_AUTH_INDIRECT_PREFIXES
                    .iter()
                    .any(|prefix| name.starts_with(prefix))
        }),
    })
}

pub(crate) fn has_conditional_move(asm: &str, isa: Isa) -> bool {
    instructions(asm, isa).any(|line| {
        mnemonic(line).is_some_and(|name| match isa {
            Isa::X86_64 => name.starts_with("cmov"),
            Isa::Aarch64 => AARCH64_SELECTS.contains(&name),
        })
    })
}

pub(crate) fn has_div(asm: &str, isa: Isa) -> bool {
    instructions(asm, isa).any(|line| {
        mnemonic(line).is_some_and(|name| match isa {
            Isa::X86_64 => name.starts_with("idiv") || name.starts_with("div"),
            Isa::Aarch64 => {
                AARCH64_INTEGER_DIVS.contains(&name) || name.starts_with(AARCH64_FLOAT_DIV_PREFIX)
            }
        })
    })
}

pub(crate) fn has_unwind(asm: &str) -> bool {
    asm.contains("_Unwind_Resume")
}

pub(crate) fn has_alloc(asm: &str) -> bool {
    asm.contains("__rust_alloc")
}

pub(crate) fn has_atomic(asm: &str, isa: Isa) -> bool {
    instructions(asm, isa).any(|line| match isa {
        Isa::X86_64 => {
            line.split_whitespace().next() == Some("lock")
                || mnemonic(line)
                    .is_some_and(|name| name.starts_with("xadd") || name.starts_with("cmpxchg"))
        }
        Isa::Aarch64 => mnemonic(line)
            .is_some_and(|name| is_aarch64_atomic_rmw(name) || is_aarch64_exclusive(name)),
    })
}

pub(crate) fn has_memcpy(asm: &str, isa: Isa) -> bool {
    instructions(asm, isa)
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

pub(crate) fn count_loads(asm: &str, isa: Isa) -> Count {
    Count::new(
        memory_ops(asm, isa)
            .filter(|access| access.is_load())
            .count(),
    )
}

pub(crate) fn count_stores(asm: &str, isa: Isa) -> Count {
    Count::new(
        memory_ops(asm, isa)
            .filter(|access| access.is_store())
            .count(),
    )
}

pub(crate) fn is_counted_loop(asm: &str, isa: Isa) -> bool {
    if !has_backedge(asm, isa) {
        return false;
    }
    let mut adds = false;
    let mut compares = false;
    for line in instructions(asm, isa) {
        let Some(name) = mnemonic(line) else {
            continue;
        };
        match isa {
            Isa::X86_64 => {
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
            Isa::Aarch64 => {
                if AARCH64_STEP_PREFIXES
                    .iter()
                    .any(|prefix| name.starts_with(prefix))
                {
                    adds = true;
                }
                if AARCH64_COMPARES.contains(&name) {
                    compares = true;
                }
            }
        }
    }
    adds && compares
}

pub(crate) fn has_stack_address(asm: &str, isa: Isa) -> bool {
    instructions(asm, isa).any(|line| match isa {
        Isa::X86_64 => line.contains("%rsp") || line.contains("%rbp"),
        Isa::Aarch64 => operand_words(line).any(|word| AARCH64_STACK_REGISTERS.contains(&word)),
    })
}

pub(crate) fn has_gather(asm: &str, isa: Isa) -> bool {
    instructions(asm, isa).any(|line| contains_ascii_ignore_case(line, "gather"))
}

pub(crate) fn has_libm(asm: &str, target: Target) -> bool {
    instructions(asm, target.isa).any(|line| {
        LIBM.iter()
            .any(|name| names_symbol(line, name, target.format))
    })
}

pub(crate) fn has_fma(asm: &str, target: Target) -> bool {
    instructions(asm, target.isa).any(|line| {
        names_symbol(line, FMA_SYMBOL, target.format)
            || mnemonic(line).is_some_and(|name| match target.isa {
                Isa::X86_64 => name.starts_with("fmadd") || name.starts_with("vfmadd"),
                Isa::Aarch64 => AARCH64_FMA_PREFIXES
                    .iter()
                    .any(|prefix| name.starts_with(prefix)),
            })
    })
}

pub(crate) fn asm_instructions(asm: &str, isa: Isa) -> Count {
    Count::new(asm.lines().filter(|line| is_instruction(line, isa)).count())
}

/// Every pointer parameter in an LLVM IR signature is `noalias`.
///
/// A signature with no pointer parameter passes.
pub(crate) fn params_are_noalias(signature: &str) -> bool {
    every_parameter(signature, |param| {
        !has_type_word(param, "ptr") || has_type_word(param, "noalias")
    })
}

fn has_backedge(asm: &str, isa: Isa) -> bool {
    hot_span(asm, isa).is_some()
}

/// How one instruction touches memory.
#[derive(Clone, Copy)]
enum Access {
    Load,
    Store,
    Both,
}

impl Access {
    fn is_load(self) -> bool {
        matches!(self, Self::Load | Self::Both)
    }

    fn is_store(self) -> bool {
        matches!(self, Self::Store | Self::Both)
    }
}

fn memory_ops(asm: &str, isa: Isa) -> impl Iterator<Item = Access> + '_ {
    instructions(asm, isa).filter_map(move |line| {
        let name = mnemonic(line)?;
        match isa {
            Isa::X86_64 => {
                if name.starts_with("lea")
                    || name.starts_with("call")
                    || name.starts_with("jmp")
                    || is_conditional_jump(name, isa)
                {
                    return None;
                }
                let operands = line[line.find(name)? + name.len()..].trim();
                operand_access(operands)
            }
            Isa::Aarch64 => aarch64_access(name),
        }
    })
}

/// `AArch64` memory access is named by the mnemonic: `ld*` loads, `st*` stores.
fn aarch64_access(name: &str) -> Option<Access> {
    if is_aarch64_atomic_rmw(name) {
        Some(Access::Both)
    } else if name.starts_with(AARCH64_LOAD_PREFIX) {
        Some(Access::Load)
    } else if name.starts_with(AARCH64_STORE_PREFIX) {
        Some(Access::Store)
    } else {
        None
    }
}

fn is_aarch64_atomic_rmw(name: &str) -> bool {
    AARCH64_ATOMIC_RMW_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

fn is_aarch64_exclusive(name: &str) -> bool {
    AARCH64_EXCLUSIVE_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

/// `v0`, `q3`, `z12`: a bank letter and a register number.
fn is_aarch64_vector_register(word: &str) -> bool {
    let mut chars = word.chars();
    chars
        .next()
        .is_some_and(|bank| AARCH64_VECTOR_BANKS.contains(&bank))
        && !chars.as_str().is_empty()
        && chars.all(|ch| ch.is_ascii_digit())
}

/// Alphanumeric words after the mnemonic of `line`.
fn operand_words(line: &str) -> impl Iterator<Item = &str> {
    line.split_once(char::is_whitespace)
        .map_or("", |(_, rest)| rest)
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
}

/// `line` calls or names the C symbol `name`.
///
/// ELF writes `name@PLT`. Mach-O writes `_name` as an operand.
fn names_symbol(line: &str, name: &str, format: ObjectFormat) -> bool {
    match format {
        ObjectFormat::Elf => contains_symbol_at(line, name),
        ObjectFormat::MachO => line
            .split(|ch: char| ch.is_whitespace() || ch == ',')
            .any(|word| word.strip_prefix(MACH_O_SYMBOL_PREFIX) == Some(name)),
    }
}

fn instructions(asm: &str, isa: Isa) -> impl Iterator<Item = &str> {
    asm.lines()
        .map(move |line| isa.strip_comment(line))
        .map(str::trim)
        .filter(move |line| is_instruction(line, isa))
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

fn is_conditional_jump(name: &str, isa: Isa) -> bool {
    match isa {
        Isa::X86_64 => name.starts_with('j') && !name.starts_with("jmp"),
        Isa::Aarch64 => {
            AARCH64_COMPARE_BRANCHES.contains(&name)
                || AARCH64_CONDITIONAL_BRANCH_PREFIXES
                    .iter()
                    .any(|prefix| name.starts_with(prefix))
        }
    }
}

pub(crate) fn is_instruction(line: &str, isa: Isa) -> bool {
    let trimmed = isa.strip_comment(line).trim();
    !trimmed.is_empty()
        && !trimmed.ends_with(':')
        && !trimmed.starts_with('.')
        && !trimmed.starts_with('#')
}

fn label_before(asm: &str, before: usize, name: &str, isa: Isa) -> Option<usize> {
    for (start, line) in LineIter::new(asm) {
        if start >= before {
            break;
        }
        if label_name(isa.strip_comment(line).trim()) == Some(name) {
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

fn branch_target(line: &str, isa: Isa) -> Option<&str> {
    if !is_instruction(line, isa) {
        return None;
    }
    let token = isa.strip_comment(line).split_whitespace().last()?;
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

/// Independent facts about the operands seen so far.
#[derive(Clone, Copy, Default)]
struct OperandBits(u8);

impl OperandBits {
    const SEEN: u8 = 1 << 0;
    const PRIOR_LOAD: u8 = 1 << 1;
    const LAST_MEMORY: u8 = 1 << 2;

    fn note(mut self, memory: bool) -> Self {
        if self.has(Self::SEEN) && self.has(Self::LAST_MEMORY) {
            self.0 |= Self::PRIOR_LOAD;
        }
        self.0 |= Self::SEEN;
        if memory {
            self.0 |= Self::LAST_MEMORY;
        } else {
            self.0 &= !Self::LAST_MEMORY;
        }
        self
    }

    fn access(self) -> Option<Access> {
        if !self.has(Self::SEEN) {
            return None;
        }
        match (self.has(Self::PRIOR_LOAD), self.has(Self::LAST_MEMORY)) {
            (false, false) => None,
            (true, false) => Some(Access::Load),
            (false, true) => Some(Access::Store),
            (true, true) => Some(Access::Both),
        }
    }

    fn has(self, flag: u8) -> bool {
        self.0 & flag != 0
    }
}

fn operand_access(text: &str) -> Option<Access> {
    let mut depth = 0i32;
    let mut start = 0usize;
    let mut bits = OperandBits::default();
    for (index, ch) in text.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                bits = note_operand(text[start..index].trim(), bits);
                start = index + 1;
            }
            _ => {}
        }
    }
    note_operand(text[start..].trim(), bits).access()
}

fn note_operand(part: &str, bits: OperandBits) -> OperandBits {
    if part.is_empty() {
        return bits;
    }
    bits.note(part.contains('('))
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
