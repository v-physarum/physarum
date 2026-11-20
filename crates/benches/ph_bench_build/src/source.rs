//! Case templates on disk.

use std::fs;
use std::path::Path;

use crate::error::Result;

/// Header file, item file, and how many times to paste the item.
///
/// `INDEX` in the item is the paste index. `next`, when set, is the key replaced
/// with the following index. Each copy is written into the one output string.
pub(crate) struct Repeat<'a> {
    header: &'a Path,
    item: &'a Path,
    count: u32,
    next: Option<&'a str>,
}

impl<'a> Repeat<'a> {
    /// One repeated item under `header`.
    #[must_use]
    pub(crate) fn new(header: &'a Path, item: &'a Path, count: u32, next: Option<&'a str>) -> Self {
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
    pub(crate) fn load(&self) -> Result<String> {
        let mut source = fs::read_to_string(self.header)?;
        if !source.ends_with('\n') {
            source.push('\n');
        }
        if !source.ends_with("\n\n") {
            source.push('\n');
        }
        let item = fs::read_to_string(self.item)?;
        append_items(&mut source, &item, self.count, self.next);
        Ok(source)
    }
}

const INDEX_KEY: &str = "INDEX";
/// `u32` is at most this many decimal digits.
const DIGIT_CAP: usize = 10;

/// Appends `item` `count` times. One scratch buffer is reused when `next` is set.
fn append_items(source: &mut String, item: &str, count: u32, next: Option<&str>) {
    let pasted = expanded(item, INDEX_KEY);
    let per = match next {
        Some(key) => grown(pasted, key),
        None => pasted,
    };
    source.reserve(per.saturating_mul(count as usize));
    if let Some(key) = next {
        let mut scratch = String::new();
        scratch.reserve(pasted);
        for index in 0..count {
            scratch.clear();
            splice(&mut scratch, item, INDEX_KEY, index);
            splice(source, &scratch, key, index + 1);
        }
        return;
    }
    for index in 0..count {
        splice(source, item, INDEX_KEY, index);
    }
}

/// Upper bound on `text` after `key` is replaced by a decimal `u32`.
fn expanded(text: &str, key: &str) -> usize {
    if key.is_empty() {
        return grown(text.len(), key);
    }
    let hits = text.matches(key).count();
    text.len()
        .saturating_sub(key.len().saturating_mul(hits))
        .saturating_add(DIGIT_CAP.saturating_mul(hits))
}

/// Upper bound after a non-overlapping replace of `key` inside `len` bytes.
fn grown(len: usize, key: &str) -> usize {
    if key.is_empty() {
        return len.saturating_add(DIGIT_CAP.saturating_mul(len.saturating_add(1)));
    }
    let hits = len / key.len();
    len.saturating_add(DIGIT_CAP.saturating_mul(hits))
}

/// Writes `text` into `out`, replacing every `key` with the decimal `value`.
fn splice(out: &mut String, text: &str, key: &str, value: u32) {
    let digits = Digits::new(value);
    if key.is_empty() {
        digits.append_to(out);
        for ch in text.chars() {
            out.push(ch);
            digits.append_to(out);
        }
        return;
    }
    let mut rest = text;
    while let Some(at) = rest.find(key) {
        out.push_str(&rest[..at]);
        digits.append_to(out);
        rest = &rest[at + key.len()..];
    }
    out.push_str(rest);
}

/// Decimal digits of one `u32`, stored on the stack.
#[derive(Clone, Copy)]
struct Digits {
    bytes: [u8; DIGIT_CAP],
    at: usize,
}

impl Digits {
    fn new(mut value: u32) -> Self {
        let mut bytes = [0u8; DIGIT_CAP];
        let mut at = bytes.len();
        loop {
            at -= 1;
            bytes[at] = b"0123456789"[(value % 10) as usize];
            value /= 10;
            if value == 0 {
                break;
            }
        }
        Self { bytes, at }
    }

    fn append_to(self, out: &mut String) {
        for byte in &self.bytes[self.at..] {
            out.push(char::from(*byte));
        }
    }
}

/// Replaces keys in a template file.
pub(crate) trait Fill {
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
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::Repeat;

    #[test]
    fn repeat_writes_every_index_into_one_string() {
        let dir = TempDir::new();
        let header = dir.file("header.rs");
        let item = dir.file("item.rs");
        fs::write(&header, "use crate;\n").unwrap();
        fs::write(&item, "struct IxINDEX;\nstruct TwinINDEX;\n").unwrap();
        let text = Repeat::new(&header, &item, 3, None).load().unwrap();
        assert_eq!(
            text,
            "use crate;\n\nstruct Ix0;\nstruct Twin0;\nstruct Ix1;\nstruct Twin1;\nstruct Ix2;\nstruct Twin2;\n"
        );
    }

    #[test]
    fn repeat_writes_the_following_index_for_next() {
        let dir = TempDir::new();
        let header = dir.file("header.rs");
        let item = dir.file("item.rs");
        fs::write(&header, "use crate;\n\n").unwrap();
        fs::write(&item, "range = INDEX..NEXT;\n").unwrap();
        let text = Repeat::new(&header, &item, 2, Some("NEXT")).load().unwrap();
        assert_eq!(text, "use crate;\n\nrange = 0..1;\nrange = 1..2;\n");
    }

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("ph_bench_build_{}_{id}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn file(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}
