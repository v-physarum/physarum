//! `.filter` keeps `E` for predicates and replaces it for `On`.
//!
//! Each node states that policy on [`FilterDesc`]. The step is one blanket.

use core::marker::PhantomData;

use crate::descriptor::live::Rewait;
use crate::descriptor::{Get, Mut};
use crate::filter::shape::{ApplyFilter, HasMut, NestOk, Spine};
use crate::filter::{And, Not, On, Or};

/// Event scope stays the previous `E`.
#[doc(hidden)]
pub struct Keep;

/// Event scope becomes the path `P`.
#[doc(hidden)]
pub struct Replace<P> {
    _markers: PhantomData<P>,
}

/// How a scope policy rewrites the previous event path.
#[doc(hidden)]
pub trait ScopeOf<E> {
    /// Event scope after the filter.
    type NextE;
}

impl<E> ScopeOf<E> for Keep {
    type NextE = E;
}

impl<E, P> ScopeOf<E> for Replace<P> {
    type NextE = P;
}

/// Per-node policy for `.filter`: wait flag and whether `E` is replaced.
///
/// This is not claim-list [`OpDesc`](crate::OpDesc).
#[doc(hidden)]
pub trait FilterDesc {
    /// [`Yes`](crate::filter::Yes) when the node waits.
    type Mut;
    /// [`Keep`] or [`Replace`].
    type Scope;
}

/// Predicate nodes leave `E` unchanged. `On` does not implement this.
trait KeepsScope {}

impl KeepsScope for () {}

impl<T> KeepsScope for Get<T> {}

impl<T> KeepsScope for Mut<T> {}

impl<A, B> KeepsScope for And<A, B> {}

impl<A, B> KeepsScope for Or<A, B> {}

impl<A> KeepsScope for Not<A> {}

impl<F> FilterDesc for F
where
    F: KeepsScope + HasMut,
{
    type Mut = F::Mut;
    type Scope = Keep;
}

impl<E, I> FilterDesc for On<E, I>
where
    On<E, I>: HasMut + Spine + NestOk,
{
    type Mut = <On<E, I> as HasMut>::Mut;
    type Scope = Replace<<On<E, I> as Spine>::Path>;
}

impl<E, A, F> ApplyFilter<E, A> for F
where
    F: FilterDesc,
    F::Scope: ScopeOf<E>,
    A: Rewait<F::Mut>,
{
    type NextE = <F::Scope as ScopeOf<E>>::NextE;
    type NextArgs = A::Out;
}
