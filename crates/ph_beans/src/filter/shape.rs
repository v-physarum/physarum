//! Spine, wait, and parent checks over the filter AST.

use crate::bean::{BeanInfo, EventInfo, Marked, Proxied};
use crate::context::{Leaf, List};
use crate::descriptor::{Get, Mut};
use crate::filter::{And, Not, On, Or};

use super::flag::OrWith;

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

impl<P, C> HasMut for List<Leaf<(P, C)>>
where
    C: HasMut,
{
    type Mut = C::Mut;
}

impl<P, O, C> HasMut for List<Leaf<(P, O, C)>>
where
    C: HasMut,
{
    type Mut = C::Mut;
}

impl<P, O0, O1, C> HasMut for List<Leaf<(P, O0, O1, C)>>
where
    C: HasMut,
{
    type Mut = C::Mut;
}

impl<P, O0, O1, O2, C> HasMut for List<Leaf<(P, O0, O1, O2, C)>>
where
    C: HasMut,
{
    type Mut = C::Mut;
}

impl<A, B> HasMut for And<A, B>
where
    A: HasMut,
    B: HasMut,
    A::Mut: OrWith<B::Mut>,
{
    type Mut = <A::Mut as OrWith<B::Mut>>::Out;
}

impl<A, B> HasMut for Or<A, B>
where
    A: HasMut,
    B: HasMut,
    A::Mut: OrWith<B::Mut>,
{
    type Mut = <A::Mut as OrWith<B::Mut>>::Out;
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

/// Parent check that also accepts the proxy's [`EventInfo::Parents`].
///
/// `K` is inferred. [`()`][unit] is a predicate. [`Own`](crate::Own) is the
/// event's own parents. [`FromProxy`](crate::FromProxy) is the proxy's parents.
/// The target's [`EventInfo::Parents`] is not rewritten.
///
/// [unit]: https://doc.rust-lang.org/std/primitive.unit.html
#[doc(hidden)]
pub trait ParentsOk<K> {}

/// `I` lists outer event `P` on its own [`EventInfo::Parents`] or on its proxy.
#[doc(hidden)]
pub trait UnderVia<P, K> {}

impl<P> UnderVia<P, ()> for () {}

impl<P, T> UnderVia<P, ()> for Get<T> {}

impl<P, T> UnderVia<P, ()> for Mut<T> {}

impl<P, A, B, Ka, Kb> UnderVia<P, (Ka, Kb)> for And<A, B>
where
    A: UnderVia<P, Ka>,
    B: UnderVia<P, Kb>,
{
}

impl<P, A, B, Ka, Kb> UnderVia<P, (Ka, Kb)> for Or<A, B>
where
    A: UnderVia<P, Ka>,
    B: UnderVia<P, Kb>,
{
}

impl<P, A, K> UnderVia<P, K> for Not<A> where A: UnderVia<P, K> {}

impl<P, C, I, At, K> UnderVia<P, (crate::Own, At, K)> for On<C, I>
where
    C: EventInfo,
    C::Parents: HasParent<P, At>,
    I: UnderVia<C, K>,
{
}

impl<P, C, I, X, At, K> UnderVia<P, (crate::FromProxy, At, K)> for On<C, I>
where
    C: Proxied<Link: Marked<Target = X>>,
    X: BeanInfo + EventInfo,
    X::Parents: HasParent<P, At>,
    I: UnderVia<C, K>,
{
}

/// `P` is a parent of some outer proxy, not of the direct proxy.
#[doc(hidden)]
pub trait ChainParents<P, K> {}

trait TailParents<P, K> {}

impl<O0, P, At> TailParents<P, (crate::Own, At)> for (O0,)
where
    O0: EventInfo,
    O0::Parents: HasParent<P, At>,
{
}

impl<O0, O1, P, At> TailParents<P, (crate::Own, At)> for (O0, O1)
where
    O0: EventInfo,
    O0::Parents: HasParent<P, At>,
{
}

impl<O0, O1, P, At> TailParents<P, (crate::FromProxy, (crate::Own, At))> for (O0, O1)
where
    O1: EventInfo,
    O1::Parents: HasParent<P, At>,
{
}

impl<O0, O1, O2, P, At> TailParents<P, (crate::Own, At)> for (O0, O1, O2)
where
    O0: EventInfo,
    O0::Parents: HasParent<P, At>,
{
}

impl<O0, O1, O2, P, At> TailParents<P, (crate::FromProxy, (crate::Own, At))> for (O0, O1, O2)
where
    O1: EventInfo,
    O1::Parents: HasParent<P, At>,
{
}

impl<O0, O1, O2, P, At> TailParents<P, (crate::FromProxy, (crate::FromProxy, (crate::Own, At)))>
    for (O0, O1, O2)
where
    O2: EventInfo,
    O2::Parents: HasParent<P, At>,
{
}

/// `P` occupies one slot of this parent list. The slot is `At`.
trait HasParent<P, At> {}

impl<P> HasParent<P, ()> for P {}

impl<P> HasParent<P, crate::Ix0> for (P,) {}

impl<P, R> HasParent<P, crate::Ix0> for (P, R) {}

impl<L, P> HasParent<P, crate::Ix1> for (L, P) {}

impl<P, A, R> HasParent<P, crate::Ix0> for (P, A, R) {}

impl<L, P, R> HasParent<P, crate::Ix1> for (L, P, R) {}

impl<L, A, P> HasParent<P, crate::Ix2> for (L, A, P) {}

impl<P, A, B, R> HasParent<P, crate::Ix0> for (P, A, B, R) {}

impl<L, P, B, R> HasParent<P, crate::Ix1> for (L, P, B, R) {}

impl<L, A, P, R> HasParent<P, crate::Ix2> for (L, A, P, R) {}

impl<L, A, B, P> HasParent<P, crate::Ix3> for (L, A, B, P) {}

impl<X, L, P, K> ChainParents<P, K> for X
where
    X: BeanInfo<Chain = L>,
    L: TailParents<P, K>,
{
}

impl<P, C, I, X, Kc, Ki> UnderVia<P, (crate::FromChain, Kc, Ki)> for On<C, I>
where
    C: Proxied<Link: Marked<Target = X>>,
    X: BeanInfo + ChainParents<P, Kc>,
    I: UnderVia<C, Ki>,
{
}

impl<E, I, K> ParentsOk<K> for On<E, I>
where
    E: EventInfo,
    I: UnderVia<E, K>,
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
