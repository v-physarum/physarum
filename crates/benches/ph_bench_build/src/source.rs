//! Case templates on disk.

use std::fs;
use std::path::Path;

use crate::error::{Error, Result};

/// The kind and count parsed from a case name such as `flat_structs_1024`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaseName<'a> {
    kind: &'a str,
    count: u32,
}

impl<'a> CaseName<'a> {
    /// Splits `case` at the last `_` into a kind and a count.
    ///
    /// # Errors
    ///
    /// Returns an error when `case` has no `_` or the suffix is not an integer.
    pub fn parse(case: &'a str) -> Result<Self> {
        let (kind, count) = case
            .rsplit_once('_')
            .ok_or_else(|| Error::unknown_case(case))?;
        let count = count.parse().map_err(|_| Error::unknown_case(case))?;
        Ok(Self { kind, count })
    }

    /// The text before the last `_`.
    #[must_use]
    pub fn kind(self) -> &'a str {
        self.kind
    }

    /// The integer after the last `_`.
    #[must_use]
    pub fn count(self) -> u32 {
        self.count
    }
}

/// Header file, item file, and how many times to paste the item.
///
/// `INDEX` in the item is the paste index. `next`, when set, is the key replaced
/// with the following index.
pub struct Repeat<'a> {
    header: &'a Path,
    item: &'a Path,
    count: u32,
    next: Option<&'a str>,
}

impl<'a> Repeat<'a> {
    /// One repeated item under `header`.
    #[must_use]
    pub fn new(header: &'a Path, item: &'a Path, count: u32, next: Option<&'a str>) -> Self {
        Self {
            header,
            item,
            count,
            next,
        }
    }

    /// Reads the header and appends the item `count` times.
    ///
    /// # Errors
    ///
    /// Returns an error when a file cannot be read.
    pub fn load(&self) -> Result<String> {
        let mut source = fs::read_to_string(self.header)?;
        if !source.ends_with('\n') {
            source.push('\n');
        }
        if !source.ends_with("\n\n") {
            source.push('\n');
        }
        let item = fs::read_to_string(self.item)?;
        for index in 0..self.count {
            let mut line = item.replace("INDEX", &index.to_string());
            if let Some(next_key) = self.next {
                line = line.replace(next_key, &(index + 1).to_string());
            }
            source.push_str(&line);
        }
        Ok(source)
    }
}

/// Replaces keys in a template file.
pub trait Fill {
    /// Replaces each key in `pairs` inside this file.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read.
    fn fill(&self, pairs: &[(&str, &str)]) -> Result<String>;
}

impl Fill for Path {
    fn fill(&self, pairs: &[(&str, &str)]) -> Result<String> {
        let mut text = fs::read_to_string(self)?;
        for (key, value) in pairs {
            text = text.replace(key, value);
        }
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::CaseName;

    #[test]
    fn case_name_splits_on_the_last_underscore() {
        let name = CaseName::parse("flat_structs_1024").unwrap();
        assert_eq!(name.kind(), "flat_structs");
        assert_eq!(name.count(), 1024);
    }

    #[test]
    fn unknown_case_name_is_an_error() {
        let Err(error) = CaseName::parse("flat") else {
            panic!("parsed");
        };
        let error = error.to_string();
        assert!(error.contains("unknown case"), "{error}");
    }
}
