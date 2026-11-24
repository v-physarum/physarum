//! Resource markers and the stub accessor set.
//!
//! [`Get`], [`Mut`], and [`Marker`] are **frozen** (design rule 19 / Accessors):
//! do not change their shape without prior owner approval. Extra views use a
//! side type (`Dyn`), not a reshape of these markers.

use core::marker::PhantomData;
use core::ops::Deref;

use crate::bean::BeanInfo;

/// Stub live-accessor set.
///
/// Every descriptor in this slice reports this empty set.
pub struct Accessors;

/// Shared claim on `T`.
///
/// Frozen shape — `Get<T>(PhantomData<T>)` only.
/// Generics: `T` claimed type.
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
/// Generics: `T` claimed type.
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

/// State of event bean `T`, claimed as [`Get`]`<State<T>>` or [`Mut`]`<State<T>>`.
///
/// Not an accessor. `*Ev::T` / `*Ev::MUT` are the existing [`Get`] / [`Mut`].
/// Generics: `T` claimed type.
pub struct State<T>(PhantomData<T>);

// `*` yields a value. Only the state parameter is [`Copy`]; [`Get`] itself is not.
impl<T> Copy for Get<State<T>> {}

impl<T> Clone for Get<State<T>> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Mut<State<T>> {}

impl<T> Clone for Mut<State<T>> {
    fn clone(&self) -> Self {
        *self
    }
}

// Generics: `T` event bean ([`BeanInfo`]).
impl<T: BeanInfo> Deref for Get<T> {
    type Target = Get<State<T>>;

    fn deref(&self) -> &Self::Target {
        let _ = self;
        const { &Get::<State<T>>::new() }
    }
}

// Generics: `T` event bean ([`BeanInfo`]).
impl<T: BeanInfo> Deref for Mut<T> {
    type Target = Mut<State<T>>;

    fn deref(&self) -> &Self::Target {
        let _ = self;
        const { &Mut::<State<T>>::new() }
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

// Generics: `T` claimed type.
impl<T: BeanInfo> Marker for T {
    const T: Get<Self> = Get::new();
    const MUT: Mut<Self> = Mut::new();
}
