//! Which beans a resource claim names, and the view the init function sees.

use core::marker::PhantomData;

use crate::bean::{ApplyProxy, BeanInfo, Marked, Proxied, Stop, Target};
use crate::borrow::{At, Ix0, Ix1, Ix2, Ix3};
use crate::context::{Leaf, List};
use crate::descriptor::LevelDesc;
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

/// Proxy list `P` and module list `M`. [`At`] forwards to one inner.
///
/// `W` is that inner's path. Callers do not name it.
pub struct Surface<P, M, W>(PhantomData<(P, M, W)>);

/// This proxy plus the outer tuple on [`Chain`](crate::BeanInfo::Chain), as one leaf.
trait ProxyRows<L> {
    /// Direct proxy first, then each outer.
    type Rows;
}

impl<P> ProxyRows<Stop> for P {
    type Rows = List<Leaf<(P,)>>;
}

impl<P, O0> ProxyRows<(O0,)> for P {
    type Rows = List<Leaf<(P, O0)>>;
}

impl<P, O0, O1> ProxyRows<(O0, O1)> for P {
    type Rows = List<Leaf<(P, O0, O1)>>;
}

impl<P, O0, O1, O2> ProxyRows<(O0, O1, O2)> for P {
    type Rows = List<Leaf<(P, O0, O1, O2)>>;
}

/// Pipeline bean: its proxy chain and itself as the module list.
pub(crate) trait Host: BeanInfo {
    /// Proxies from the direct one out to the end of [`Chain`](crate::BeanInfo::Chain).
    type Proxies;
    /// The pipeline bean. A longer module list uses the same [`List`] shape.
    type Modules;
}

impl<B, P, U> Host for B
where
    B: Proxied<Link: Marked<Target = P>>,
    P: BeanInfo + ProxyRows<P::Chain, Rows = U>,
{
    type Proxies = U;
    type Modules = List<Leaf<(B,)>>;
}

/// `At` on this element.
#[doc(hidden)]
pub struct Hit;

/// `At` is further along the list.
#[doc(hidden)]
pub struct Skip<K>(PhantomData<K>);

/// Proxy forward: the element must also be in `Provides`.
#[doc(hidden)]
pub struct InProvides;

/// Module forward: the element's own [`At`] is enough.
#[doc(hidden)]
pub struct InAt;

/// Forward `S` to an inner [`At`].
#[doc(hidden)]
pub trait Find<S, K, D> {
    /// That inner index.
    type Ix;
}

impl<S, E, G> Find<S, (Hit, G), InProvides> for List<Leaf<(E,)>>
where
    E: At<S> + Listed<S, G>,
{
    type Ix = <E as At<S>>::Ix;
}

impl<S, E> Find<S, Hit, InAt> for List<Leaf<(E,)>>
where
    E: At<S>,
{
    type Ix = <E as At<S>>::Ix;
}

impl<S, E0, E1, G> Find<S, (Hit, G), InProvides> for List<Leaf<(E0, E1)>>
where
    E0: At<S> + Listed<S, G>,
{
    type Ix = <E0 as At<S>>::Ix;
}

impl<S, E0, E1> Find<S, Hit, InAt> for List<Leaf<(E0, E1)>>
where
    E0: At<S>,
{
    type Ix = <E0 as At<S>>::Ix;
}

impl<S, E0, E1, D, K> Find<S, Skip<K>, D> for List<Leaf<(E0, E1)>>
where
    List<Leaf<(E1,)>>: Find<S, K, D>,
{
    type Ix = <List<Leaf<(E1,)>> as Find<S, K, D>>::Ix;
}

impl<S, E0, E1, E2, G> Find<S, (Hit, G), InProvides> for List<Leaf<(E0, E1, E2)>>
where
    E0: At<S> + Listed<S, G>,
{
    type Ix = <E0 as At<S>>::Ix;
}

impl<S, E0, E1, E2> Find<S, Hit, InAt> for List<Leaf<(E0, E1, E2)>>
where
    E0: At<S>,
{
    type Ix = <E0 as At<S>>::Ix;
}

impl<S, E0, E1, E2, D, K> Find<S, Skip<K>, D> for List<Leaf<(E0, E1, E2)>>
where
    List<Leaf<(E1, E2)>>: Find<S, K, D>,
{
    type Ix = <List<Leaf<(E1, E2)>> as Find<S, K, D>>::Ix;
}

impl<S, E0, E1, E2, E3, G> Find<S, (Hit, G), InProvides> for List<Leaf<(E0, E1, E2, E3)>>
where
    E0: At<S> + Listed<S, G>,
{
    type Ix = <E0 as At<S>>::Ix;
}

impl<S, E0, E1, E2, E3> Find<S, Hit, InAt> for List<Leaf<(E0, E1, E2, E3)>>
where
    E0: At<S>,
{
    type Ix = <E0 as At<S>>::Ix;
}

impl<S, E0, E1, E2, E3, D, K> Find<S, Skip<K>, D> for List<Leaf<(E0, E1, E2, E3)>>
where
    List<Leaf<(E1, E2, E3)>>: Find<S, K, D>,
{
    type Ix = <List<Leaf<(E1, E2, E3)>> as Find<S, K, D>>::Ix;
}

/// `W` is `(OnProxy, _)` or `(OnModule, _)`.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "`{S}` is not on the proxy list or the module list",
    note = "a proxy object must be an item of that proxy's Provides; a module object needs At"
)]
pub trait Pick<S, W> {
    /// Index on the chosen inner.
    type Ix;
}

/// Forward through the proxy chain.
#[doc(hidden)]
pub struct OnProxy;

/// Forward through the module list.
#[doc(hidden)]
pub struct OnModule;

impl<S, P, M, K> Pick<S, (OnProxy, K)> for (P, M)
where
    P: Find<S, K, InProvides>,
{
    type Ix = <P as Find<S, K, InProvides>>::Ix;
}

impl<S, P, M, K> Pick<S, (OnModule, K)> for (P, M)
where
    M: Find<S, K, InAt>,
{
    type Ix = <M as Find<S, K, InAt>>::Ix;
}

impl<S, P, M, W> At<S> for Surface<P, M, W>
where
    (P, M): Pick<S, W>,
{
    type Ix = <(P, M) as Pick<S, W>>::Ix;
}

/// Resource claim the pipeline bean can see on its own [`At`] list, or on a
/// proxy that lists the bean in `Provides`.
pub trait ClaimVia<B, K> {
    /// Absolute indexes of the claimed beans.
    type Indexes;
}

/// Resource claim `B` can project.
///
/// [`ClaimVia`] leaves carry [`Surface`]'s [`At`]. Lists only require each part to be a [`Claim`].
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

impl<B, P, M, C, W> ClaimVia<B, Surface<P, M, W>> for C
where
    C: Marked,
    B: Host<Proxies = P, Modules = M>,
    Surface<P, M, W>: At<C::Target>,
    C::Target: Target + Proxied,
{
    type Indexes = <Surface<P, M, W> as At<C::Target>>::Ix;
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

impl<B, P, C> Claim<B> for List<Leaf<(P, C)>>
where
    C: Marked,
    C::Target: LevelDesc<Rows = List<Leaf<(P,)>>>,
{
    type Indexes = ();
}

impl<B, P, O, C> Claim<B> for List<Leaf<(P, O, C)>>
where
    C: Marked,
    C::Target: LevelDesc<Rows = List<Leaf<(P, O)>>>,
{
    type Indexes = ();
}

impl<B, P, O0, O1, C> Claim<B> for List<Leaf<(P, O0, O1, C)>>
where
    C: Marked,
    C::Target: LevelDesc<Rows = List<Leaf<(P, O0, O1)>>>,
{
    type Indexes = ();
}

impl<B, P, O0, O1, O2, C> Claim<B> for List<Leaf<(P, O0, O1, O2, C)>>
where
    C: Marked,
    C::Target: LevelDesc<Rows = List<Leaf<(P, O0, O1, O2)>>>,
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

impl<P, C> ArgsView for List<Leaf<(P, C)>>
where
    C: ArgsView,
{
    type View<'a>
        = C::View<'a>
    where
        Self: 'a;
}

impl<P, O, C> ArgsView for List<Leaf<(P, O, C)>>
where
    C: ArgsView,
{
    type View<'a>
        = C::View<'a>
    where
        Self: 'a;
}

impl<P, O0, O1, C> ArgsView for List<Leaf<(P, O0, O1, C)>>
where
    C: ArgsView,
{
    type View<'a>
        = C::View<'a>
    where
        Self: 'a;
}

impl<P, O0, O1, O2, C> ArgsView for List<Leaf<(P, O0, O1, O2, C)>>
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
