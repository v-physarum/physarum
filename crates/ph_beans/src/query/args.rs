//! Which beans a resource claim names, and the view the init function sees.

use crate::bean::{ApplyProxy, Proxied, Via};
use crate::borrow::At;
use crate::descriptor::live::{Idle, Live};
use crate::descriptor::{Dyn, DynView, Get, Mut};
use crate::filter::And;

/// Own [`At`] on the pipeline bean.
pub struct Own;

/// [`At`] on the pipeline bean's proxy (rule 3).
pub struct FromProxy;

/// Resource claim that the pipeline bean can see, either on its own [`At`]
/// list or on its proxy's [`At`] list.
pub trait ClaimVia<B, K> {
    /// Absolute indexes of the claimed beans.
    type Indexes;
}

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
    P: Proxied,
{
    type Indexes = B::Ix;
}

impl<B, P> Claim<B> for Mut<P>
where
    B: At<P>,
    P: Proxied,
{
    type Indexes = B::Ix;
}

impl<B, S> ClaimVia<B, Own> for Get<S>
where
    B: At<S>,
    S: Proxied,
{
    type Indexes = B::Ix;
}

impl<B, S> ClaimVia<B, Own> for Mut<S>
where
    B: At<S>,
    S: Proxied,
{
    type Indexes = B::Ix;
}

impl<B, S, P> ClaimVia<B, FromProxy> for Get<S>
where
    B: Proxied<Proxy = P>,
    P: At<S>,
    S: Proxied,
{
    type Indexes = P::Ix;
}

impl<B, S, P> ClaimVia<B, FromProxy> for Mut<S>
where
    B: Proxied<Proxy = P>,
    P: At<S>,
    S: Proxied,
{
    type Indexes = P::Ix;
}

impl<B, A, C, Ka, Kc> ClaimVia<B, (Ka, Kc)> for And<A, C>
where
    A: ClaimVia<B, Ka>,
    C: ClaimVia<B, Kc>,
{
    type Indexes = (A::Indexes, C::Indexes);
}

impl<B, A, C, Ka, Kc> ClaimVia<B, (Ka, Kc)> for (A, C)
where
    A: ClaimVia<B, Ka>,
    C: ClaimVia<B, Kc>,
{
    type Indexes = (A::Indexes, C::Indexes);
}

impl<B, P, S> Claim<B> for Via<P, Get<S>>
where
    S: Proxied<Proxy = P>,
{
    type Indexes = ();
}

impl<B, P, S> Claim<B> for Via<P, Mut<S>>
where
    S: Proxied<Proxy = P>,
{
    type Indexes = ();
}

impl<A: ApplyProxy, B: ApplyProxy> ApplyProxy for And<A, B> {
    type Wrapped = And<A::Wrapped, B::Wrapped>;
}

impl<A: ApplyProxy, B: ApplyProxy> ApplyProxy for (A, B) {
    type Wrapped = (A::Wrapped, B::Wrapped);
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

impl<P, T> ArgsView for Via<P, Get<T>> {
    type View<'a>
        = &'a T
    where
        Self: 'a;
}

impl<P, T> ArgsView for Via<P, Mut<T>> {
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
