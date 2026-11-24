//! Spine, wait, and parent checks over the filter AST.
//!
//! [`NodeForm`] names each operator once. [`HasMut`], [`SpineTail`], and
//! [`Under`] are blankets over that body (rule 23).

use core::marker::PhantomData;

use crate::bean::{BeanInfo, EventInfo};
use crate::descriptor::{Dyn, Get, Mut, State};
use crate::filter::{And, Not, On, Or};

use super::flag::OrWith;

pub use super::flag::{No, Yes};

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
pub struct PairBody<A, B, K>(PhantomData<(A, B, K)>);

/// Child negated. [`Not`](crate::Not).
#[doc(hidden)]
pub struct Skip<A>(PhantomData<A>);

/// Event `E` around inner path `I`. [`On`](crate::On).
/// Generics: `E` event, `I` inner path.
#[doc(hidden)]
pub struct Here<E, I>(PhantomData<(E, I)>);

/// Event-state claim. `W` is the wait flag ([`No`] / [`Yes`]).
#[doc(hidden)]
pub struct Scope<P, W>(PhantomData<(P, W)>);

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

// Generics: `T` event bean. `*T::T` is [`Get`]`<`[`State`]`<T>>`.
impl<T: BeanInfo> NodeForm for Get<State<T>> {
    type Body = Scope<T, No>;
}

// Generics: `T` event bean. `*T::MUT` is [`Mut`]`<`[`State`]`<T>>`.
impl<T: BeanInfo> NodeForm for Mut<State<T>> {
    type Body = Scope<T, Yes>;
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

// Generics: `P` event bean, `W` wait flag.
impl<P, W> IdleEmit for Scope<P, W> {}

/// [`HasMut`] projection of a [`NodeForm::Body`].
#[doc(hidden)]
pub trait AsMut {
    /// [`Yes`] when this body waits.
    type Out;
}

/// Bodies with [`HasMut::Mut`] = [`No`] ([`Pass`] / [`Read`]).
trait NoWait {}

impl NoWait for Pass {}

impl NoWait for Read {}

// Generics: `T` tick / type / tip.
impl<T: NoWait> AsMut for T {
    type Out = No;
}

impl AsMut for Yes {
    type Out = Yes;
}

// Generics: `P` event bean, `W` wait flag.
impl<P, W> AsMut for Scope<P, W> {
    type Out = W;
}

// Generics: `A` left child, `B` right child, `K` join kind — one walk for And / Or bodies.
impl<A, B, K> AsMut for PairBody<A, B, K>
where
    A: HasMut,
    B: HasMut,
    A::Mut: OrWith<B::Mut>,
{
    type Out = <A::Mut as OrWith<B::Mut>>::Out;
}

// Generics: `A` child.
impl<A: HasMut> AsMut for Skip<A> {
    type Out = A::Mut;
}

// Generics: `E` event, `I` inner path.
impl<E, I: HasMut> AsMut for Here<E, I> {
    type Out = I::Mut;
}

/// [`SpineTail`] projection of a [`NodeForm::Body`].
#[doc(hidden)]
pub trait AsSpine {
    /// Path with `O` appended.
    type With<O>;
}

impl<T: IdleEmit> AsSpine for T {
    type With<O> = O;
}

// Generics: `A` left child, `B` right child, `K` join kind.
impl<A, B, K> AsSpine for PairBody<A, B, K>
where
    A: SpineTail,
    B: SpineTail,
{
    type With<O> = A::With<B::With<O>>;
}

// Generics: `A` child.
impl<A: SpineTail> AsSpine for Skip<A> {
    type With<O> = A::With<O>;
}

// Generics: `E` event, `I` inner path.
impl<E, I: SpineTail> AsSpine for Here<E, I> {
    type With<O> = I::With<(E, O)>;
}

/// [`Under`] projection of a [`NodeForm::Body`].
trait AsUnder<P> {}

// Generics: `P` parent event, `T` idle body ([`IdleEmit`]).
impl<P, T: IdleEmit> AsUnder<P> for T {}

// Generics: `P` parent event, `A` left child, `B` right child, `K` join kind.
impl<P, A, B, K> AsUnder<P> for PairBody<A, B, K>
where
    A: Under<P>,
    B: Under<P>,
{
}

// Generics: `P` parent event, `A` child.
impl<P, A: Under<P>> AsUnder<P> for Skip<A> {}

// Generics: `P` parent event, `C` child event, `I` inner path.
impl<P, C, I> AsUnder<P> for Here<C, I>
where
    C: EventInfo<Parents = P>,
    I: Under<C>,
{
}

/// Whether this node waits on a `Mut` claim.
#[doc(hidden)]
pub trait HasMut {
    /// [`Yes`] when a wait marker is present.
    type Mut;
}

// Generics: `T` operator ([`NodeForm`]).
impl<T> HasMut for T
where
    T: NodeForm,
    T::Body: AsMut,
{
    type Mut = <T::Body as AsMut>::Out;
}

/// Inverted event path. Innermost event first. Waits are not events.
#[doc(hidden)]
pub trait SpineTail {
    /// Path with `O` appended.
    type With<O>;
}

// Generics: `T` operator ([`NodeForm`]).
impl<T> SpineTail for T
where
    T: NodeForm,
    T::Body: AsSpine,
{
    type With<O> = <T::Body as AsSpine>::With<O>;
}

/// Event path of a root `On` node.
pub trait Spine {
    /// Innermost event first.
    type Path;
}

// Generics: `E` event, `I` inner / index.
impl<E, I> Spine for On<E, I>
where
    I: SpineTail,
{
    type Path = I::With<E>;
}

/// Nested `On<C, _>` lists `P` in `C::Parents`.
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `ErrBadEvent` (tip `{Self}`)",
    label = "fix Descriptor::Status",
    note = "a nested event must list the outer event in Parents"
)]
// Generics: `P` parent.
pub trait Under<P> {}

// Generics: `P` parent event, `T` operator ([`NodeForm`]).
impl<P, T> Under<P> for T
where
    T: NodeForm,
    T::Body: AsUnder<P>,
{
}

/// The event node is well formed.
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `ErrBadEvent` (tip `{Self}`)",
    label = "fix Descriptor::Status",
    note = "one Status for the whole chain (build, events, accessors, DI)"
)]
pub trait NestOk {}

// Generics: `E` event, `I` inner / index.
impl<E, I> NestOk for On<E, I>
where
    E: EventInfo,
    I: Under<E>,
{
}

/// What `.filter` does to `E` and the live args.
/// Generics: `E` event, `A` live args.
pub trait ApplyFilter<E, A> {
    /// Event scope after the filter.
    type NextE;
    /// Args after a wait, or the same args.
    type NextArgs;
}
