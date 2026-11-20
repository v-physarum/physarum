//! Parsed `rustc` JSON diagnostics.

use serde::Deserialize;

use crate::error::{Error, Result};

/// Diagnostics from one debug `rustc` run.
///
/// Summaries such as "aborting due to" are omitted.
#[must_use]
#[derive(Debug, Clone)]
pub(crate) struct Diagnostics {
    ok: bool,
    messages: Vec<Diagnostic>,
}

/// One compiler diagnostic.
#[derive(Debug, Clone)]
pub(crate) struct Diagnostic {
    level: Level,
    code: Option<String>,
    message: String,
    children: Vec<Diagnostic>,
    rendered: String,
}

/// Severity of a [`Diagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Level {
    /// Compilation failed.
    Error,
    /// The probe compiled, with a warning.
    Warning,
    /// Extra context attached to another diagnostic.
    Note,
    /// A suggestion attached to another diagnostic.
    Help,
    /// A severity this crate does not name.
    Other,
}

impl Diagnostics {
    /// `rustc` exited successfully and emitted no errors.
    #[must_use]
    pub(crate) fn succeeded(&self) -> bool {
        self.ok && self.messages.iter().all(|item| item.level != Level::Error)
    }

    /// Error codes, in emission order.
    ///
    /// The iterator borrows the parsed diagnostics. It does not allocate.
    pub(crate) fn error_codes(&self) -> impl Iterator<Item = &str> {
        self.messages
            .iter()
            .filter(|item| item.level == Level::Error)
            .filter_map(|item| item.code.as_deref())
    }

    /// Error messages, in emission order.
    ///
    /// The iterator borrows the parsed diagnostics. It does not allocate.
    pub(crate) fn error_messages(&self) -> impl Iterator<Item = &str> {
        self.messages
            .iter()
            .filter(|item| item.level == Level::Error)
            .map(|item| item.message.as_str())
    }

    /// Diagnostics `rustc` emitted, in order.
    #[must_use]
    pub(crate) fn messages(&self) -> &[Diagnostic] {
        &self.messages
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

impl Diagnostic {
    /// Severity.
    #[must_use]
    pub(crate) const fn level(&self) -> Level {
        self.level
    }

    /// Compiler code such as `E0425`, when `rustc` assigned one.
    #[must_use]
    pub(crate) fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    /// Primary message without the rendered source listing.
    #[must_use]
    pub(crate) fn message(&self) -> &str {
        &self.message
    }

    /// Notes and help attached to this diagnostic.
    #[must_use]
    pub(crate) fn children(&self) -> &[Diagnostic] {
        &self.children
    }

    /// Text `rustc` would print for this diagnostic.
    #[must_use]
    pub(crate) fn rendered(&self) -> &str {
        &self.rendered
    }
}

pub(crate) fn parse(ok: bool, stderr: &str) -> Result<Diagnostics> {
    let mut messages = Vec::new();
    for line in stderr.lines() {
        let line = line.trim();
        if line.is_empty() || !line.starts_with('{') {
            continue;
        }
        let raw: RawDiagnostic =
            serde_json::from_str(line).map_err(|source| Error::Json { source })?;
        if raw
            .message_type
            .as_deref()
            .is_some_and(|kind| kind != "diagnostic")
        {
            continue;
        }
        if let Some(message) = convert(raw) {
            messages.push(message);
        }
    }
    Ok(Diagnostics { ok, messages })
}

fn convert(raw: RawDiagnostic) -> Option<Diagnostic> {
    if is_summary(&raw.level, &raw.message) {
        return None;
    }
    let children = raw.children.into_iter().filter_map(convert).collect();
    Some(Diagnostic {
        level: level(&raw.level),
        code: raw.code.map(|code| code.code),
        message: raw.message,
        children,
        rendered: raw.rendered.unwrap_or_default(),
    })
}

fn level(text: &str) -> Level {
    match text {
        "error" => Level::Error,
        "warning" => Level::Warning,
        "note" => Level::Note,
        "help" => Level::Help,
        _ => Level::Other,
    }
}

fn is_summary(level: &str, message: &str) -> bool {
    level == "failure-note"
        || message.starts_with("aborting due to ")
        || message.ends_with(" warning emitted")
        || message.ends_with(" warnings emitted")
}

#[derive(Debug, Deserialize)]
struct RawDiagnostic {
    #[serde(default)]
    message: String,
    #[serde(default)]
    level: String,
    code: Option<RawCode>,
    #[serde(default)]
    children: Vec<RawDiagnostic>,
    rendered: Option<String>,
    #[serde(rename = "$message_type")]
    message_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawCode {
    code: String,
}
