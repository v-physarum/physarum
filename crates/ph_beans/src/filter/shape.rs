//! Spine, wait, and parent checks over the filter AST.

use crate::bean::EventInfo;
use crate::descriptor::{Get, Mut};
use crate::filter::{And, Not, On, Or};

use super::flag::OrFlag;

pub use super::flag::{No, Yes};

/// Whether this node waits on a `Mut` claim.
#[doc(hidden)]
pub trait HasMut {
    /// [`Yes`] when a wait marker is present.
    type Mut;
}

impl HasMut for () {
    type Mut = No;
}

impl<T> HasMut for Get<T> {
    type Mut = No;
}

impl<T> HasMut for Mut<T> {
    type Mut = Yes;
}

impl<A, B> HasMut for And<A, B>
where
    A: HasMut,
    B: HasMut,
    (A::Mut, B::Mut): OrFlag,
{
    type Mut = <(A::Mut, B::Mut) as OrFlag>::Out;
}

impl<A, B> HasMut for Or<A, B>
where
    A: HasMut,
    B: HasMut,
    (A::Mut, B::Mut): OrFlag,
{
    type Mut = <(A::Mut, B::Mut) as OrFlag>::Out;
}

impl<A> HasMut for Not<A>
where
    A: HasMut,
{
    type Mut = A::Mut;
}

impl<E, I> HasMut for On<E, I>
where
    I: HasMut,
{
    type Mut = I::Mut;
}

/// Inverted event path. Innermost event first. Waits are not events.
#[doc(hidden)]
pub trait SpineTail {
    /// Path with `O` appended.
    type With<O>;
}

impl SpineTail for () {
    type With<O> = O;
}

impl<T> SpineTail for Get<T> {
    type With<O> = O;
}

impl<T> SpineTail for Mut<T> {
    type With<O> = O;
}

impl<A, B> SpineTail for And<A, B>
where
    A: SpineTail,
    B: SpineTail,
{
    type With<O> = A::With<B::With<O>>;
}

impl<A, B> SpineTail for Or<A, B>
where
    A: SpineTail,
    B: SpineTail,
{
    type With<O> = A::With<B::With<O>>;
}

impl<A> SpineTail for Not<A>
where
    A: SpineTail,
{
    type With<O> = A::With<O>;
}

impl<E, I> SpineTail for On<E, I>
where
    I: SpineTail,
{
    type With<O> = I::With<(E, O)>;
}

/// Event path of a root `On` node.
pub trait Spine {
    /// Innermost event first.
    type Path;
}

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
pub trait Under<P> {}

impl<P> Under<P> for () {}

impl<P, T> Under<P> for Get<T> {}

impl<P, T> Under<P> for Mut<T> {}

impl<P, A, B> Under<P> for And<A, B>
where
    A: Under<P>,
    B: Under<P>,
{
}

impl<P, A, B> Under<P> for Or<A, B>
where
    A: Under<P>,
    B: Under<P>,
{
}

impl<P, A> Under<P> for Not<A> where A: Under<P> {}

impl<P, C, I> Under<P> for On<C, I>
where
    C: EventInfo<Parents = P>,
    I: Under<C>,
{
}

/// The event node is well formed.
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `ErrBadEvent` (tip `{Self}`)",
    label = "fix Descriptor::Status",
    note = "one Status for the whole chain (build, events, accessors, DI)"
)]
pub trait NestOk {}

impl<E, I> NestOk for On<E, I>
where
    E: EventInfo,
    I: Under<E>,
{
}

/// What `.filter` does to `E` and the live args.
pub trait ApplyFilter<E, A> {
    /// Event scope after the filter.
    type NextE;
    /// Args after a wait, or the same args.
    type NextArgs;
}
