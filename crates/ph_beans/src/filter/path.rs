//! Hierarchical emit walk: DELTA gates + strategy structs.
//!
//! Design **rule 8**: [`PathStrategy`] is the trait; [`Sparse`] / [`Dense`] are
//! the variants; siblings are a flat leaf ≤8 (often inside [`List`](crate::List)).
//! Every node is walked by [`Hand`](crate::context::list::Hand). [`PathNode`] is
//! that walk under one strategy, beside [`Drive`](crate::query::schedule::Drive).
//!
//! Each gate intersects [`PathStep::DELTA`] with the emit const `M::MASK`.
//! A miss deletes the subtree. A hit runs children with no runtime bit test
//! ([`Dense`]) or lets each child repeat that const test ([`Sparse`]).

use core::marker::PhantomData;

use crate::context::list::{Hand, Visit};
use crate::context::maps::MapEntry;
use crate::context::sys::MapSys;
use crate::filter::EmitMask;
use crate::filter::eval::{Collect, ConstMatch, MatchEmit};
use crate::filter::slot::{EmitBit, EmitIx};

/// Bits this path step adds relative to its parent (one event bit, typically).
#[doc(hidden)]
pub trait PathStep {
    /// Gate mask: `mask & DELTA == DELTA` to enter.
    const DELTA: EmitMask;
}

// Generics: `E` event bit.
impl<E: EmitBit> PathStep for E {
    const DELTA: EmitMask = EmitMask::from_bit(<E as EmitIx>::BIT);
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

    /// Runs one map leaf after this strategy's gate policy.
    ///
    /// Generics: `M` emit tree, `F` filter, `H` handler.
    fn run_map<M, F, H>(handler: &H)
    where
        M: Collect,
        F: MatchEmit,
        H: MapSys<()>;
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

impl PathStrategy for Sparse {
    fn run_map<M, F, H>(handler: &H)
    where
        M: Collect,
        F: MatchEmit,
        H: MapSys<()>,
    {
        if const { F::Against::<M>::MATCHES } {
            handler.run(&mut ());
        }
    }
}

impl PathStrategy for Dense {
    fn run_map<M, F, H>(handler: &H)
    where
        M: Collect,
        F: MatchEmit,
        H: MapSys<()>,
    {
        handler.run(&mut ());
    }
}

/// Path behavior placed in [`Hand`]. Generics: `S` strategy, `M` emit const.
struct PathTag<S, M> {
    _markers: PhantomData<(S, M)>,
}

// Generics: `S` strategy, `M` emit tree.
impl<S, M> Copy for PathTag<S, M> {}

// Generics: `S` strategy, `M` emit tree.
impl<S, M> Clone for PathTag<S, M> {
    fn clone(&self) -> Self {
        *self
    }
}

// Generics: `S` strategy, `M` emit tree. A gate tree runs with no resources.
impl<S, M> Visit<()> for PathTag<S, M>
where
    S: PathStrategy,
    M: Collect,
{
    fn on_entry<F, H>(&self, entry: &MapEntry<F, H>, _res: &mut ())
    where
        F: MatchEmit + Collect,
        H: MapSys<()>,
    {
        S::run_map::<M, F, H>(&entry.handler);
    }

    fn on_gate<P, St, C>(&self, gate: &Gate<P, St, C>, res: &mut ())
    where
        P: PathStep,
        St: PathStrategy,
        C: Hand<()>,
    {
        if const { M::MASK.contains(P::DELTA) } {
            gate.children
                .hand(PathTag::<St, M> { _markers: PhantomData }, res);
        }
    }
}

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

// Generics: `R` resources, `P` path step, `S` strategy, `C` children.
impl<R, P, S, C> Hand<R> for Gate<P, S, C>
where
    P: PathStep,
    S: PathStrategy,
    C: Hand<R>,
{
    fn hand<V>(&self, op: V, res: &mut R)
    where
        V: Copy + Visit<R>,
    {
        op.on_gate(self, res);
    }
}

// Every node the list walk can hand on is a path node: a gate, a map leaf,
// a carrier, or a tuple list. Visiting it places [`PathTag`].
// Generics: `S` strategy, `M` emit tree, `T` node ([`Hand`]).
impl<S, M, T> PathNode<S, M> for T
where
    S: PathStrategy,
    M: Collect,
    T: Hand<()>,
{
    fn visit(&self) {
        self.hand(PathTag::<S, M> { _markers: PhantomData }, &mut ());
    }
}
