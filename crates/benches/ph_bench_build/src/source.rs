//! Case templates on disk.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::error::Result;

/// Header and item text, pasted `count` times into one sized string.
///
/// `INDEX` in the item is the paste index. `next`, when set, is the key replaced
/// with the following index. The header is padded so it ends on a blank line.
#[must_use]
pub(crate) fn render(header: &str, item: &str, count: u32, next: Option<&str>) -> String {
    let mut source = String::with_capacity(output_len(header, item, count, next));
    source.push_str(header);
    for _ in 0..blank_pad(header) {
        source.push('\n');
    }
    append_items(&mut source, item, count, next);
    source
}

/// `text` with `key` replaced by the decimal `count`, written once.
#[must_use]
pub(crate) fn fill_count(text: &str, key: &str, count: u32) -> String {
    let hits = text.matches(key).count();
    let digits = Digits::new(count);
    let cap = text
        .len()
        .saturating_sub(key.len().saturating_mul(hits))
        .saturating_add(digits.len().saturating_mul(hits));
    let mut out = String::with_capacity(cap);
    splice(&mut out, text, key, count);
    out
}

/// `{kind}_{count}` in one allocation.
#[must_use]
pub(crate) fn case_name(kind: &str, count: u32) -> String {
    let digits = Digits::new(count);
    let mut name = String::with_capacity(kind.len() + 1 + digits.len());
    name.push_str(kind);
    name.push('_');
    digits.append_to(&mut name);
    name
}

/// File bytes in one buffer sized from the file length.
///
/// # Errors
///
/// Returns an error when the file cannot be read.
pub(crate) fn read_exact(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let cap = file
        .metadata()
        .ok()
        .and_then(|meta| usize::try_from(meta.len()).ok())
        .unwrap_or(0);
    let mut text = String::with_capacity(cap);
    file.read_to_string(&mut text)?;
    Ok(text)
}

const INDEX_KEY: &str = "INDEX";
/// `u32` is at most this many decimal digits.
const DIGIT_CAP: usize = 10;

/// How many newlines `render` writes so the header ends on a blank line.
fn blank_pad(header: &str) -> usize {
    if header.ends_with("\n\n") {
        0
    } else if header.ends_with('\n') {
        1
    } else {
        2
    }
}

/// Bytes `render` needs for `count` copies, including the blank line.
fn output_len(header: &str, item: &str, count: u32, next: Option<&str>) -> usize {
    let pasted = expanded(item, INDEX_KEY);
    let per = match next {
        Some(key) => grown(pasted, key),
        None => pasted,
    };
    header
        .len()
        .saturating_add(blank_pad(header))
        .saturating_add(per.saturating_mul(count as usize))
}

/// Appends `item` `count` times. One scratch buffer is reused when `next` is set.
fn append_items(source: &mut String, item: &str, count: u32, next: Option<&str>) {
    if let Some(key) = next {
        let mut scratch = String::with_capacity(expanded(item, INDEX_KEY));
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

    fn len(self) -> usize {
        DIGIT_CAP - self.at
    }

    fn append_to(self, out: &mut String) {
        for byte in &self.bytes[self.at..] {
            out.push(char::from(*byte));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::render;

    #[test]
    fn repeat_writes_every_index_into_one_string() {
        let text = render(
            "use crate;\n",
            "struct IxINDEX;\nstruct TwinINDEX;\n",
            3,
            None,
        );
        assert_eq!(
            text,
            "use crate;\n\nstruct Ix0;\nstruct Twin0;\nstruct Ix1;\nstruct Twin1;\nstruct Ix2;\nstruct Twin2;\n"
        );
    }

    #[test]
    fn repeat_writes_the_following_index_for_next() {
        let text = render("use crate;\n\n", "range = INDEX..NEXT;\n", 2, Some("NEXT"));
        assert_eq!(text, "use crate;\n\nrange = 0..1;\nrange = 1..2;\n");
    }
}
