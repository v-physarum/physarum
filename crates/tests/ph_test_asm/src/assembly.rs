//! Assembly split into functions and hot loops.

use crate::error::{ErrorKind, Result};
use crate::scan;

/// Assembly of one release probe.
#[derive(Debug)]
pub(crate) struct Assembly {
    functions: Vec<Function>,
}

impl Assembly {
    pub(crate) fn parse(asm: &str) -> Self {
        let functions = asm_functions(asm)
            .into_iter()
            .map(|(name, body)| Function {
                hot: scan::hot_region(&body),
                name,
                asm: body,
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
