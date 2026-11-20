//! Assembly split into functions and hot loops.

use crate::error::{ErrorKind, Result};
use crate::scan;

/// Assembly of one release probe.
#[derive(Debug)]
pub(crate) struct Assembly {
    functions: Vec<Function>,
}

impl Assembly {
    pub(crate) fn parse(ir: &str, asm: &str) -> Self {
        let signatures = signatures(ir);
        let functions = asm_functions(asm)
            .into_iter()
            .map(|(name, body)| {
                let signature = signatures
                    .iter()
                    .find(|(symbol, _)| symbol == &name)
                    .map_or_default(|(_, text)| text.clone());
                Function {
                    hot: scan::hot_region(&body),
                    name,
                    asm: body,
                    signature,
                }
            })
            .collect();
        Self { functions }
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
            .ok_or_else(|| {
                ErrorKind::MissingFunction {
                    name: name.to_owned(),
                }
                .into()
            })
    }
}

/// One assembled function.
#[derive(Debug)]
pub(crate) struct Function {
    name: String,
    asm: String,
    signature: String,
    hot: Option<String>,
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

    /// Parameter list from the LLVM IR `define`, inside the parentheses.
    #[must_use]
    pub(crate) fn signature(&self) -> &str {
        &self.signature
    }

    /// Backedge region with the most instructions.
    ///
    /// # Errors
    ///
    /// Returns an error when the function has no backedge.
    pub(crate) fn hot_loop(&self) -> Result<&str> {
        self.hot.as_deref().ok_or_else(|| {
            ErrorKind::NoHotLoop {
                name: self.name.clone(),
            }
            .into()
        })
    }
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

fn signatures(ir: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = ir.lines().collect();
    let mut found = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        if lines[index].trim_start().starts_with(';') || !lines[index].starts_with("define ") {
            index += 1;
            continue;
        }
        let (header, next) = take_header(&lines, index);
        if let Some(name) = symbol_name(&header) {
            found.push((name, signature_text(&header)));
        }
        index = next;
    }
    found
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

fn is_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == '$'
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
