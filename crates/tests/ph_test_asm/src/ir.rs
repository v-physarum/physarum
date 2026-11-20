//! LLVM IR and assembly split into functions, loops, and checks.

use crate::error::{ErrorKind, Result};

/// LLVM IR and assembly of one release probe.
#[derive(Debug)]
pub(crate) struct Assembly {
    functions: Vec<Function>,
}

impl Assembly {
    pub(crate) fn parse(ir: &str, asm: &str) -> Self {
        let asm_functions = asm_functions(asm);
        let functions = ir_functions(ir, &asm_functions);
        Self { functions }
    }

    /// Defined functions, in module order.
    #[must_use]
    pub(crate) fn functions(&self) -> &[Function] {
        &self.functions
    }

    /// The `define` named `name`.
    ///
    /// # Errors
    ///
    /// Returns an error when the module has no such function.
    pub(crate) fn function(&self, name: &str) -> Result<&Function> {
        self.functions
            .iter()
            .find(|function| function.name == name)
            .ok_or_else(|| {
                ErrorKind::MissingFunction {
                    name: name.to_owned(),
                }
                .into()
            })
    }
}

/// One `define`d function.
#[derive(Debug)]
pub(crate) struct Function {
    name: String,
    ir: String,
    asm: String,
    signature: String,
    checks: Vec<Check>,
    loops: Vec<Loop>,
    hot: Option<usize>,
}

impl Function {
    /// Symbol name without `@`.
    #[must_use]
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// `define` text, including the body.
    #[must_use]
    pub(crate) fn ir(&self) -> &str {
        &self.ir
    }

    /// Assembly of this symbol.
    #[must_use]
    pub(crate) fn asm(&self) -> &str {
        &self.asm
    }

    /// Parameter list inside the function parentheses.
    #[must_use]
    pub(crate) fn signature(&self) -> &str {
        &self.signature
    }

    /// Panic and bounds checks in the function.
    #[must_use]
    pub(crate) fn checks(&self) -> &[Check] {
        &self.checks
    }

    /// Natural loops. [`Loop::asm`] is set only on the hot loop.
    #[must_use]
    pub(crate) fn loops(&self) -> &[Loop] {
        &self.loops
    }

    /// Innermost natural loop with the most instructions.
    ///
    /// # Errors
    ///
    /// Returns an error when the function has no loop.
    pub(crate) fn hot_loop(&self) -> Result<&Loop> {
        self.hot.map(|index| &self.loops[index]).ok_or_else(|| {
            ErrorKind::NoHotLoop {
                name: self.name.clone(),
            }
            .into()
        })
    }
}

/// One natural loop.
///
/// `asm` is the assembly backedge region when this is the hot loop, and empty otherwise.
#[derive(Debug)]
pub(crate) struct Loop {
    ir: String,
    asm: String,
    checks: Vec<Check>,
}

impl Loop {
    /// LLVM IR of the blocks in the loop.
    #[must_use]
    pub(crate) fn ir(&self) -> &str {
        &self.ir
    }

    /// Assembly of the hot loop, or empty when this loop is not the hot one.
    #[must_use]
    pub(crate) fn asm(&self) -> &str {
        &self.asm
    }

    /// Panic and bounds checks inside the loop.
    #[must_use]
    pub(crate) fn checks(&self) -> &[Check] {
        &self.checks
    }
}

/// A panic or bounds check call.
#[derive(Debug, Clone)]
pub(crate) struct Check {
    kind: CheckKind,
}

impl Check {
    /// Bounds check or other panic.
    #[must_use]
    pub(crate) const fn kind(&self) -> CheckKind {
        self.kind
    }
}

/// Kind of [`Check`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CheckKind {
    /// Slice or index bounds panic.
    Bounds,
    /// Any other panic or assert.
    Panic,
}

struct Block {
    label: String,
    lines: Vec<String>,
}

struct RawFunction {
    name: String,
    signature: String,
    ir: String,
    blocks: Vec<Block>,
}

fn ir_functions(ir: &str, asm_functions: &[(String, String)]) -> Vec<Function> {
    raw_functions(ir)
        .into_iter()
        .map(|raw| finish_function(raw, asm_functions))
        .collect()
}

fn finish_function(raw: RawFunction, asm_functions: &[(String, String)]) -> Function {
    let asm = asm_functions
        .iter()
        .find(|(name, _)| name == &raw.name)
        .map_or_default(|(_, text)| text.clone());
    let checks = checks_in(
        &raw.blocks
            .iter()
            .flat_map(|block| block.lines.clone())
            .collect::<Vec<_>>(),
    );
    let mut loops = build_loops(&raw.blocks);
    let hot = hottest(&loops);
    if let Some(index) = hot {
        loops[index].asm = hot_asm(&asm);
    }
    Function {
        name: raw.name,
        ir: raw.ir,
        asm,
        signature: raw.signature,
        checks,
        loops,
        hot,
    }
}

fn checks_in(lines: &[String]) -> Vec<Check> {
    let mut found = Vec::new();
    let mut comment = String::new();
    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with(';') {
            trimmed.clone_into(&mut comment);
            continue;
        }
        let Some(inst) = Inst::parse(trimmed) else {
            continue;
        };
        if inst.opcode != "call" && inst.opcode != "invoke" {
            comment.clear();
            continue;
        }
        let Some(callee) = callee(trimmed) else {
            comment.clear();
            continue;
        };
        let haystack = format!("{callee} {comment}");
        if let Some(kind) = check_kind(&haystack) {
            found.push(Check { kind });
        }
        comment.clear();
    }
    found
}

fn check_kind(haystack: &str) -> Option<CheckKind> {
    const BOUNDS: &[&str] = &[
        "panic_bounds_check",
        "slice_index_len_fail",
        "slice_start_index_len_fail",
        "slice_end_index_len_fail",
    ];
    if BOUNDS.iter().any(|marker| haystack.contains(marker)) {
        return Some(CheckKind::Bounds);
    }
    if haystack.contains("panicking")
        || haystack.contains("panic_")
        || haystack.contains("assert_failed")
    {
        return Some(CheckKind::Panic);
    }
    None
}

fn build_loops(blocks: &[Block]) -> Vec<Loop> {
    let bodies = natural_loops(blocks);
    let mut loops = Vec::new();
    for body in bodies {
        let mut lines = Vec::new();
        let mut ir_lines = Vec::new();
        for index in &body {
            let block = &blocks[*index];
            ir_lines.push(format!("{}:", block.label));
            ir_lines.extend(block.lines.iter().cloned());
            lines.extend(block.lines.iter().cloned());
        }
        loops.push(Loop {
            ir: ir_lines.join("\n"),
            asm: String::new(),
            checks: checks_in(&lines),
        });
    }
    loops
}

fn hottest(loops: &[Loop]) -> Option<usize> {
    if loops.is_empty() {
        return None;
    }
    let innermost: Vec<usize> = loops
        .iter()
        .enumerate()
        .filter(|(index, _)| !is_outer(*index, loops))
        .map(|(index, _)| index)
        .collect();
    innermost
        .into_iter()
        .max_by_key(|&index| instruction_count(&loops[index].ir))
}

fn is_outer(index: usize, loops: &[Loop]) -> bool {
    let labels = labels_of(&loops[index].ir);
    loops.iter().enumerate().any(|(other, _)| {
        if other == index {
            return false;
        }
        let inner = labels_of(&loops[other].ir);
        inner.len() < labels.len() && inner.iter().all(|label| labels.contains(label))
    })
}

fn labels_of(ir: &str) -> Vec<String> {
    ir.lines()
        .filter_map(|line| label_name(line.trim()))
        .collect()
}

fn instruction_count(ir: &str) -> usize {
    ir.lines()
        .filter(|line| Inst::parse(line).is_some())
        .count()
}

fn natural_loops(blocks: &[Block]) -> Vec<Vec<usize>> {
    let count = blocks.len();
    if count == 0 {
        return Vec::new();
    }
    let successors = successor_indexes(blocks);
    let dominated = dominators(&successors);
    let predecessors = predecessors(&successors);
    let mut found = Vec::new();
    for (tail, edges) in successors.iter().enumerate() {
        for &header in edges {
            if header < count && dominated[tail][header] {
                let body = natural_loop(header, tail, &predecessors);
                if !found.contains(&body) {
                    found.push(body);
                }
            }
        }
    }
    found
}

fn successor_indexes(blocks: &[Block]) -> Vec<Vec<usize>> {
    blocks
        .iter()
        .map(|block| {
            label_uses(&block.lines)
                .into_iter()
                .filter_map(|name| blocks.iter().position(|other| other.label == name))
                .collect()
        })
        .collect()
}

fn predecessors(successors: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut preds = vec![Vec::new(); successors.len()];
    for (tail, edges) in successors.iter().enumerate() {
        for &header in edges {
            if header < preds.len() {
                preds[header].push(tail);
            }
        }
    }
    preds
}

fn dominators(successors: &[Vec<usize>]) -> Vec<Vec<bool>> {
    let count = successors.len();
    let preds = predecessors(successors);
    let mut dominated = vec![vec![true; count]; count];
    for (node, slot) in dominated[0].iter_mut().enumerate() {
        *slot = node == 0;
    }
    let mut changed = true;
    while changed {
        changed = false;
        for node in 1..count {
            let next = dominate_node(node, &preds, &dominated);
            if next != dominated[node] {
                dominated[node] = next;
                changed = true;
            }
        }
    }
    dominated
}

fn dominate_node(node: usize, preds: &[Vec<usize>], dominated: &[Vec<bool>]) -> Vec<bool> {
    let count = dominated.len();
    let mut next = vec![false; count];
    if preds[node].is_empty() {
        next[node] = true;
        return next;
    }
    next.clone_from(&dominated[preds[node][0]]);
    for &pred in &preds[node][1..] {
        for dominator in 0..count {
            next[dominator] &= dominated[pred][dominator];
        }
    }
    next[node] = true;
    next
}

fn natural_loop(header: usize, tail: usize, preds: &[Vec<usize>]) -> Vec<usize> {
    let mut body = vec![header];
    let mut stack = Vec::new();
    if tail != header {
        body.push(tail);
        stack.push(tail);
    }
    while let Some(node) = stack.pop() {
        for &pred in &preds[node] {
            if !body.contains(&pred) {
                body.push(pred);
                stack.push(pred);
            }
        }
    }
    body.sort_unstable();
    body
}

fn raw_functions(ir: &str) -> Vec<RawFunction> {
    let lines: Vec<&str> = ir.lines().collect();
    let mut functions = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        if !lines[index].starts_with("define ") {
            index += 1;
            continue;
        }
        let (header, next) = take_header(&lines, index);
        let (body, after) = take_body(&lines, next);
        if let Some(function) = raw_function(&header, &body) {
            functions.push(function);
        }
        index = after;
    }
    functions
}

fn take_header(lines: &[&str], start: usize) -> (String, usize) {
    let mut header = String::new();
    let mut index = start;
    while index < lines.len() {
        if !header.is_empty() {
            header.push('\n');
        }
        header.push_str(lines[index]);
        let opens = lines[index].contains('{');
        index += 1;
        if opens {
            break;
        }
    }
    (header, index)
}

fn take_body(lines: &[&str], start: usize) -> (Vec<String>, usize) {
    let mut body = Vec::new();
    let mut index = start;
    while index < lines.len() {
        if lines[index].trim() == "}" {
            return (body, index + 1);
        }
        body.push(lines[index].to_owned());
        index += 1;
    }
    (body, index)
}

fn raw_function(header: &str, body: &[String]) -> Option<RawFunction> {
    let name = symbol_name(header)?;
    let signature = signature_text(header);
    let mut ir = header.to_owned();
    if !ir.ends_with('\n') {
        ir.push('\n');
    }
    for line in body {
        ir.push_str(line);
        ir.push('\n');
    }
    ir.push_str("}\n");
    Some(RawFunction {
        name,
        signature,
        ir,
        blocks: split_blocks(body),
    })
}

fn symbol_name(header: &str) -> Option<String> {
    let at = header.find('@')?;
    let rest = &header[at + 1..];
    if let Some(quoted) = rest.strip_prefix('"') {
        let end = quoted.find('"')?;
        return Some(quoted[..end].to_owned());
    }
    let end = rest
        .find(|ch: char| !is_name_char(ch))
        .unwrap_or(rest.len());
    Some(rest[..end].to_owned())
}

fn signature_text(header: &str) -> String {
    let Some(at) = header.find('@') else {
        return String::new();
    };
    let Some(relative) = header[at..].find('(') else {
        return String::new();
    };
    let start = at + relative;
    let mut depth = 0i32;
    for (offset, ch) in header[start..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return header[start + 1..start + offset].to_owned();
                }
            }
            _ => {}
        }
    }
    String::new()
}

fn split_blocks(body: &[String]) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut label = String::new();
    let mut lines = Vec::new();
    let mut open = false;
    for line in body {
        let trimmed = line.trim();
        if trimmed.starts_with(';') || trimmed.is_empty() {
            continue;
        }
        // `name: ; preds = %entry` is a label. The predecessor note sits on the same line.
        if let Some(name) = label_name(without_comment(trimmed)) {
            if open {
                blocks.push(Block { label, lines });
            }
            label = name;
            lines = Vec::new();
            open = true;
            continue;
        }
        if open {
            lines.push(line.clone());
        }
    }
    if open {
        blocks.push(Block { label, lines });
    }
    blocks
}

fn without_comment(line: &str) -> &str {
    line.split(';').next().unwrap_or(line).trim()
}

fn label_name(trimmed: &str) -> Option<String> {
    let name = trimmed.strip_suffix(':')?;
    if name.is_empty() || name.contains(char::is_whitespace) {
        return None;
    }
    Some(name.to_owned())
}

fn label_uses(lines: &[String]) -> Vec<String> {
    let mut names = Vec::new();
    for line in lines {
        let mut rest = line.as_str();
        while let Some(index) = rest.find("label %") {
            let after = &rest[index + "label %".len()..];
            let name = if let Some(quoted) = after.strip_prefix('"') {
                let end = quoted.find('"').unwrap_or(quoted.len());
                quoted[..end].to_owned()
            } else {
                let end = after
                    .find(|ch: char| !is_name_char(ch))
                    .unwrap_or(after.len());
                after[..end].to_owned()
            };
            if !name.is_empty() {
                names.push(name);
            }
            rest = &after[1.min(after.len())..];
        }
    }
    names
}

fn is_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == '$'
}

struct Inst<'a> {
    opcode: &'a str,
}

impl<'a> Inst<'a> {
    fn parse(line: &'a str) -> Option<Self> {
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
            return Some(Self { opcode: token });
        }
    }
}

fn callee(line: &str) -> Option<&str> {
    let bytes = line.as_bytes();
    let mut found = None;
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
            found = Some(token);
            break;
        }
    }
    found
}

fn is_symbol_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'$' | b'@' | b'%')
}

fn asm_functions(asm: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = asm.lines().collect();
    let mut symbols = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if let Some(name) = function_type_symbol(line) {
            symbols.push((name, index));
        }
    }
    let mut functions = Vec::new();
    for (ordinal, (name, index)) in symbols.iter().enumerate() {
        let Some(start) = lines
            .iter()
            .skip(index + 1)
            .position(|line| line.trim() == format!("{name}:"))
        else {
            continue;
        };
        let start = index + 1 + start;
        let end = symbols
            .get(ordinal + 1)
            .map_or(lines.len(), |(_, next)| *next);
        let text = lines[start..end].join("\n");
        functions.push((name.clone(), text));
    }
    functions
}

fn function_type_symbol(line: &str) -> Option<String> {
    let compact: String = line.chars().filter(|ch| !ch.is_whitespace()).collect();
    let rest = compact.strip_prefix(".type")?;
    let name = rest.strip_suffix(",@function")?;
    if name.is_empty() {
        None
    } else {
        Some(name.to_owned())
    }
}

fn hot_asm(function_asm: &str) -> String {
    let lines: Vec<&str> = function_asm.lines().collect();
    let mut labels = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if let Some(name) = label_name(line.trim()) {
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
        let weight = region
            .lines()
            .filter(|line| is_asm_instruction(line))
            .count();
        let replace = best.is_none_or(|(_, _, best_weight)| weight > best_weight);
        if replace {
            best = Some((*label_at, index, weight));
        }
    }
    best.map_or_default(|(start, end, _)| lines[start..=end].join("\n"))
}

fn branch_target(line: &str) -> Option<String> {
    if !is_asm_instruction(line) {
        return None;
    }
    let token = strip_asm_comment(line).split_whitespace().last()?;
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

fn is_asm_instruction(line: &str) -> bool {
    let trimmed = strip_asm_comment(line).trim();
    !trimmed.is_empty()
        && !trimmed.ends_with(':')
        && !trimmed.starts_with('.')
        && !trimmed.starts_with('#')
}

fn strip_asm_comment(line: &str) -> &str {
    line.split('#').next().unwrap_or(line)
}
