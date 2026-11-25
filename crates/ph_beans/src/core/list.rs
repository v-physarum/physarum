//! Grouped list: a closed [`Head`] and one open slot.
//!
//! An open slot is [`End`] or a [`Group`] of one to three items. Four items
//! close into one [`Full`]. At most four groups. Append is [`PushShape`].

use crate::core::config::Config;
use crate::core::config::OfConfig;
pub use crate::core::push::{AddItem, Pack, Place, PushEntry, PushShape};

/// List carrier. Generics: `I` payload.
pub struct List<I>(pub(crate) I);

// Generics: `I` items.
impl<I> List<I> {
    /// Wraps `inner`.
    #[must_use]
    pub const fn new(inner: I) -> Self {
        Self(inner)
    }

    /// Borrows the payload.
    #[must_use]
    pub const fn inner(&self) -> &I {
        &self.0
    }
}

/// Empty list: [`Head`]`<()>` beside [`End`].
pub struct EmptySlots;

impl EmptySlots {
    /// Builds [`OfConfig`]`<Self>`.
    #[must_use]
    #[inline]
    pub fn empty() -> OfConfig<Self> {
        List::new((Head(()), End))
    }
}

impl Config for EmptySlots {
    type Type = List<(Head<()>, End)>;
}

/// Closed groups already stored.
///
/// Generics: `I` closed groups (`()` or a tuple of [`Full`]).
pub struct Head<I>(pub(crate) I);

// Generics: `I` items.
impl<I> Head<I> {
    /// Borrows the closed groups.
    #[must_use]
    pub const fn inner(&self) -> &I {
        &self.0
    }
}

/// Open slot with no items yet.
#[derive(Clone, Copy)]
pub struct End;

/// Open group of one to three items.
///
/// Generics: `I` items.
pub struct Group<I>(pub(crate) I);

// Generics: `I` items.
impl<I> Group<I> {
    /// Borrows the items.
    #[must_use]
    pub const fn inner(&self) -> &I {
        &self.0
    }
}

/// Closed group of four items.
///
/// Generics: `I` items.
pub struct Full<I>(pub(crate) I);

// Generics: `I` items.
impl<I> Full<I> {
    /// Borrows the items.
    #[must_use]
    pub const fn inner(&self) -> &I {
        &self.0
    }
}
