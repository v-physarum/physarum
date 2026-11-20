//! A byte range inside one assembly or IR string.

const TEXT_START: usize = 0;

/// Start and end offsets into the string that owns the bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ByteSpan {
    start: usize,
    end: usize,
}

impl ByteSpan {
    /// `start` and `end` are offsets into the same string.
    #[must_use]
    pub(crate) const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// The whole `len` bytes.
    #[must_use]
    pub(crate) const fn whole(len: usize) -> Self {
        Self {
            start: TEXT_START,
            end: len,
        }
    }

    /// The range of `inner` inside `outer`. `inner` is a subslice of `outer`.
    #[must_use]
    pub(crate) fn covering(outer: &str, inner: &str) -> Self {
        let start = (inner.as_ptr() as usize).saturating_sub(outer.as_ptr() as usize);
        Self {
            start,
            end: start + inner.len(),
        }
    }

    /// The text of this span.
    #[must_use]
    pub(crate) fn slice(self, text: &str) -> &str {
        &text[self.start..self.end]
    }
}
