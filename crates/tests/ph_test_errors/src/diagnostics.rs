//! Parsed `rustc` JSON diagnostics.

use serde::Deserialize;

use crate::error::{Error, Result};

/// Error diagnostics from one debug `rustc` run.
///
/// Warnings, notes, help, and summaries such as "aborting due to" are omitted.
#[must_use]
#[derive(Debug)]
pub(crate) struct Diagnostics {
    pub(crate) messages: Vec<Diagnostic>,
}

/// One error diagnostic: its code, when `rustc` assigned one, and its message.
#[derive(Debug)]
pub(crate) struct Diagnostic {
    code: Option<ErrorCode>,
    message: String,
}

/// A compiler error code, such as `E0308`.
#[derive(Debug)]
pub(crate) struct ErrorCode(String);

impl Diagnostics {
    pub(crate) fn parse(stderr: &str) -> Result<Self> {
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
            if raw.level != Level::Error || raw.message.starts_with("aborting due to ") {
                continue;
            }
            messages.push(Diagnostic {
                code: raw.code.map(|code| ErrorCode::new(code.code)),
                message: raw.message,
            });
        }
        Ok(Self { messages })
    }
}

impl ErrorCode {
    fn new(code: String) -> Self {
        Self(code)
    }

    fn matches(&self, code: &str) -> bool {
        self.0 == code
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

impl Diagnostic {
    pub(crate) const NONE: &str = "(none)";
    const SEPARATOR: &str = ": ";

    /// This error has `code` and its message contains `description`.
    pub(crate) fn matches(&self, code: &str, description: &str) -> bool {
        self.code.as_ref().is_some_and(|item| item.matches(code))
            && self.message.contains(description)
    }

    /// Bytes of the failure text: each line is code, `": "`, message, with a
    /// newline between lines. An empty list prints `(none)`.
    pub(crate) fn arrived_len(messages: &[Self]) -> usize {
        if messages.is_empty() {
            return Self::NONE.len();
        }
        let lines: usize = messages.iter().map(Self::line_len).sum();
        lines + messages.len() - 1
    }

    fn line_len(&self) -> usize {
        self.code_text().len() + Self::SEPARATOR.len() + self.message.len()
    }

    pub(crate) fn push_line(&self, arrived: &mut String) {
        arrived.push_str(self.code_text());
        arrived.push_str(Self::SEPARATOR);
        arrived.push_str(&self.message);
    }

    fn code_text(&self) -> &str {
        self.code.as_ref().map_or(Self::NONE, ErrorCode::as_str)
    }
}

/// Severity of one `rustc` diagnostic.
///
/// The JSON field is the lowercase name. Any other string deserializes as
/// [`Level::Other`] and does not fail the parse.
#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Level {
    /// Compilation failed.
    Error,
    /// The probe compiled, with a warning.
    Warning,
    /// Extra context attached to another diagnostic.
    Note,
    /// A suggestion attached to another diagnostic.
    Help,
    /// A level this crate does not name, including a missing field.
    #[default]
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct RawDiagnostic {
    #[serde(default)]
    message: String,
    #[serde(default)]
    level: Level,
    code: Option<RawCode>,
    #[serde(rename = "$message_type")]
    message_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawCode {
    code: String,
}
