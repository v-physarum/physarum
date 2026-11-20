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
    pub(crate) code: Option<String>,
    pub(crate) message: String,
}

pub(crate) fn parse(stderr: &str) -> Result<Diagnostics> {
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
            code: raw.code.map(|code| code.code),
            message: raw.message,
        });
    }
    Ok(Diagnostics { messages })
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
