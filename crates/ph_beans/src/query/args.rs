//! Which beans a resource claim names, and the view the init function sees.

use crate::bean::{ApplyProxy, BeanInfo, Marked, More, Proxied, ProxyInfo, Target, Via};
use crate::borrow::{At, Ix0, Ix1, Ix2, Ix3};
use crate::descriptor::live::{Idle, Live};
use crate::descriptor::{Dyn, DynView, Get, Mut};
use crate::filter::And;

/// Own [`At`] on the pipeline bean.
pub struct Own;

/// [`At`] on the direct proxy. The bean is in that proxy's `Provides`.
pub struct FromProxy;

/// [`At`] on an outer proxy. The bean is in that proxy's `Provides`.
pub struct FromChain;

/// `S` is an item of `Self::Provides`. The shape is an existing index type.
trait Listed<S, Shape> {}

impl<B, S> Listed<S, ()> for B where B: BeanInfo<Provides = S> {}

impl<B, S> Listed<S, Ix0> for B where B: BeanInfo<Provides = (S,)> {}

impl<B, S, R> Listed<S, (Ix0, R)> for B where B: BeanInfo<Provides = (S, R)> {}

impl<B, S, L> Listed<S, (Ix1, L)> for B where B: BeanInfo<Provides = (L, S)> {}

impl<B, S, A, R> Listed<S, (Ix0, A, R)> for B where B: BeanInfo<Provides = (S, A, R)> {}

impl<B, S, L, R> Listed<S, (Ix1, L, R)> for B where B: BeanInfo<Provides = (L, S, R)> {}

impl<B, S, L, A> Listed<S, (Ix2, L, A)> for B where B: BeanInfo<Provides = (L, A, S)> {}

impl<B, S, A, C, R> Listed<S, (Ix0, A, C, R)> for B where B: BeanInfo<Provides = (S, A, C, R)> {}

impl<B, S, L, C, R> Listed<S, (Ix1, L, C, R)> for B where B: BeanInfo<Provides = (L, S, C, R)> {}

impl<B, S, L, A, R> Listed<S, (Ix2, L, A, R)> for B where B: BeanInfo<Provides = (L, A, S, R)> {}

impl<B, S, L, A, C> Listed<S, (Ix3, L, A, C)> for B where B: BeanInfo<Provides = (L, A, C, S)> {}

/// `At` visible on a proxy or on a proxy further out in its chain.
#[doc(hidden)]
pub trait Sees<S, K> {
    /// Index of `S` on the proxy that provides it.
    type Ix;
}

impl<P, S, Shape> Sees<S, (Own, Shape)> for P
where
    P: At<S> + Listed<S, Shape>,
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

/// Resource claim the pipeline bean can see on its own [`At`] list, or on a
/// proxy that lists the bean in `Provides`.
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

impl<B, C> Claim<B> for C
where
    C: Marked,
    B: At<C::Target>,
    C::Target: Target + Proxied,
{
    type Indexes = B::Ix;
}

impl<B, C> ClaimVia<B, Own> for C
where
    C: Marked,
    B: At<C::Target>,
    C::Target: Target + Proxied,
{
    type Indexes = B::Ix;
}

impl<B, P, C, Shape> ClaimVia<B, (FromProxy, Shape)> for C
where
    C: Marked,
    B: Proxied<Link: Marked<Target = P>>,
    P: ProxyInfo<Targets = B> + At<C::Target> + Listed<C::Target, Shape>,
    C::Target: Target + Proxied,
{
    type Indexes = P::Ix;
}

impl<B, P, O, K, C> ClaimVia<B, (FromChain, K)> for C
where
    C: Marked,
    B: Proxied<Link: Marked<Target = P>>,
    P: ProxyInfo<Targets = B, Chain = More<O>>,
    O: Sees<C::Target, K>,
    C::Target: Target + Proxied,
{
    type Indexes = <O as Sees<C::Target, K>>::Ix;
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

/// Body of `Via<P, _>`: a marker whose proxy is `P`, or a nested `Via`.
trait UnderProxy<P> {}

impl<P, C> UnderProxy<P> for C
where
    C: Marked,
    C::Target: Target + Proxied<Link: Marked<Target = P>>,
    P: ProxyInfo<Targets = C::Target>,
{
}

impl<P, Q, C> UnderProxy<P> for Via<Q, C> where C: UnderProxy<Q> {}

impl<B, P, C> Claim<B> for Via<P, C>
where
    C: UnderProxy<P>,
{
    type Indexes = ();
}

impl<A: ApplyProxy, B: ApplyProxy> ApplyProxy for And<A, B> {
    type Wrapped = And<A::Wrapped, B::Wrapped>;
}

impl<A: ApplyProxy, B: ApplyProxy> ApplyProxy for (A, B) {
    type Wrapped = (A::Wrapped, B::Wrapped);
}

impl<B, M, V: ?Sized> Claim<B> for Dyn<M, V>
where
    M: Marked,
    B: At<M::Target>,
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
