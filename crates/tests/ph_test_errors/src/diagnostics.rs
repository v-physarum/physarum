//! Parsed `rustc` JSON diagnostics.

use serde::Deserialize;

use crate::error::{ErrorKind, Result};

/// Diagnostics from one debug `rustc` run.
///
/// Summaries such as "aborting due to" are omitted.
#[must_use]
#[derive(Debug, Clone)]
pub struct Diagnostics {
    ok: bool,
    messages: Vec<Diagnostic>,
}

/// One compiler diagnostic.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    level: Level,
    code: Option<String>,
    message: String,
    spans: Vec<Span>,
    children: Vec<Diagnostic>,
    rendered: String,
}

/// Severity of a [`Diagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
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

/// A source span cited by a diagnostic.
#[derive(Debug, Clone)]
pub struct Span {
    file_name: String,
    line_start: u32,
    label: Option<String>,
    text: String,
}

impl Diagnostics {
    /// `rustc` exited successfully and emitted no errors.
    #[must_use]
    pub fn succeeded(&self) -> bool {
        self.ok && self.error_count() == 0
    }

    /// Error codes, in emission order.
    ///
    /// The iterator borrows the parsed diagnostics. It does not allocate.
    pub fn error_codes(&self) -> impl Iterator<Item = &str> {
        self.messages
            .iter()
            .filter(|item| item.level == Level::Error)
            .filter_map(|item| item.code.as_deref())
    }

    /// Error messages, in emission order.
    ///
    /// The iterator borrows the parsed diagnostics. It does not allocate.
    pub fn error_messages(&self) -> impl Iterator<Item = &str> {
        self.messages
            .iter()
            .filter(|item| item.level == Level::Error)
            .map(|item| item.message.as_str())
    }

    /// Diagnostics `rustc` emitted, in order.
    #[must_use]
    pub fn messages(&self) -> &[Diagnostic] {
        &self.messages
    }

    pub(crate) fn error_count(&self) -> usize {
        self.messages
            .iter()
            .filter(|item| item.level == Level::Error)
            .count()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

impl Diagnostic {
    /// Severity.
    #[must_use]
    pub const fn level(&self) -> Level {
        self.level
    }

    /// Compiler code such as `E0425`, when `rustc` assigned one.
    #[must_use]
    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    /// Primary message without the rendered source listing.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Spans cited by this diagnostic.
    #[must_use]
    pub fn spans(&self) -> &[Span] {
        &self.spans
    }

    /// Notes and help attached to this diagnostic.
    #[must_use]
    pub fn children(&self) -> &[Diagnostic] {
        &self.children
    }

    /// Text `rustc` would print for this diagnostic.
    #[must_use]
    pub fn rendered(&self) -> &str {
        &self.rendered
    }
}

impl Span {
    /// Path `rustc` reported.
    #[must_use]
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// First line of the span. Lines start at 1.
    #[must_use]
    pub const fn line_start(&self) -> u32 {
        self.line_start
    }

    /// Label printed under the span, when present.
    #[must_use]
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Source text `rustc` quoted for the span.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
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
            serde_json::from_str(line).map_err(|source| ErrorKind::Json { source })?;
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
        spans: raw.spans.into_iter().map(span).collect(),
        children,
        rendered: raw.rendered.unwrap_or_default(),
    })
}

fn span(raw: RawSpan) -> Span {
    let mut text = String::new();
    for (index, snippet) in raw.text.into_iter().enumerate() {
        if index > 0 {
            text.push('\n');
        }
        text.push_str(&snippet.text);
    }
    Span {
        file_name: raw.file_name,
        line_start: raw.line_start,
        label: raw.label,
        text,
    }
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
    spans: Vec<RawSpan>,
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

#[derive(Debug, Deserialize)]
struct RawSpan {
    file_name: String,
    line_start: u32,
    label: Option<String>,
    #[serde(default)]
    text: Vec<RawSnippet>,
}

#[derive(Debug, Deserialize)]
struct RawSnippet {
    text: String,
}
