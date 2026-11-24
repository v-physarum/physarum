//! `.filter` keeps `E` for predicates and replaces it for `On`.
//!
//! Each node states that policy on [`FilterDesc`]. The step is one blanket.

use core::marker::PhantomData;

use crate::descriptor::live::Rewait;
use crate::filter::On;
use crate::filter::shape::{
    ApplyFilter, HasMut, IdleEmit, NestOk, NodeForm, PairBody, Skip, Spine,
};

/// Event scope stays the previous `E`.
#[doc(hidden)]
pub struct Keep;

/// Event scope becomes the path `P`.
/// Generics: `P` path.
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

// Generics: `E` event, `P` path.
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

/// Predicate bodies leave `E` unchanged. [`Here`](crate::filter::shape::Here) does not.
trait Keeps {}

// Generics: `T` tick / type / tip.
impl<T: IdleEmit> Keeps for T {}

// Generics: `A` left child, `B` right child, `K` join kind.
impl<A, B, K> Keeps for PairBody<A, B, K> {}

// Generics: `A` child.
impl<A> Keeps for Skip<A> {}

/// Predicate nodes leave `E` unchanged. `On` does not implement this.
trait KeepsScope {}

// Generics: `F` operator ([`NodeForm`]).
impl<F> KeepsScope for F
where
    F: NodeForm,
    F::Body: Keeps,
{
}

impl<F> FilterDesc for F
where
    F: KeepsScope + HasMut,
{
    type Mut = F::Mut;
    type Scope = Keep;
}

// Generics: `E` event, `I` inner / index.
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
