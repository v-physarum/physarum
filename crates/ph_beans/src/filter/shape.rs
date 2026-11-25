//! Spine, wait, and parent checks over the filter AST.
//!
//! [`NodeForm`] names each operator once. [`HasMut`], [`SpineTail`], and
//! [`Under`] are blankets over that body (rule 23).

use core::marker::PhantomData;

use crate::bean::BeanInfo;
use crate::descriptor::{Dyn, Get, Mut, State};
use crate::filter::{And, Not, On, Or};

pub use super::flag::{No, Yes};
pub use crate::filter::nest::{ApplyFilter, HasMut, NestOk, Spine, Under};

/// One description of a filter operator. Walks read [`Body`](NodeForm::Body).
#[doc(hidden)]
pub trait NodeForm {
    /// Leaf flag, pair, or the node that carries this operator's children.
    type Body;
}

/// `()` — no wait, not an operand.
#[doc(hidden)]
pub struct Pass;

/// [`Get`](crate::Get) — no wait, is an operand.
#[doc(hidden)]
pub struct Read;

/// Join kind for [`PairBody`]: both children ([`And`](crate::And)).
#[doc(hidden)]
pub struct JoinBoth;

/// Join kind for [`PairBody`]: either child ([`Or`](crate::Or)).
#[doc(hidden)]
pub struct JoinEither;

/// Pair of children. `K` is [`JoinBoth`] or [`JoinEither`].
/// Generics: `A` left child, `B` right child, `K` join kind.
#[doc(hidden)]
pub struct PairBody<A, B, K> {
    _markers: PhantomData<(A, B, K)>,
}

/// Child negated. [`Not`](crate::Not).
#[doc(hidden)]
pub struct Skip<A> {
    _markers: PhantomData<A>,
}

/// Event `E` around inner path `I`. [`On`](crate::On).
/// Generics: `E` event, `I` inner path.
#[doc(hidden)]
pub struct Here<E, I> {
    _markers: PhantomData<(E, I)>,
}

impl NodeForm for () {
    type Body = Pass;
}

// Generics: `T` bean ([`BeanInfo`]).
impl<T: BeanInfo> NodeForm for Get<T> {
    type Body = Read;
}

// Generics: `T` bean ([`BeanInfo`]).
impl<T: BeanInfo> NodeForm for Mut<T> {
    type Body = Yes;
}

// Generics: `T` bean. `*T::T` is [`Get`]`<`[`State`]`<T>>`: the state in `T`'s slot.
impl<T: BeanInfo> NodeForm for Get<State<T>> {
    type Body = Read;
}

// Generics: `T` bean. `*T::MUT` is [`Mut`]`<`[`State`]`<T>>`: the state in `T`'s slot.
impl<T: BeanInfo> NodeForm for Mut<State<T>> {
    type Body = Yes;
}

// Generics: `P` bean, `V` view.
impl<P, V: ?Sized> NodeForm for Dyn<Get<P>, V> {
    type Body = Read;
}

// Generics: `P` bean, `V` view.
impl<P, V: ?Sized> NodeForm for Dyn<Mut<P>, V> {
    type Body = Yes;
}

// Generics: `A` left child, `B` right child.
impl<A, B> NodeForm for And<A, B> {
    type Body = PairBody<A, B, JoinBoth>;
}

// Generics: `A` left child, `B` right child.
impl<A, B> NodeForm for Or<A, B> {
    type Body = PairBody<A, B, JoinEither>;
}

// Generics: `A` child.
impl<A> NodeForm for Not<A> {
    type Body = Skip<A>;
}

// Generics: `E` event, `I` inner path.
impl<E, I> NodeForm for On<E, I> {
    type Body = Here<E, I>;
}

/// Leaves whose mask, spine, and parent check are empty.
#[doc(hidden)]
pub trait IdleEmit {}

impl IdleEmit for Pass {}

impl IdleEmit for Read {}

impl IdleEmit for Yes {}
