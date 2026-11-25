//! Wait, spine and parent checks over the filter AST: blankets over
//! [`NodeForm::Body`] (rule 23).

use super::flag::{No, OrWith, Yes};
use crate::bean::EventInfo;
use crate::filter::On;
use crate::filter::shape::{Here, IdleEmit, NodeForm, PairBody, Pass, Read, Skip};

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

// Generics: `T` operator.
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
    message = "pipeline not ready: Descriptor::Status = `FilterError<1>` (tip `{Self}`)",
    label = "fix Descriptor::Status",
    note = "a nested event must list the outer event in Parents; FilterError<1> — docs/errors.md"
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
    message = "pipeline not ready: Descriptor::Status = `FilterError<1>` (tip `{Self}`)",
    label = "fix Descriptor::Status",
    note = "one Status for the whole chain; FilterError<1> — docs/errors.md"
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
