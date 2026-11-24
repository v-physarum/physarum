//! Shared list carrier.
//!
//! A leaf is a tuple of at most eight items. Past that, items sit in groups
//! of those leaves. Push and the map walk both use this carrier.
//!
//! The trait and the carrier cannot both be named `List` (E0428). The carrier
//! is [`List`]. Traversal is [`Walk`]. Append is [`PushShape`](super::push::PushShape).

use crate::context::scope::ScopeBufs;
use crate::filter::eval::{Collect, ConstMatch, EmitMask};

/// Tuple of at most eight items, or a tuple of such groups.
/// Generics: `I` inner payload.
pub struct List<I>(pub(crate) I);

// Generics: `I` inner payload.
impl<I> List<I> {
    /// Wraps a flat leaf or group payload.
    #[must_use]
    pub const fn new(inner: I) -> Self {
        Self(inner)
    }
}

/// Flat leaf while the list is growing through lengths 0..=8.
///
/// Distinct from a bare tuple so [`PushShape`](super::push::PushShape) can use
/// opaque element params (`E0`, …) without overlapping `(List, Grow)`.
pub struct Leaf<I>(pub(crate) I);

/// Second group while the list is growing through lengths 9..=15.
///
/// Distinct from [`List`] so `(List<H>, Grow<R>)` can have one
/// [`PushShape`](super::push::PushShape) blanket (`R: AddItem`) without
/// overlapping the sixteen-item reshape (E0119). At sixteen the shape
/// becomes four [`List`] leaves again.
/// Generics: `I` inner payload.
pub struct Grow<I>(pub(crate) I);

/// Newtype list carrier: payload is [`Inner`](CarrierDesc::Inner).
///
/// [`List`] / [`Leaf`] / [`Grow`] set this once ([`ClaimDesc`](crate::ClaimDesc)
/// style). [`Walk`] / path walk blanket on `T: CarrierDesc` — not one behavior
/// impl per wrapper (rule 8).
#[doc(hidden)]
pub trait CarrierDesc {
    /// Inner payload.
    type Inner: ?Sized;
    /// Borrows the inner payload.
    fn carrier_inner(&self) -> &Self::Inner;
}

// Generics: `I` inner payload.
impl<I> CarrierDesc for List<I> {
    type Inner = I;

    fn carrier_inner(&self) -> &I {
        &self.0
    }
}

// Generics: `I` inner payload.
impl<I> CarrierDesc for Leaf<I> {
    type Inner = I;

    fn carrier_inner(&self) -> &I {
        &self.0
    }
}

// Generics: `I` inner payload.
impl<I> CarrierDesc for Grow<I> {
    type Inner = I;

    fn carrier_inner(&self) -> &I {
        &self.0
    }
}

/// Shared borrow for one walk: the concrete list and the scope buffers.
pub(crate) struct Lane<'a, L> {
    pub(crate) root: &'a L,
    pub(crate) scopes: &'a ScopeBufs,
}

/// Child emit: one const mask check per map (`F::MASK == P::MASK`).
pub trait ExactDrive {
    /// Runs maps whose filter mask equals `P::MASK`.
    ///
    /// Generics: `P` child path ([`Collect`]).
    fn drive_exact<P: Collect>(&self, scopes: &ScopeBufs);
}

// Generics: `I` inner payload ([`Walk`]).
impl<I> ExactDrive for List<I>
where
    I: Walk<List<I>>,
{
    fn drive_exact<P: Collect>(&self, scopes: &ScopeBufs) {
        let lane = Lane { root: self, scopes };
        self.0.walk(Exact::<P>(core::marker::PhantomData), &lane);
    }
}

/// How one walk decides which maps run.
///
/// Const emit, the deferred mask, and the exact child path are structs here.
/// Tuple fan-out is one [`Walk::walk`] (rule 8) — not a method per mode.
pub(crate) trait WalkStrategy {
    /// Whether a plain map under filter `F` runs.
    fn keep<F: Kept>(&self) -> bool;
    /// Whether a `.map_at` under filter `F` runs.
    ///
    /// Default: same as [`keep`](Self::keep). Exact child walks return false so
    /// a direct emit does not re-enter the caller.
    fn enter_at<F: Kept>(&self) -> bool {
        self.keep::<F>()
    }
}

/// Filter a walk strategy can test. One bound for [`WalkStrategy::keep`] and
/// [`WalkStrategy::enter_at`](WalkStrategy::enter_at).
pub(crate) trait Kept: crate::filter::eval::MatchEmit + Collect {}

// Generics: `F` filter.
impl<F: crate::filter::eval::MatchEmit + Collect> Kept for F {}

/// Const plan: `F::Against::<E>::MATCHES`.
/// Generics: `E` emit event.
pub(crate) struct ConstEmit<E>(pub(crate) core::marker::PhantomData<E>);

impl<E> Copy for ConstEmit<E> {}

impl<E> Clone for ConstEmit<E> {
    fn clone(&self) -> Self {
        *self
    }
}

// Generics: `E` emit tree ([`Collect`]).
impl<E: Collect> WalkStrategy for ConstEmit<E> {
    fn keep<F: Kept>(&self) -> bool {
        let _ = self;
        const { F::Against::<E>::MATCHES }
    }
}

/// Deferred scope mask, tested with [`MatchEmit::matches`](crate::filter::eval::MatchEmit::matches).
#[derive(Clone, Copy)]
pub(crate) struct DueMask(pub(crate) EmitMask);

impl WalkStrategy for DueMask {
    fn keep<F: Kept>(&self) -> bool {
        F::matches(self.0)
    }
}

/// Child path: keep a plain map only when `F::MASK` equals `P::MASK`.
/// Generics: `P` exact path.
pub(crate) struct Exact<P>(pub(crate) core::marker::PhantomData<P>);

impl<P> Copy for Exact<P> {}

impl<P> Clone for Exact<P> {
    fn clone(&self) -> Self {
        *self
    }
}

// Generics: `P` child path ([`Collect`]).
impl<P: Collect> WalkStrategy for Exact<P> {
    fn keep<F: Kept>(&self) -> bool {
        let _ = self;
        const { F::MASK.same(P::MASK) }
    }

    fn enter_at<F: Kept>(&self) -> bool {
        let _ = (self, core::marker::PhantomData::<F>);
        false
    }
}

/// Walks a leaf, then each group, in order.
///
/// One method for every mode ([`WalkStrategy`]). `L: ExactDrive` is a `where`
/// on this method, so `List: ExactDrive` can require [`Walk`] and the
/// `.map_at` leaf can still call [`ExactDrive`] while that impl is open.
///
/// Generics: `L` list ([`ExactDrive`]). `S` is the [`WalkStrategy`].
#[doc(hidden)]
pub(crate) trait Walk<L> {
    /// Visits every child under `strategy`.
    fn walk<S: WalkStrategy + Copy>(&self, strategy: S, lane: &Lane<L>)
    where
        L: ExactDrive;
}

// Generics: `L` list.
impl<L> Walk<L> for () {
    fn walk<S: WalkStrategy + Copy>(&self, strategy: S, lane: &Lane<L>)
    where
        L: ExactDrive,
    {
        let _ = (self, strategy, lane);
    }
}

// Generics: `L` list, `E0` child slot.
impl<L, E0> Walk<L> for (E0,)
where
    E0: Walk<L>,
{
    fn walk<S: WalkStrategy + Copy>(&self, strategy: S, lane: &Lane<L>)
    where
        L: ExactDrive,
    {
        self.0.walk(strategy, lane);
    }
}

// Generics: `L` list, `E0`…`E1` child slots.
impl<L, E0, E1> Walk<L> for (E0, E1)
where
    E0: Walk<L>,
    E1: Walk<L>,
{
    fn walk<S: WalkStrategy + Copy>(&self, strategy: S, lane: &Lane<L>)
    where
        L: ExactDrive,
    {
        self.0.walk(strategy, lane);
        self.1.walk(strategy, lane);
    }
}

// Generics: `L` list, `E0`…`E2` child slots.
impl<L, E0, E1, E2> Walk<L> for (E0, E1, E2)
where
    E0: Walk<L>,
    E1: Walk<L>,
    E2: Walk<L>,
{
    fn walk<S: WalkStrategy + Copy>(&self, strategy: S, lane: &Lane<L>)
    where
        L: ExactDrive,
    {
        self.0.walk(strategy, lane);
        self.1.walk(strategy, lane);
        self.2.walk(strategy, lane);
    }
}

// Generics: `L` list, `E0`…`E3` child slots.
impl<L, E0, E1, E2, E3> Walk<L> for (E0, E1, E2, E3)
where
    E0: Walk<L>,
    E1: Walk<L>,
    E2: Walk<L>,
    E3: Walk<L>,
{
    fn walk<S: WalkStrategy + Copy>(&self, strategy: S, lane: &Lane<L>)
    where
        L: ExactDrive,
    {
        self.0.walk(strategy, lane);
        self.1.walk(strategy, lane);
        self.2.walk(strategy, lane);
        self.3.walk(strategy, lane);
    }
}

// Generics: `L` list, `E0`…`E4` child slots.
impl<L, E0, E1, E2, E3, E4> Walk<L> for (E0, E1, E2, E3, E4)
where
    E0: Walk<L>,
    E1: Walk<L>,
    E2: Walk<L>,
    E3: Walk<L>,
    E4: Walk<L>,
{
    fn walk<S: WalkStrategy + Copy>(&self, strategy: S, lane: &Lane<L>)
    where
        L: ExactDrive,
    {
        self.0.walk(strategy, lane);
        self.1.walk(strategy, lane);
        self.2.walk(strategy, lane);
        self.3.walk(strategy, lane);
        self.4.walk(strategy, lane);
    }
}

// Generics: `L` list, `E0`…`E5` child slots.
impl<L, E0, E1, E2, E3, E4, E5> Walk<L> for (E0, E1, E2, E3, E4, E5)
where
    E0: Walk<L>,
    E1: Walk<L>,
    E2: Walk<L>,
    E3: Walk<L>,
    E4: Walk<L>,
    E5: Walk<L>,
{
    fn walk<S: WalkStrategy + Copy>(&self, strategy: S, lane: &Lane<L>)
    where
        L: ExactDrive,
    {
        self.0.walk(strategy, lane);
        self.1.walk(strategy, lane);
        self.2.walk(strategy, lane);
        self.3.walk(strategy, lane);
        self.4.walk(strategy, lane);
        self.5.walk(strategy, lane);
    }
}

// Generics: `L` list, `E0`…`E6` child slots.
impl<L, E0, E1, E2, E3, E4, E5, E6> Walk<L> for (E0, E1, E2, E3, E4, E5, E6)
where
    E0: Walk<L>,
    E1: Walk<L>,
    E2: Walk<L>,
    E3: Walk<L>,
    E4: Walk<L>,
    E5: Walk<L>,
    E6: Walk<L>,
{
    fn walk<S: WalkStrategy + Copy>(&self, strategy: S, lane: &Lane<L>)
    where
        L: ExactDrive,
    {
        self.0.walk(strategy, lane);
        self.1.walk(strategy, lane);
        self.2.walk(strategy, lane);
        self.3.walk(strategy, lane);
        self.4.walk(strategy, lane);
        self.5.walk(strategy, lane);
        self.6.walk(strategy, lane);
    }
}

// Generics: `L` list, `E0`…`E7` child slots. One impl per length (rules 11–12).
impl<L, E0, E1, E2, E3, E4, E5, E6, E7> Walk<L> for (E0, E1, E2, E3, E4, E5, E6, E7)
where
    E0: Walk<L>,
    E1: Walk<L>,
    E2: Walk<L>,
    E3: Walk<L>,
    E4: Walk<L>,
    E5: Walk<L>,
    E6: Walk<L>,
    E7: Walk<L>,
{
    fn walk<S: WalkStrategy + Copy>(&self, strategy: S, lane: &Lane<L>)
    where
        L: ExactDrive,
    {
        self.0.walk(strategy, lane);
        self.1.walk(strategy, lane);
        self.2.walk(strategy, lane);
        self.3.walk(strategy, lane);
        self.4.walk(strategy, lane);
        self.5.walk(strategy, lane);
        self.6.walk(strategy, lane);
        self.7.walk(strategy, lane);
    }
}

// Generics: `T` carrier ([`CarrierDesc`]), `L` list.
impl<L, T> Walk<L> for T
where
    T: CarrierDesc,
    T::Inner: Walk<L>,
{
    fn walk<S: WalkStrategy + Copy>(&self, strategy: S, lane: &Lane<L>)
    where
        L: ExactDrive,
    {
        self.carrier_inner().walk(strategy, lane);
    }
}
