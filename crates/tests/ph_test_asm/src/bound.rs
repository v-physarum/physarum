//! Caps and counts used by the assembly asserts.

/// Upper bound on a count. Built with [`Limit::new`] at the call that knows the number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Limit(u32);

impl Limit {
    /// No occurrences are allowed.
    pub(crate) const ZERO: Self = Self(0);

    /// One occurrence is allowed.
    pub(crate) const ONE: Self = Self(1);

    /// `value` is the caller's cap.
    #[must_use]
    pub(crate) const fn new(value: u32) -> Self {
        Self(value)
    }

    /// The number stored in this cap.
    #[must_use]
    pub(crate) const fn get(self) -> u32 {
        self.0
    }

    /// `count` is inside this cap.
    #[must_use]
    pub(crate) fn allows(self, count: Count) -> bool {
        count.get() <= usize::try_from(self.0).unwrap_or(usize::MAX)
    }
}

/// How many times something occurred.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Count(usize);

impl Count {
    /// `value` is the measured number.
    #[must_use]
    pub(crate) const fn new(value: usize) -> Self {
        Self(value)
    }

    /// The number stored in this count.
    #[must_use]
    pub(crate) const fn get(self) -> usize {
        self.0
    }
}
