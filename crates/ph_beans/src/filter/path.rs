//! Hierarchical emit walk: DELTA gates + strategy structs.
//!
//! Design **rule 8**: [`PathStrategy`] is the trait; [`Sparse`] / [`Dense`] are
//! the variants; siblings are a flat leaf ≤8 (often inside [`List`](crate::List));
//! carriers use [`CarrierDesc`](crate::CarrierDesc) + one [`PathNode`] blanket.
//! Beside flat [`Walk`](crate::context::list::Walk) /
//! [`Drive`](crate::query::schedule::Drive).
//!
//! Each gate intersects [`PathStep::DELTA`] with the emit const `M::MASK`.
//! A miss deletes the subtree. A hit runs children with no runtime bit test
//! ([`Dense`]) or lets each child repeat that const test ([`Sparse`]).

use core::marker::PhantomData;

use crate::context::list::CarrierDesc;
use crate::context::maps::MapEntry;
use crate::filter::EmitMask;
use crate::filter::eval::{Collect, ConstMatch, EmitBit, MatchEmit};

/// Bits this path step adds relative to its parent (one event bit, typically).
#[doc(hidden)]
pub trait PathStep {
    /// Gate mask: `mask & DELTA == DELTA` to enter.
    const DELTA: EmitMask;
}

// Generics: `E` event bit.
impl<E: EmitBit> PathStep for E {
    const DELTA: EmitMask = EmitMask::from_bit(E::BIT);
}

/// How a gate runs its children after the emit const opens the gate.
#[doc(hidden)]
pub trait PathStrategy {
    /// Runs `children` for emit tree `M`. The gate test already passed as a const.
    ///
    /// Generics: `M` emit tree ([`Collect`]), `C` children ([`PathNode`]).
    fn after_open<M, C>(children: &C)
    where
        M: Collect,
        C: PathNode<Self, M>,
    {
        children.visit();
    }
}

/// Node visitable under a path strategy for one emit const.
///
/// Generics: `S` strategy ([`PathStrategy`] — [`Sparse`] / [`Dense`]),
/// `M` emit tree ([`Collect`]).
#[doc(hidden)]
pub trait PathNode<S: PathStrategy + ?Sized, M: Collect + ?Sized> {
    /// Visits this node. The emit const is `M::MASK`.
    fn visit(&self);
}

/// Children keep their own bit tests (or nested gates).
pub struct Sparse;

/// After the gate matches, run every child with no further emit tests.
pub struct Dense;

impl PathStrategy for Sparse {}

impl PathStrategy for Dense {}

/// One path step: check [`PathStep::DELTA`], then apply strategy `S`.
///
/// Generics: `P` path step ([`PathStep`]), `S` strategy ([`PathStrategy`]), `C` children.
pub struct Gate<P, S, C> {
    children: C,
    _markers: PhantomData<(P, S)>,
}

// Generics: `P` path step, `S` strategy, `C` children.
impl<P, S, C> Gate<P, S, C> {
    /// Wraps `children` under this gate.
    #[must_use]
    pub const fn new(children: C) -> Self {
        Self { children, _markers: PhantomData }
    }
}

// Generics: `P` path step, `S` strategy, `C` children, `M` emit tree.
impl<P, S, C> Gate<P, S, C>
where
    P: PathStep,
    S: PathStrategy,
{
    /// Opens when `M::MASK` contains `P::DELTA`, then runs children via `S`.
    pub fn path_plan<M>(&self)
    where
        M: Collect,
        C: PathNode<S, M>,
    {
        if const { M::MASK.contains(P::DELTA) } {
            S::after_open::<M, C>(&self.children);
        }
    }
}

// Generics: `P` path step, `S` strategy, `C` children, `M` emit tree.
impl<P, S, C, M> PathNode<Sparse, M> for Gate<P, S, C>
where
    P: PathStep,
    S: PathStrategy,
    M: Collect,
    C: PathNode<S, M>,
{
    fn visit(&self) {
        self.path_plan::<M>();
    }
}

// Generics: `M` emit tree, `F` filter ([`MatchEmit`]).
impl<M, F> PathNode<Sparse, M> for MapEntry<F, fn()>
where
    M: Collect,
    F: MatchEmit,
{
    fn visit(&self) {
        // Gate bits already passed as a const. This leaf uses the same emit const.
        if const { F::Against::<M>::MATCHES } {
            (self.handler)();
        }
    }
}

// Generics: `M` emit tree (unused: the gate already opened), `F` filter.
impl<M, F> PathNode<Dense, M> for MapEntry<F, fn()>
where
    M: Collect,
{
    fn visit(&self) {
        (self.handler)();
    }
}

// Generics: `S` strategy, `M` emit tree. Empty leaf.
impl<S, M> PathNode<S, M> for ()
where
    S: PathStrategy + ?Sized,
    M: Collect,
{
    fn visit(&self) {}
}

// Generics: `S` strategy, `M` emit tree; `E0`…`E7` child slots.

impl<S, M, E0> PathNode<S, M> for (E0,)
where
    S: PathStrategy + ?Sized,
    M: Collect,
    E0: PathNode<S, M>,
{
    fn visit(&self) {
        self.0.visit();
    }
}

impl<S, M, E0, E1> PathNode<S, M> for (E0, E1)
where
    S: PathStrategy + ?Sized,
    M: Collect,
    E0: PathNode<S, M>,
    E1: PathNode<S, M>,
{
    fn visit(&self) {
        self.0.visit();
        self.1.visit();
    }
}

// Generics: `S` strategy, `M` emit const, `E0` event 0, `E1` event 1, `E2` event 2.
impl<S, M, E0, E1, E2> PathNode<S, M> for (E0, E1, E2)
where
    S: PathStrategy + ?Sized,
    M: Collect,
    E0: PathNode<S, M>,
    E1: PathNode<S, M>,
    E2: PathNode<S, M>,
{
    fn visit(&self) {
        self.0.visit();
        self.1.visit();
        self.2.visit();
    }
}

// Generics: `S` strategy, `M` emit const, `E0` event 0, `E1` event 1, `E2` event 2, `E3` event 3.
impl<S, M, E0, E1, E2, E3> PathNode<S, M> for (E0, E1, E2, E3)
where
    S: PathStrategy + ?Sized,
    M: Collect,
    E0: PathNode<S, M>,
    E1: PathNode<S, M>,
    E2: PathNode<S, M>,
    E3: PathNode<S, M>,
{
    fn visit(&self) {
        self.0.visit();
        self.1.visit();
        self.2.visit();
        self.3.visit();
    }
}

// Generics: `S` strategy, `M` emit const, `E0` event 0, `E1` event 1, `E2` event 2, `E3` event 3, `E4` event 4.
impl<S, M, E0, E1, E2, E3, E4> PathNode<S, M> for (E0, E1, E2, E3, E4)
where
    S: PathStrategy + ?Sized,
    M: Collect,
    E0: PathNode<S, M>,
    E1: PathNode<S, M>,
    E2: PathNode<S, M>,
    E3: PathNode<S, M>,
    E4: PathNode<S, M>,
{
    fn visit(&self) {
        self.0.visit();
        self.1.visit();
        self.2.visit();
        self.3.visit();
        self.4.visit();
    }
}

// Generics: `S` strategy, `M` emit const, `E0` event 0, `E1` event 1, `E2` event 2, `E3` event 3, `E4` event 4, `E5` event 5.
impl<S, M, E0, E1, E2, E3, E4, E5> PathNode<S, M> for (E0, E1, E2, E3, E4, E5)
where
    S: PathStrategy + ?Sized,
    M: Collect,
    E0: PathNode<S, M>,
    E1: PathNode<S, M>,
    E2: PathNode<S, M>,
    E3: PathNode<S, M>,
    E4: PathNode<S, M>,
    E5: PathNode<S, M>,
{
    fn visit(&self) {
        self.0.visit();
        self.1.visit();
        self.2.visit();
        self.3.visit();
        self.4.visit();
        self.5.visit();
    }
}

// Generics: `S` strategy, `M` emit const, `E0` event 0, `E1` event 1, `E2` event 2, `E3` event 3, `E4` event 4, `E5` event 5, `E6` event 6.
impl<S, M, E0, E1, E2, E3, E4, E5, E6> PathNode<S, M> for (E0, E1, E2, E3, E4, E5, E6)
where
    S: PathStrategy + ?Sized,
    M: Collect,
    E0: PathNode<S, M>,
    E1: PathNode<S, M>,
    E2: PathNode<S, M>,
    E3: PathNode<S, M>,
    E4: PathNode<S, M>,
    E5: PathNode<S, M>,
    E6: PathNode<S, M>,
{
    fn visit(&self) {
        self.0.visit();
        self.1.visit();
        self.2.visit();
        self.3.viДавай дальше,sit();
        self.4.visit();
        self.5.visit();
        self.6.visit();
    }
}

// Generics: `S` strategy, `M` emit const, `E0` event 0, `E1` event 1, `E2` event 2, `E3` event 3, `E4` event 4, `E5` event 5, `E6` event 6, `E7` event 7.
impl<S, M, E0, E1, E2, E3, E4, E5, E6, E7> PathNode<S, M> for (E0, E1, E2, E3, E4, E5, E6, E7)
where
    S: PathStrategy + ?Sized,
    M: Collect,
    E0: PathNode<S, M>,
    E1: PathNode<S, M>,
    E2: PathNode<S, M>,
    E3: PathNode<S, M>,
    E4: PathNode<S, M>,
    E5: PathNode<S, M>,
    E6: PathNode<S, M>,
    E7: PathNode<S, M>,
{
    fn visit(&self) {
        self.0.visit();
        self.1.visit();
        self.2.visit();
        self.3.visit();
        self.4.visit();
        self.5.visit();
        self.6.visit();
        self.7.visit();
    }
}

// Generics: `S` strategy, `M` emit tree; `T` carrier ([`CarrierDesc`]).
impl<S, M, T> PathNode<S, M> for T
where
    S: PathStrategy + ?Sized,
    M: Collect,
    T: CarrierDesc,
    T::Inner: PathNode<S, M>,
{
    fn visit(&self) {
        self.carrier_inner().visit();
    }
}
