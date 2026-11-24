//! Config route for an entity: `Cfg0::T << Cfg1::T`.
//!
//! The left marker is the direct proxy. Each `<<` appends the next outer
//! config. [`Corresponds`] requires that route to be the bean's proxy levels.

use core::ops::Shl;

use crate::context::{Leaf, List};
use crate::descriptor::{Get, LevelDesc};

/// `R` is this bean's proxy levels, nearest config first.
///
/// `Update::T` is one config. `Inner::T << Mid::T << Outer::T` is the chain.
#[diagnostic::on_unimplemented(
    message = "`{R}` is not a config route for `{Self}`",
    note = "write proxies nearest first, as `Cfg0::T << Cfg1::T`, matching the proxy levels"
)]
pub trait Corresponds<R> {}

/// Levels named by a route of `Get` markers.
trait Markers {
    /// The same configs with the markers removed.
    type Levels;
}

impl<A> Markers for Get<A> {
    type Levels = List<Leaf<(A,)>>;
}

impl<A, B> Markers for List<Leaf<(Get<A>, Get<B>)>> {
    type Levels = List<Leaf<(A, B)>>;
}

impl<A, B, C> Markers for List<Leaf<(Get<A>, Get<B>, Get<C>)>> {
    type Levels = List<Leaf<(A, B, C)>>;
}

impl<A, B, C, D> Markers for List<Leaf<(Get<A>, Get<B>, Get<C>, Get<D>)>> {
    type Levels = List<Leaf<(A, B, C, D)>>;
}

#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a config route for the proxy levels `{T}`",
    note = "write proxies nearest first: `Cfg0::T << Cfg1::T`"
)]
trait SameLevels<T> {}

impl<T> SameLevels<T> for T {}

impl<B, R> Corresponds<R> for B
where
    B: LevelDesc,
    R: Markers,
    <R as Markers>::Levels: SameLevels<<B as LevelDesc>::Rows>,
{
}

impl<A, B> Shl<Get<B>> for Get<A> {
    type Output = List<Leaf<(Get<A>, Get<B>)>>;

    fn shl(self, rhs: Get<B>) -> Self::Output {
        List(Leaf((self, rhs)))
    }
}

impl<A, B, C> Shl<Get<C>> for List<Leaf<(Get<A>, Get<B>)>> {
    type Output = List<Leaf<(Get<A>, Get<B>, Get<C>)>>;

    fn shl(self, rhs: Get<C>) -> Self::Output {
        let List(Leaf((left, mid))) = self;
        List(Leaf((left, mid, rhs)))
    }
}

impl<A, B, C, D> Shl<Get<D>> for List<Leaf<(Get<A>, Get<B>, Get<C>)>> {
    type Output = List<Leaf<(Get<A>, Get<B>, Get<C>, Get<D>)>>;

    fn shl(self, rhs: Get<D>) -> Self::Output {
        let List(Leaf((a, b, c))) = self;
        List(Leaf((a, b, c, rhs)))
    }
}
