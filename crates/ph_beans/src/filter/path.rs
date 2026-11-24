//! Hierarchical emit walk: DELTA gates + strategy structs.
//!
//! Design **rule 8**: [`PathStrategy`] is the trait; [`Sparse`] / [`Dense`] are
//! the variants; siblings are a flat leaf ≤8 (often inside [`List`](crate::List));
//! carriers share [`Invoke`](crate::context::list::Invoke); [`PathNode`] is one blanket.
//! Beside flat [`Invoke`](crate::context::list::Invoke) /
//! [`Drive`](crate::query::schedule::Drive).
//!
//! Each gate intersects [`PathStep::DELTA`] with the emit const `M::MASK`.
//! A miss deletes the subtree. A hit runs children with no runtime bit test
//! ([`Dense`]) or lets each child repeat that const test ([`Sparse`]).

use core::marker::PhantomData;

use crate::context::list::{Invoke, Visit};
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

    /// Runs one map leaf after this strategy's gate policy.
    ///
    /// Generics: `M` emit tree, `F` filter, `H` handler.
    fn run_map<M, F, H>(handler: &H)
    where
        M: Collect,
        F: MatchEmit,
        H: Fn();
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
        H: Fn(),
    {
        if const { F::Against::<M>::MATCHES } {
            handler();
        }
    }
}

impl PathStrategy for Dense {
    fn run_map<M, F, H>(handler: &H)
    where
        M: Collect,
        F: MatchEmit,
        H: Fn(),
    {
        let _ = PhantomData::<(M, F)>;
        handler();
    }
}

/// Path behavior placed in [`Invoke`]. `S` is the strategy, `M` the emit const.
struct PathTag<S, M>(PhantomData<(S, M)>);

impl<S, M> Copy for PathTag<S, M> {}

impl<S, M> Clone for PathTag<S, M> {
    fn clone(&self) -> Self {
        *self
    }
}

// Generics: `S` strategy, `M` emit tree.
impl<S, M> Visit for PathTag<S, M>
where
    S: PathStrategy,
    M: Collect,
{
    fn on_entry<F, H>(&self, entry: &MapEntry<F, H>)
    where
        F: MatchEmit + Collect,
        H: Fn(),
    {
        let _ = self;
        S::run_map::<M, F, H>(&entry.handler);
    }

    fn on_at<F, H>(&self, entry: &crate::context::maps::MapAt<F, H>)
    where
        F: MatchEmit + Collect + crate::context::direct::TipEvent,
        H: crate::context::direct::DirectFn<F::Tip, F>,
    {
        let _ = (self, entry);
    }

    fn on_gate<P, St, C>(&self, gate: &Gate<P, St, C>)
    where
        P: PathStep,
        St: PathStrategy,
        C: Invoke,
    {
        let _ = self;
        gate.path_plan::<M>();
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

// Generics: `P` path step, `S` strategy, `C` children.
impl<P, S, C> Invoke for Gate<P, S, C>
where
    P: PathStep,
    S: PathStrategy,
    C: Invoke,
{
    fn invoke<Op>(&self, op: Op)
    where
        Op: Copy + Visit,
    {
        op.on_gate(self);
    }
}

// One blanket: every [`Invoke`] node is a path node. The op is [`PathTag`].
// Generics: `S` strategy, `M` emit tree, `T` node ([`Invoke`]).
impl<S, M, T> PathNode<S, M> for T
where
    S: PathStrategy,
    M: Collect,
    T: Invoke,
{
    fn visit(&self) {
        self.invoke(PathTag::<S, M>(PhantomData));
    }
}
