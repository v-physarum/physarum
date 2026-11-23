//! Which beans a resource claim names, and the view the init function sees.

use crate::bean::{ApplyProxy, More, Proxied, ProxyInfo, Target, Via};
use crate::borrow::At;
use crate::descriptor::live::{Idle, Live};
use crate::descriptor::{Dyn, DynView, Get, Mut};
use crate::filter::And;

/// Own [`At`] on the pipeline bean.
pub struct Own;

/// [`At`] on the pipeline bean's direct proxy (rule 3).
pub struct FromProxy;

/// [`At`] on an outer proxy in the chain (rule 3).
pub struct FromChain;

/// `At` visible on a proxy or on a proxy further out in its chain.
#[doc(hidden)]
pub trait Sees<S, K> {
    /// Index of `S` on the proxy that provides it.
    type Ix;
}

impl<P, S> Sees<S, Own> for P
where
    P: At<S>,
{
    type Ix = P::Ix;
}

impl<P, O, S, K> Sees<S, (FromProxy, K)> for P
where
    P: ProxyInfo<Chain = More<O>>,
    O: Sees<S, K>,
{
    type Ix = <O as Sees<S, K>>::Ix;
}

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
    P: Target + Proxied,
{
    type Indexes = B::Ix;
}

impl<B, P> Claim<B> for Mut<P>
where
    B: At<P>,
    P: Target + Proxied,
{
    type Indexes = B::Ix;
}

impl<B, S> ClaimVia<B, Own> for Get<S>
where
    B: At<S>,
    S: Target + Proxied,
{
    type Indexes = B::Ix;
}

impl<B, S> ClaimVia<B, Own> for Mut<S>
where
    B: At<S>,
    S: Target + Proxied,
{
    type Indexes = B::Ix;
}

impl<B, S, P> ClaimVia<B, FromProxy> for Get<S>
where
    B: Proxied<Proxy = P>,
    P: At<S>,
    S: Target + Proxied,
{
    type Indexes = P::Ix;
}

impl<B, S, P> ClaimVia<B, FromProxy> for Mut<S>
where
    B: Proxied<Proxy = P>,
    P: At<S>,
    S: Target + Proxied,
{
    type Indexes = P::Ix;
}

impl<B, S, P, O, K> ClaimVia<B, (FromChain, K)> for Get<S>
where
    B: Proxied<Proxy = P>,
    P: ProxyInfo<Chain = More<O>>,
    O: Sees<S, K>,
    S: Target + Proxied,
{
    type Indexes = <O as Sees<S, K>>::Ix;
}

impl<B, S, P, O, K> ClaimVia<B, (FromChain, K)> for Mut<S>
where
    B: Proxied<Proxy = P>,
    P: ProxyInfo<Chain = More<O>>,
    O: Sees<S, K>,
    S: Target + Proxied,
{
    type Indexes = <O as Sees<S, K>>::Ix;
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
    S: Target + Proxied<Proxy = P>,
{
    type Indexes = ();
}

impl<B, P, S> Claim<B> for Via<P, Mut<S>>
where
    S: Target + Proxied<Proxy = P>,
{
    type Indexes = ();
}

impl<B, P, Q, C> Claim<B> for Via<P, Via<Q, C>>
where
    Via<Q, C>: Claim<B>,
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

impl<P, C> ArgsView for Via<P, C>
where
    C: ArgsView,
{
    type View<'a>
        = C::View<'a>
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
