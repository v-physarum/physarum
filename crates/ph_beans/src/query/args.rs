//! Which beans a resource claim names, and the view the init function sees.

use crate::borrow::At;
use crate::descriptor::live::{Idle, Live};
use crate::descriptor::{Dyn, DynView, Get, Mut};
use crate::filter::And;

/// Resource claim `B` can project.
///
/// Leaves carry `B: At<P>`. Lists only require each part to be a [`Claim`].
pub trait Claim<B> {
    /// Absolute indexes of the claimed beans.
    type Indexes;
}

impl<B> Claim<B> for () {
    type Indexes = ();
}

impl<B, P> Claim<B> for Get<P>
where
    B: At<P>,
{
    type Indexes = B::Ix;
}

impl<B, P> Claim<B> for Mut<P>
where
    B: At<P>,
{
    type Indexes = B::Ix;
}

impl<B, P, V: ?Sized> Claim<B> for Dyn<Get<P>, V>
where
    B: At<P>,
{
    type Indexes = B::Ix;
}

impl<B, P, V: ?Sized> Claim<B> for Dyn<Mut<P>, V>
where
    B: At<P>,
{
    type Indexes = B::Ix;
}

impl<B, A, C> Claim<B> for And<A, C>
where
    A: Claim<B>,
    C: Claim<B>,
{
    type Indexes = (A::Indexes, C::Indexes);
}

impl<B, A, C> Claim<B> for (A, C)
where
    A: Claim<B>,
    C: Claim<B>,
{
    type Indexes = (A::Indexes, C::Indexes);
}

impl<B, C> Claim<B> for Live<C>
where
    C: Claim<B>,
{
    type Indexes = C::Indexes;
}

impl<B, C> Claim<B> for Idle<C>
where
    C: Claim<B>,
{
    type Indexes = C::Indexes;
}

/// Argument view an init function receives for a claim.
pub trait ArgsView {
    /// Borrowed arguments for one init call.
    type View<'a>
    where
        Self: 'a;
}

impl ArgsView for () {
    type View<'a>
        = ()
    where
        Self: 'a;
}

impl<T> ArgsView for Get<T> {
    type View<'a>
        = &'a T
    where
        Self: 'a;
}

impl<T> ArgsView for Mut<T> {
    type View<'a>
        = &'a mut T
    where
        Self: 'a;
}

impl<T, V: ?Sized> ArgsView for Dyn<Get<T>, V>
where
    V: DynView<T>,
{
    type View<'a>
        = &'a V
    where
        Self: 'a;
}

impl<T, V: ?Sized> ArgsView for Dyn<Mut<T>, V>
where
    V: DynView<T>,
{
    type View<'a>
        = &'a mut V
    where
        Self: 'a;
}

impl<A, B> ArgsView for And<A, B>
where
    A: ArgsView,
    B: ArgsView,
{
    type View<'a>
        = (A::View<'a>, B::View<'a>)
    where
        Self: 'a;
}

impl<A, B> ArgsView for (A, B)
where
    A: ArgsView,
    B: ArgsView,
{
    type View<'a>
        = (A::View<'a>, B::View<'a>)
    where
        Self: 'a;
}

impl<C> ArgsView for Live<C>
where
    C: ArgsView,
{
    type View<'a>
        = C::View<'a>
    where
        Self: 'a;
}

impl<C> ArgsView for Idle<C>
where
    C: ArgsView,
{
    type View<'a>
        = C::View<'a>
    where
        Self: 'a;
}
