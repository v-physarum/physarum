//! Shared list carrier.
//!
//! A leaf is a tuple of at most eight items. Past that, items sit in groups
//! of those leaves. Push and the map walk both use this carrier.
//!
//! The trait and the carrier cannot both be named `List` (E0428). The carrier
//! is [`List`]. One [`Invoke`] fans a tuple out. The use site passes the
//! behavior as [`Visit`]. Append is [`PushShape`](super::push::PushShape).

use crate::context::direct::{Direct, DirectFn, TipEvent};
use crate::context::maps::{MapAt, MapEntry};
use crate::context::scope::ScopeBufs;
use crate::filter::eval::{Collect, ConstMatch, EmitMask, MatchEmit};
use crate::filter::{Gate, PathStep, PathStrategy};

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
/// style). [`Invoke`] blankets on `T: CarrierDesc` — not one behavior
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

// Generics: `I` inner payload ([`Invoke`]).
impl<I> ExactDrive for List<I>
where
    I: Invoke,
{
    fn drive_exact<P: Collect>(&self, scopes: &ScopeBufs) {
        let lane = Lane { root: self, scopes };
        self.0
            .invoke(WalkOp { strategy: Exact::<P>(core::marker::PhantomData), lane: &lane });
    }
}

/// How one walk decides which maps run.
///
/// Const emit, the deferred mask, and the exact child path are structs here.
/// Tuple fan-out is one [`Invoke::invoke`] (rule 8) — not a method per mode.
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

/// One list. `invoke` places `Op` (the behavior) on every child.
///
/// Lengths `0..=8` and the [`List`] / [`Leaf`] / [`Grow`] carriers share this.
/// A leaf forwards to [`Visit`]. Take, append, and claim pairs stay outside:
/// those tuples are the data, and their output type is not a uniform visit.
pub(crate) trait Invoke {
    /// Runs `op` on this node.
    fn invoke<Op>(&self, op: Op)
    where
        Op: Copy + Visit;
}

/// Behavior a list visit places in [`Invoke`].
pub(crate) trait Visit {
    /// Plain `.map` leaf.
    fn on_entry<F, H>(&self, entry: &MapEntry<F, H>)
    where
        F: MatchEmit + Collect,
        H: Fn();

    /// `.map_at` leaf. `L: ExactDrive` stays on the walk op, not here.
    fn on_at<F, H>(&self, entry: &MapAt<F, H>)
    where
        F: MatchEmit + Collect + TipEvent,
        H: DirectFn<F::Tip, F>;

    /// Path gate. `M` is the op's emit const, not a parameter here.
    fn on_gate<P, S, C>(&self, gate: &Gate<P, S, C>)
    where
        P: PathStep,
        S: PathStrategy,
        C: Invoke;
}

/// Walk behavior: one [`WalkStrategy`] plus the shared lane.
pub(crate) struct WalkOp<'a, L, S> {
    pub(crate) strategy: S,
    pub(crate) lane: &'a Lane<'a, L>,
}

impl<L, S: Copy> Copy for WalkOp<'_, L, S> {}

impl<L, S: Copy> Clone for WalkOp<'_, L, S> {
    fn clone(&self) -> Self {
        *self
    }
}

// Generics: `L` list ([`ExactDrive`]), `S` walk strategy.
impl<L, S> Visit for WalkOp<'_, L, S>
where
    S: WalkStrategy + Copy,
    L: ExactDrive,
{
    fn on_entry<F, H>(&self, entry: &MapEntry<F, H>)
    where
        F: MatchEmit + Collect,
        H: Fn(),
    {
        let _ = self.lane;
        if self.strategy.keep::<F>() {
            (entry.handler)();
        }
    }

    fn on_at<F, H>(&self, entry: &MapAt<F, H>)
    where
        F: MatchEmit + Collect + TipEvent,
        H: DirectFn<F::Tip, F>,
    {
        if self.strategy.enter_at::<F>() {
            entry.handler.run(&Direct::from_lane(self.lane));
        }
    }

    fn on_gate<P, S2, C>(&self, gate: &Gate<P, S2, C>)
    where
        P: PathStep,
        S2: PathStrategy,
        C: Invoke,
    {
        let _ = (self, gate);
    }
}

// Generics: `F` filter, `H` handler.
impl<F, H> Invoke for MapEntry<F, H>
where
    F: MatchEmit + Collect,
    H: Fn(),
{
    fn invoke<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        op.on_entry(self);
    }
}

// Generics: `F` filter, `H` handler ([`DirectFn`]).
impl<F, H> Invoke for MapAt<F, H>
where
    F: MatchEmit + Collect + TipEvent,
    H: DirectFn<F::Tip, F>,
{
    fn invoke<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        op.on_at(self);
    }
}

// Generics: `Op` behavior ([`Visit`]).
impl Invoke for () {
    fn invoke<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        let _ = (self, op);
    }
}

// Generics: `Op` behavior, `E0` child slot.
impl<E0> Invoke for (E0,)
where
    E0: Invoke,
{
    fn invoke<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        self.0.invoke(op);
    }
}

// Generics: `Op` behavior, `E0`…`E1` child slots.
impl<E0, E1> Invoke for (E0, E1)
where
    E0: Invoke,
    E1: Invoke,
{
    fn invoke<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        self.0.invoke(op);
        self.1.invoke(op);
    }
}

// Generics: `Op` behavior, `E0`…`E2` child slots.
impl<E0, E1, E2> Invoke for (E0, E1, E2)
where
    E0: Invoke,
    E1: Invoke,
    E2: Invoke,
{
    fn invoke<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        self.0.invoke(op);
        self.1.invoke(op);
        self.2.invoke(op);
    }
}

// Generics: `Op` behavior, `E0`…`E3` child slots.
impl<E0, E1, E2, E3> Invoke for (E0, E1, E2, E3)
where
    E0: Invoke,
    E1: Invoke,
    E2: Invoke,
    E3: Invoke,
{
    fn invoke<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        self.0.invoke(op);
        self.1.invoke(op);
        self.2.invoke(op);
        self.3.invoke(op);
    }
}

// Generics: `Op` behavior, `E0`…`E4` child slots.
impl<E0, E1, E2, E3, E4> Invoke for (E0, E1, E2, E3, E4)
where
    E0: Invoke,
    E1: Invoke,
    E2: Invoke,
    E3: Invoke,
    E4: Invoke,
{
    fn invoke<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        self.0.invoke(op);
        self.1.invoke(op);
        self.2.invoke(op);
        self.3.invoke(op);
        self.4.invoke(op);
    }
}

// Generics: `Op` behavior, `E0`…`E5` child slots.
impl<E0, E1, E2, E3, E4, E5> Invoke for (E0, E1, E2, E3, E4, E5)
where
    E0: Invoke,
    E1: Invoke,
    E2: Invoke,
    E3: Invoke,
    E4: Invoke,
    E5: Invoke,
{
    fn invoke<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        self.0.invoke(op);
        self.1.invoke(op);
        self.2.invoke(op);
        self.3.invoke(op);
        self.4.invoke(op);
        self.5.invoke(op);
    }
}

// Generics: `Op` behavior, `E0`…`E6` child slots.
impl<E0, E1, E2, E3, E4, E5, E6> Invoke for (E0, E1, E2, E3, E4, E5, E6)
where
    E0: Invoke,
    E1: Invoke,
    E2: Invoke,
    E3: Invoke,
    E4: Invoke,
    E5: Invoke,
    E6: Invoke,
{
    fn invoke<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        self.0.invoke(op);
        self.1.invoke(op);
        self.2.invoke(op);
        self.3.invoke(op);
        self.4.invoke(op);
        self.5.invoke(op);
        self.6.invoke(op);
    }
}

// Generics: `Op` behavior, `E0`…`E7` child slots. One impl per length (rules 11–12).
impl<E0, E1, E2, E3, E4, E5, E6, E7> Invoke for (E0, E1, E2, E3, E4, E5, E6, E7)
where
    E0: Invoke,
    E1: Invoke,
    E2: Invoke,
    E3: Invoke,
    E4: Invoke,
    E5: Invoke,
    E6: Invoke,
    E7: Invoke,
{
    fn invoke<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        self.0.invoke(op);
        self.1.invoke(op);
        self.2.invoke(op);
        self.3.invoke(op);
        self.4.invoke(op);
        self.5.invoke(op);
        self.6.invoke(op);
        self.7.invoke(op);
    }
}

// Generics: `T` carrier ([`CarrierDesc`]), `Op` behavior.
impl<T> Invoke for T
where
    T: CarrierDesc,
    T::Inner: Invoke,
{
    fn invoke<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        self.carrier_inner().invoke(op);
    }
}
