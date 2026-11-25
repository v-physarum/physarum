//! Assembly split into functions and hot loops.

use crate::error::{ErrorKind, Result};
use crate::scan;
use crate::span::ByteSpan;
use crate::target::{MACH_O_SYMBOL_PREFIX, ObjectFormat, Target};

/// Assembly of one release probe.
#[derive(Debug)]
pub(crate) struct Assembly {
    functions: Vec<Function>,
    target: Target,
}

impl Assembly {
    pub(crate) fn parse(ir: &str, asm: &str, target: Target) -> Self {
        let signatures = signatures(ir);
        let raw = match target.format {
            ObjectFormat::Elf => elf_functions(asm),
            ObjectFormat::MachO => mach_o_functions(asm),
        };
        let functions = raw
            .into_iter()
            .map(|(name, body)| {
                let signature = signatures
                    .iter()
                    .find(|(symbol, _)| *symbol == name)
                    .map_or_default(|(_, text)| (*text).to_owned());
                let hot = scan::hot_span(&body, target.isa);
                Function { hot, name, asm: body, signature }
            })
            .collect();
        Self { functions, target }
    }

    /// Target the probe was compiled for.
    #[must_use]
    pub(crate) fn target(&self) -> Target {
        self.target
    }

    /// Removes the symbol named `name` and returns it.
    ///
    /// # Errors
    ///
    /// Returns an error when the module has no such function.
    pub(crate) fn into_function(mut self, name: &str) -> Result<Function> {
        let Some(index) = self
            .functions
            .iter()
            .position(|function| function.name == name)
        else {
            return Err(ErrorKind::MissingFunction { name: name.to_owned() }.into());
        };
        Ok(self.functions.swap_remove(index))
    }

    /// Defined functions, in module order.
    #[must_use]
    pub(crate) fn functions(&self) -> &[Function] {
        &self.functions
    }

    /// The symbol named `name`.
    ///
    /// # Errors
    ///
    /// Returns an error when the module has no such function.
    pub(crate) fn function(&self, name: &str) -> Result<&Function> {
        self.functions
            .iter()
            .find(|function| function.name == name)
            .ok_or_else(|| ErrorKind::MissingFunction { name: name.to_owned() }.into())
    }
}

/// One assembled function.
#[derive(Debug)]
pub(crate) struct Function {
    pub(crate) name: String,
    pub(crate) asm: String,
    pub(crate) signature: String,
    /// Hot loop inside `asm`.
    pub(crate) hot: Option<ByteSpan>,
}

impl Function {
    /// Symbol name.
    #[must_use]
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// Assembly of this symbol.
    #[must_use]
    pub(crate) fn asm(&self) -> &str {
        &self.asm
    }

    /// Backedge region with the most instructions.
    ///
    /// # Errors
    ///
    /// Returns an error when the function has no backedge.
    pub(crate) fn hot_loop(&self) -> Result<&str> {
        self.hot_bounds().map(|span| span.slice(&self.asm))
    }

    /// Hot loop inside [`Self::asm`].
    ///
    /// # Errors
    ///
    /// Returns an error when the function has no backedge.
    pub(crate) fn hot_bounds(&self) -> Result<ByteSpan> {
        self.hot
            .ok_or_else(|| ErrorKind::NoHotLoop { name: self.name.clone() }.into())
    }
}

const SHORTEST_TYPE_DIRECTIVE: &str = ".type a,@function";
const SHORTEST_MACH_O_LABEL: &str = "_a:";
const SHORTEST_DEFINE: &str = "define void @a(){";
const LABEL_COLON_LEN: usize = 1;
/// Prefixes of assembler-local labels on Mach-O (`Ltmp0`, `LBB0_1`, `lCPI0_0`).
const MACH_O_LOCAL_PREFIXES: [char; 2] = ['L', 'l'];
const SECTION_DIRECTIVE: &str = ".section";
const TEXT_DIRECTIVE: &str = ".text";
const MACH_O_TEXT_SECTION: &str = "__TEXT,__text";

fn elf_functions(asm: &str) -> Vec<(String, String)> {
    let mut symbols = Vec::with_capacity(asm.len() / SHORTEST_TYPE_DIRECTIVE.len());
    for (start, line) in scan::LineIter::new(asm) {
        if let Some(name) = function_type_symbol(line) {
            symbols.push((name, start));
        }
    }
    let mut functions = Vec::with_capacity(symbols.len());
    for (ordinal, (name, type_at)) in symbols.iter().enumerate() {
        let search_from = next_line_at(asm, *type_at);
        let Some(label_at) = find_label(asm, search_from, name) else {
            continue;
        };
        let end = symbols
            .get(ordinal + 1)
            .map_or(asm.len(), |(_, next)| *next);
        let body = asm[label_at..end].trim_end_matches(['\n', '\r']).to_owned();
        functions.push(((*name).to_owned(), body));
    }
    functions
}

fn mach_o_functions(asm: &str) -> Vec<(String, String)> {
    let mut starts = Vec::with_capacity(asm.len() / SHORTEST_MACH_O_LABEL.len());
    let mut in_text = false;
    for (start, line) in scan::LineIter::new(asm) {
        if let Some(text) = section_is_text(line) {
            in_text = text;
            starts.push((None, start));
            continue;
        }
        if in_text && let Some(name) = mach_o_symbol_label(line) {
            starts.push((Some(name), start));
        }
    }
    let mut functions = Vec::with_capacity(starts.len());
    for (ordinal, (name, label_at)) in starts.iter().enumerate() {
        let Some(name) = name else {
            continue;
        };
        let end = starts.get(ordinal + 1).map_or(asm.len(), |(_, next)| *next);
        let body = asm[*label_at..end]
            .trim_end_matches(['\n', '\r'])
            .to_owned();
        functions.push(((*name).to_owned(), body));
    }
    functions
}

/// `Some(true)` when `line` switches to the code section, `Some(false)` for
/// another section, `None` when it is not a section directive.
fn section_is_text(line: &str) -> Option<bool> {
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix(SECTION_DIRECTIVE) {
        return Some(rest.trim_start().starts_with(MACH_O_TEXT_SECTION));
    }
    (trimmed == TEXT_DIRECTIVE).then_some(true)
}

/// Rust name of the symbol `line` defines, without the Mach-O prefix.
fn mach_o_symbol_label(line: &str) -> Option<&str> {
    if line.starts_with(char::is_whitespace) {
        return None;
    }
    let label = line.trim_end().strip_suffix(':')?;
    if label.starts_with(MACH_O_LOCAL_PREFIXES) || !label.chars().all(is_name_char) {
        return None;
    }
    label.strip_prefix(MACH_O_SYMBOL_PREFIX)
}

fn next_line_at(text: &str, line_start: usize) -> usize {
    match text[line_start..].find('\n') {
        Some(at) => line_start + at + 1,
        None => text.len(),
    }
}

fn find_label(asm: &str, from: usize, name: &str) -> Option<usize> {
    for (start, line) in scan::LineIter::new(&asm[from..]) {
        if is_label(line, name) {
            return Some(from + start);
        }
    }
    None
}

fn is_label(line: &str, name: &str) -> bool {
    let trimmed = line.trim();
    trimmed.len() == name.len() + LABEL_COLON_LEN
        && trimmed.starts_with(name)
        && trimmed.ends_with(':')
}

fn signatures(ir: &str) -> Vec<(&str, &str)> {
    let mut found = Vec::with_capacity(ir.len() / SHORTEST_DEFINE.len());
    let mut index = 0;
    while index < ir.len() {
        let (line, next) = one_line(ir, index);
        if !line.trim_start().starts_with(';') && line.starts_with("define ") {
            let (header, after) = take_header(ir, index);
            if let Some(name) = symbol_name(header) {
                found.push((name, signature_text(header)));
            }
            index = after;
            continue;
        }
        index = next;
    }
    found
}

fn one_line(text: &str, start: usize) -> (&str, usize) {
    let rest = &text[start..];
    if let Some(at) = rest.find('\n') {
        let mut end = at;
        if end > 0 && rest.as_bytes()[end - 1] == b'\r' {
            end -= 1;
        }
        (&rest[..end], start + at + 1)
    } else {
        (rest, text.len())
    }
}

fn take_header(ir: &str, start: usize) -> (&str, usize) {
    let mut index = start;
    let mut end;
    loop {
        let (line, next) = one_line(ir, index);
        end = index + line.len();
        let opens = line.contains('{');
        index = next;
        if opens || index >= ir.len() {
            break;
        }
    }
    (&ir[start..end], index)
}

fn symbol_name(header: &str) -> Option<&str> {
    let at = header.find('@')?;
    let rest = &header[at + 1..];
    if let Some(quoted) = rest.strip_prefix('"') {
        let end = quoted.find('"')?;
        return Some(&quoted[..end]);
    }
    let end = rest
        .find(|ch: char| !is_name_char(ch))
        .unwrap_or(rest.len());
    Some(&rest[..end])
}

fn signature_text(header: &str) -> &str {
    let Some(at) = header.find('@') else {
        return "";
    };
    let Some(relative) = header[at..].find('(') else {
        return "";
    };
    let start = at + relative;
    let mut depth = 0i32;
    for (offset, ch) in header[start..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return &header[start + 1..start + offset];
                }
            }
            _ => {}
        }
    }
    ""
}

fn is_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == '$'
}

fn function_type_symbol(line: &str) -> Option<&str> {
    let rest = line.trim_start().strip_prefix(".type")?.trim_start();
    let (name, after) = rest.split_once(',')?;
    let name = name.trim();
    if name.is_empty() || name.chars().any(char::is_whitespace) {
        return None;
    }
    let after = after.split('#').next().unwrap_or(after).trim();
    (after == "@function").then_some(name)
}
