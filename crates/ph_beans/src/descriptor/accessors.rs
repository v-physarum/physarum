//! Resource markers and the stub accessor set.
//!
//! [`Get`], [`Mut`], and [`Marker`] are **frozen** (design rule 19 / Accessors):
//! do not change their shape without prior owner approval. Extra views use a
//! side type (`Dyn`), not a reshape of these markers.

use core::marker::PhantomData;

use crate::bean::BeanInfo;

/// Stub live-accessor set.
///
/// Every descriptor in this slice reports this empty set.
pub struct Accessors;

/// Shared claim on `T`.
///
/// Frozen shape — `Get<T>(PhantomData<T>)` only.
pub struct Get<T>(PhantomData<T>);

impl<T> Get<T> {
    /// Marker for `T`.
    #[must_use]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T> Default for Get<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Exclusive claim on `T`.
///
/// Frozen shape — `Mut<T>(PhantomData<T>)` only.
pub struct Mut<T>(PhantomData<T>);

impl<T> Mut<T> {
    /// Marker for `T`.
    #[must_use]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T> Default for Mut<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// `T` and `MUT` markers used by [`.inject_resources`](crate::Init::inject_resources).
///
/// Frozen — only [`Get`] / [`Mut`] consts; no new associated items without owner OK.
pub trait Marker: Sized {
    /// Shared claim.
    const T: Get<Self>;
    /// Exclusive claim.
    const MUT: Mut<Self>;
}

impl<T: BeanInfo> Marker for T {
    const T: Get<Self> = Get::new();
    const MUT: Mut<Self> = Mut::new();
}
