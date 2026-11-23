//! Target inject and init go through a proxy chain (`Via`, outer around inner).
//!
//! Manual impls. Derive later. `Get`, `Mut`, and `Marker` stay unchanged.
//! Examples live in the crate tests: `an_unset_proxy_uses_the_default`,
//! `injecting_the_target_wraps_the_explicit_proxy`,
//! `a_chain_nests_wrappers_outer_around_inner`.
//! Chain emits are `emitting_chain_level_*`, expanded with `impl_foreach`.
//! A component injects a proxy object only when that proxy lists it in `Provides`.
//! `BeanInfo::Proxy` is always `Proxy<Self>`. `Link` names the real proxy.

use core::any::TypeId;
use core::marker::PhantomData;

use crate::descriptor::{Get, Mut};

/// Injectable bean. Proxy types are not keys.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not an injectable bean",
    note = "proxy types do not implement `Target`. Name the bean; the proxy chain wraps it"
)]
pub trait Target: crate::BeanInfo {}

mod marked_seal {
    pub trait Seal {}
    impl<S> Seal for crate::Get<S> {}
    impl<S> Seal for crate::Mut<S> {}
}

/// Sealed `Get<S>` or `Mut<S>`. `Target` is `S`.
pub trait Marked: marked_seal::Seal {
    /// Inner type.
    type Target;
}

impl<S> Marked for Get<S> {
    type Target = S;
}

impl<S> Marked for Mut<S> {
    type Target = S;
}

/// `Get<P>` or `Mut<P>`: the `proxy = P::T` / `proxy = P::MUT` marker.
pub trait LinkMark<P>: Marked<Target = P> {}

impl<S, C> LinkMark<S> for C where C: Marked<Target = S> {}

/// No further proxy. [`Proxy`] ends here too.
pub struct Stop;

/// Next outer proxy.
pub struct More<P>(PhantomData<P>);

/// Data-only proxy meta.
/// `Targets` is the wrapped value, `Link` is `Get` or `Mut`, `Chain` is `Stop` or `More`.
pub trait ProxyInfo: crate::BeanInfo {
    /// Bean or proxy this proxy wraps.
    type Targets: crate::BeanInfo;
    /// `Get<Self>` or `Mut<Self>`.
    type Link: LinkMark<Self>;
    /// [`Stop`] when this proxy is outermost. [`More`] of the outer proxy otherwise.
    type Chain;
}

/// Bean wrapped by the proxy named in [`Link`](Proxied::Link).
///
/// [`BeanInfo::Proxy`](crate::BeanInfo::Proxy) is always [`Proxy<Self>`].
/// No explicit proxy uses `Link = Get<<Self as BeanInfo>::Proxy>`.
/// An explicit proxy is `Link = Get<Update>` (`T`) or `Mut<Update>` (`MUT`).
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a proxied target",
    note = "inject the target bean; a proxy is not a free-standing key. `BeanInfo::Proxy` is `Proxy<Self>`. `Link` is `Get<P>` or `Mut<P>` of the wrapping proxy"
)]
pub trait Proxied: crate::BeanInfo {
    /// `Get<P>` or `Mut<P>` of the proxy that wraps `Self`.
    /// The default `P` is [`<Self as BeanInfo>::Proxy`](crate::BeanInfo::Proxy).
    type Link: Marked<Target: ProxyInfo<Targets = Self>>;
}

impl<P, O> Proxied for P
where
    P: ProxyInfo<Chain = More<O>>,
    O: ProxyInfo<Targets = P>,
{
    type Link = O::Link;
}

/// Proxy used when a bean names none. `Targets = B`, `Link = Get<Self>`, `Chain = Stop`.
pub struct Proxy<B>(PhantomData<B>);

impl<B: crate::BeanInfo> crate::BeanInfo for Proxy<B> {
    /// This proxy provides no beans of its own.
    type Provides = ();
    /// This proxy injects nothing.
    type Injects = ();
    /// No provides and no injects.
    type Beans = ();
    /// The target keeps its own state.
    type State = ();
    /// The wrapper's own default proxy.
    type Proxy = Proxy<Proxy<B>>;
}

impl<B: crate::BeanInfo> ProxyInfo for Proxy<B> {
    type Targets = B;
    type Link = Get<Proxy<B>>;
    type Chain = Stop;
}

impl<B: crate::BeanInfo> crate::EventInfo for Proxy<B> {
    /// No parent events.
    type Parents = ();
}

/// Claim `C` wrapped by proxy `P`. A chain is `Via<Outer, Via<Inner, Get<T>>>`.
pub struct Via<P, C>(PhantomData<(P, C)>);

/// Init handler `H` through proxy `P`. A chain is `Fwd<Outer, Fwd<Inner, H>>`.
pub struct Fwd<P, H> {
    handler: H,
    _markers: PhantomData<P>,
}

impl<P, H> Fwd<P, H> {
    /// Wraps `handler` as an init that came through `P`.
    #[must_use]
    pub const fn new(handler: H) -> Self {
        Self { handler, _markers: PhantomData }
    }

    /// The function item the proxy stored.
    #[must_use]
    pub const fn handler(&self) -> &H {
        &self.handler
    }
}

/// This proxy initializes its direct target. `enter` runs outer-first, then `default_state`.
pub trait InitPath<B: crate::BeanInfo>: ProxyInfo<Targets = B> {
    /// Wraps `handler` in `Fwd`. Override to change the stored init.
    fn install<H>(handler: H) -> Fwd<Self, H> {
        Fwd::new(handler)
    }
    /// State of the target. Default is `Default::default()`.
    #[must_use]
    fn default_state() -> B::State
    where
        B::State: Default,
    {
        Default::default()
    }
    /// Runs before the next inner init step.
    fn enter() {}
}

impl<B: crate::BeanInfo> InitPath<B> for Proxy<B> {}

/// Projects a call-site claim onto its proxy wrapper.
pub trait ApplyProxy {
    /// [`Via`] around the target, nested through the chain.
    type Wrapped;
}

/// Nests `C` through proxy `Self` according to `Link` ([`Stop`] or [`More`]).
#[doc(hidden)]
pub trait Enfold<C, Link> {
    /// Fully wrapped claim.
    type Wrapped;
}

impl<P, C> Enfold<C, Stop> for P {
    type Wrapped = Via<P, C>;
}

impl<P, O, C> Enfold<C, More<O>> for P
where
    O: ProxyInfo + Enfold<Via<P, C>, O::Chain>,
{
    type Wrapped = <O as Enfold<Via<P, C>, O::Chain>>::Wrapped;
}

impl<C, P> ApplyProxy for C
where
    C: Marked,
    C::Target: Proxied<Link: Marked<Target = P>>,
    P: ProxyInfo<Targets = C::Target> + Enfold<C, P::Chain>,
{
    type Wrapped = <P as Enfold<C, P::Chain>>::Wrapped;
}

/// Builds the init wrapper for proxy `Self` around bean `B`, outer around inner.
#[doc(hidden)]
pub trait ForwardInit<B, H, Link> {
    /// Nested [`Fwd`] for the whole chain above `Self`, including `Self`.
    type Out;
    /// Wraps `handler` through this proxy and its outers.
    fn forward(handler: H) -> Self::Out;
}

/// Nests an already-built inner [`Fwd`] through an outer proxy.
#[doc(hidden)]
pub trait WrapFwd<H, Link> {
    /// Nested [`Fwd`].
    type Out;
    /// Wraps `handler` in this proxy, then in its outers.
    fn wrap_fwd(handler: H) -> Self::Out;
}

impl<P, B, H> ForwardInit<B, H, Stop> for P
where
    B: crate::BeanInfo,
    P: InitPath<B>,
{
    type Out = Fwd<P, H>;

    fn forward(handler: H) -> Self::Out {
        <P as InitPath<B>>::install(handler)
    }
}

impl<P, B, O, H> ForwardInit<B, H, More<O>> for P
where
    B: crate::BeanInfo,
    P: InitPath<B>,
    O: ProxyInfo + WrapFwd<Fwd<P, H>, O::Chain>,
{
    type Out = <O as WrapFwd<Fwd<P, H>, O::Chain>>::Out;

    fn forward(handler: H) -> Self::Out {
        O::wrap_fwd(<P as InitPath<B>>::install(handler))
    }
}

impl<P, H> WrapFwd<H, Stop> for P {
    type Out = Fwd<P, H>;

    fn wrap_fwd(handler: H) -> Self::Out {
        Fwd::new(handler)
    }
}

impl<P, O, H> WrapFwd<H, More<O>> for P
where
    O: ProxyInfo + WrapFwd<Fwd<P, H>, O::Chain>,
{
    type Out = <O as WrapFwd<Fwd<P, H>, O::Chain>>::Out;

    fn wrap_fwd(handler: H) -> Self::Out {
        O::wrap_fwd(Fwd::<P, H>::new(handler))
    }
}

/// `enter` from the outermost proxy in to `Self`. `B` is `Self`'s direct target.
#[doc(hidden)]
pub trait EnterAll<B, Link> {
    /// Enters every proxy from the outside in.
    fn enter_all();
}

impl<P, B> EnterAll<B, Stop> for P
where
    B: crate::BeanInfo,
    P: InitPath<B>,
{
    fn enter_all() {
        <P as InitPath<B>>::enter();
    }
}

impl<P, B, O> EnterAll<B, More<O>> for P
where
    B: crate::BeanInfo,
    P: crate::BeanInfo + InitPath<B> + Proxied<Link: Marked<Target = O>>,
    O: ProxyInfo<Targets = P> + InitPath<P> + EnterAll<P, O::Chain>,
{
    fn enter_all() {
        <O as EnterAll<P, O::Chain>>::enter_all();
        <P as InitPath<B>>::enter();
    }
}

/// Init of bean `B` through its whole proxy chain.
#[doc(hidden)]
pub trait DriveInit<B: crate::BeanInfo> {
    /// Enters outer proxies first, then returns `B`'s state from its direct proxy.
    fn drive() -> B::State
    where
        B::State: Default;
}

impl<B, P> DriveInit<B> for P
where
    B: crate::BeanInfo,
    P: InitPath<B> + EnterAll<B, P::Chain>,
{
    fn drive() -> B::State
    where
        B::State: Default,
    {
        <P as EnterAll<B, P::Chain>>::enter_all();
        <P as InitPath<B>>::default_state()
    }
}

/// Inject key. Proxy types are not keys.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be injected alone",
    note = "a proxy is not a free-standing inject key; name the target bean and the proxy chain wraps it"
)]
pub trait FreeKey {}

impl<C> FreeKey for C
where
    C: Marked,
    C::Target: Target + Proxied,
{
}

/// Parent events of a proxy. Its own id is matched separately.
pub trait ParentList {
    /// `true` when `emitted` contains one of these parent events.
    fn listed(emitted: &[TypeId]) -> bool;
}
