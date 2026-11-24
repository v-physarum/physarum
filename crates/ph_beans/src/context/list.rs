//! Map walk over the shared list in [`crate::core::list`].
//!
//! [`Invoke`] is only the tuple list (`()` and lengths 1..=8).
//! [`MapEntry`](crate::MapEntry), [`MapAt`](super::maps::MapAt), [`Gate`](crate::Gate),
//! [`Group`], [`Full`], [`Head`], and [`End`] hand the visitor through [`Hand`]
//! and do not implement [`Invoke`].

pub use crate::core::list::{EmptySlots, End, Full, Group, Head, List};

use crate::context::direct::{Direct, DirectFn, TipEvent};
use crate::context::maps::{MapAt, MapEntry};
use crate::context::scope::ScopeBufs;
use crate::filter::eval::{Collect, ConstMatch, EmitMask, MatchEmit};
use crate::filter::{Gate, PathStep, PathStrategy};

/// Newtype list carrier: payload is [`Inner`](CarrierDesc::Inner).
///
/// [`List`], [`Head`], [`Group`], [`Full`], and [`End`] set this once
/// ([`ClaimDesc`](crate::ClaimDesc) style). They do not implement [`Invoke`].
/// [`Hand`] forwards to [`Inner`](Self::Inner).
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
        self.inner()
    }
}

impl CarrierDesc for End {
    type Inner = Self;

    fn carrier_inner(&self) -> &Self {
        self
    }
}

// Generics: `I` items.
impl<I> CarrierDesc for Group<I> {
    type Inner = I;

    fn carrier_inner(&self) -> &I {
        self.inner()
    }
}

// Generics: `I` items.
impl<I> CarrierDesc for Full<I> {
    type Inner = I;

    fn carrier_inner(&self) -> &I {
        self.inner()
    }
}

// Generics: `I` closed groups.
impl<I> CarrierDesc for Head<I> {
    type Inner = I;

    fn carrier_inner(&self) -> &I {
        self.inner()
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

// Generics: `I` inner payload ([`Hand`]).
impl<I> ExactDrive for List<I>
where
    I: Hand,
{
    fn drive_exact<P: Collect>(&self, scopes: &ScopeBufs) {
        let lane = Lane { root: self, scopes };
        self.inner()
            .hand(WalkOp { strategy: Exact::<P>(core::marker::PhantomData), lane: &lane });
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
        let _ = self;
        false
    }
}

/// The tuple list. `invoke` places `Op` (the behavior) on every child.
///
/// Only `()` and flat lengths 1..=8 implement this. One impl per length
/// (rules 11–12): a single impl cannot name every arity. [`MapEntry`],
/// [`MapAt`], [`Gate`](crate::Gate), and [`CarrierDesc`] types use [`Hand`]
/// (and their own traits) instead.
pub(crate) trait Invoke {
    /// Runs `op` on this tuple.
    fn invoke<Op>(&self, op: Op)
    where
        Op: Copy + Visit;
}

/// Presents a node that is not the tuple list to [`Visit`].
///
/// [`Invoke`] is not implemented for these types. A tuple calls [`hand`](Self::hand)
/// on each child. Carriers forward to [`CarrierDesc::Inner`].
pub(crate) trait Hand {
    /// Presents this node to `op`.
    fn hand<Op>(&self, op: Op)
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
        C: Hand;
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
        C: Hand,
    {
        let _ = (self, gate);
    }
}

// Generics: `F` filter, `H` handler. Not [`Invoke`] — this leaf stays on [`Visit::on_entry`].
impl<F, H> Hand for MapEntry<F, H>
where
    F: MatchEmit + Collect,
    H: Fn(),
{
    fn hand<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        op.on_entry(self);
    }
}

// Generics: `F` filter, `H` handler ([`DirectFn`]). Not [`Invoke`].
impl<F, H> Hand for MapAt<F, H>
where
    F: MatchEmit + Collect + TipEvent,
    H: DirectFn<F::Tip, F>,
{
    fn hand<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        op.on_at(self);
    }
}

// Carriers are not the tuple list. They open [`CarrierDesc::Inner`].
impl<I: Hand> Hand for List<I> {
    fn hand<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        self.inner().hand(op);
    }
}

impl Hand for End {
    fn hand<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        let _ = (self, op);
    }
}

// Generics: `I` items ([`Hand`]).
impl<I: Hand> Hand for Group<I> {
    fn hand<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        self.inner().hand(op);
    }
}

// Generics: `I` items ([`Hand`]).
impl<I: Hand> Hand for Full<I> {
    fn hand<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        self.inner().hand(op);
    }
}

// Generics: `I` closed groups ([`Hand`]).
impl<I: Hand> Hand for Head<I> {
    fn hand<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        self.inner().hand(op);
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
    E0: Hand,
{
    fn invoke<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        self.0.hand(op);
    }
}

// Generics: `Op` behavior, `E0`…`E1` child slots.
impl<E0, E1> Invoke for (E0, E1)
where
    E0: Hand,
    E1: Hand,
{
    fn invoke<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        self.0.hand(op);
        self.1.hand(op);
    }
}

// Generics: `Op` behavior, `E0`…`E2` child slots.
impl<E0, E1, E2> Invoke for (E0, E1, E2)
where
    E0: Hand,
    E1: Hand,
    E2: Hand,
{
    fn invoke<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        self.0.hand(op);
        self.1.hand(op);
        self.2.hand(op);
    }
}

// Generics: `Op` behavior, `E0`…`E3` child slots.
impl<E0, E1, E2, E3> Invoke for (E0, E1, E2, E3)
where
    E0: Hand,
    E1: Hand,
    E2: Hand,
    E3: Hand,
{
    fn invoke<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        self.0.hand(op);
        self.1.hand(op);
        self.2.hand(op);
        self.3.hand(op);
    }
}

// Generics: `Op` behavior, `E0`…`E4` child slots.
impl<E0, E1, E2, E3, E4> Invoke for (E0, E1, E2, E3, E4)
where
    E0: Hand,
    E1: Hand,
    E2: Hand,
    E3: Hand,
    E4: Hand,
{
    fn invoke<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        self.0.hand(op);
        self.1.hand(op);
        self.2.hand(op);
        self.3.hand(op);
        self.4.hand(op);
    }
}

// Generics: `Op` behavior, `E0`…`E5` child slots.
impl<E0, E1, E2, E3, E4, E5> Invoke for (E0, E1, E2, E3, E4, E5)
where
    E0: Hand,
    E1: Hand,
    E2: Hand,
    E3: Hand,
    E4: Hand,
    E5: Hand,
{
    fn invoke<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        self.0.hand(op);
        self.1.hand(op);
        self.2.hand(op);
        self.3.hand(op);
        self.4.hand(op);
        self.5.hand(op);
    }
}

// Generics: `Op` behavior, `E0`…`E6` child slots.
impl<E0, E1, E2, E3, E4, E5, E6> Invoke for (E0, E1, E2, E3, E4, E5, E6)
where
    E0: Hand,
    E1: Hand,
    E2: Hand,
    E3: Hand,
    E4: Hand,
    E5: Hand,
    E6: Hand,
{
    fn invoke<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        self.0.hand(op);
        self.1.hand(op);
        self.2.hand(op);
        self.3.hand(op);
        self.4.hand(op);
        self.5.hand(op);
        self.6.hand(op);
    }
}

// Generics: `Op` behavior, `E0`…`E7` child slots. One impl per length (rules 11–12).
impl<E0, E1, E2, E3, E4, E5, E6, E7> Invoke for (E0, E1, E2, E3, E4, E5, E6, E7)
where
    E0: Hand,
    E1: Hand,
    E2: Hand,
    E3: Hand,
    E4: Hand,
    E5: Hand,
    E6: Hand,
    E7: Hand,
{
    fn invoke<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        self.0.hand(op);
        self.1.hand(op);
        self.2.hand(op);
        self.3.hand(op);
        self.4.hand(op);
        self.5.hand(op);
        self.6.hand(op);
        self.7.hand(op);
    }
}

// A tuple list is handed on by running [`Invoke`]. Not implemented for carriers.
impl<T: Invoke> Hand for T {
    fn hand<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        self.invoke(op);
    }
}
