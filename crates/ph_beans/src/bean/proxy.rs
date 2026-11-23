//! Target inject and init go through a proxy chain (`Via`, outer around inner).
//!
//! Manual impls. Derive later. `Get`, `Mut`, and `Marker` stay unchanged.
//! Examples live in the crate tests: `an_unset_proxy_uses_the_default`,
//! `injecting_the_target_wraps_the_explicit_proxy`,
//! `a_chain_nests_wrappers_outer_around_inner`.
//! Chain emits are `emitting_chain_level_*`, expanded with `impl_foreach`.
//! A component injects a proxy object only when that proxy lists it in `Provides`.
//! `BeanInfo::Proxy` is that proxy. Its `Link` is `Get<Self>` or `Mut<Self>`.

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

/// No further proxy. [`Proxy`] ends here too.
///
/// A longer chain is a tuple of the outer proxies, nearest first: `(Mid, Outer)`.
pub struct Stop;

/// Bean wrapped by [`BeanInfo::Proxy`](crate::BeanInfo::Proxy).
///
/// Blanket for every [`BeanInfo`](crate::BeanInfo).
/// [`Link`](Proxied::Link) is that proxy's [`BeanInfo::Link`](crate::BeanInfo::Link).
/// No named proxy: `Proxy<Self>`, so the link is `Get<Proxy<Self>>`.
/// A named proxy is `BeanInfo::Proxy` (`Get<Update>` or `Mut<Gate>`).
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a proxied target",
    note = "inject the target bean; a proxy is not a free-standing key. `Link` is that proxy's `Get<Self>` or `Mut<Self>`"
)]
pub trait Proxied: crate::BeanInfo {
    /// `Get<P>` or `Mut<P>` of the wrapping proxy.
    type Link: Marked<Target = Self::Proxy>;
}

impl<T: crate::BeanInfo> Proxied for T {
    type Link = <T::Proxy as crate::BeanInfo>::Link;
}

/// Proxy used when a bean names none. `Link = Get<Self>`, `Chain = Stop`.
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
    /// Shared claim of this wrapper.
    type Link = Get<Proxy<B>>;
    /// Nothing wraps the default proxy.
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
pub trait InitPath<B: crate::BeanInfo>: crate::BeanInfo {
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

/// Nests `C` through proxy `Self`. `Link` is [`Stop`] or a tuple of outers.
#[doc(hidden)]
pub trait Enfold<C, Link> {
    /// Fully wrapped claim.
    type Wrapped;
}

impl<P, C> Enfold<C, Stop> for P {
    type Wrapped = Via<P, C>;
}

impl<C, P> ApplyProxy for C
where
    C: Marked,
    C::Target: Proxied<Link: Marked<Target = P>>,
    P: crate::BeanInfo + Enfold<C, P::Chain>,
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

impl<P, H> WrapFwd<H, Stop> for P {
    type Out = Fwd<P, H>;

    fn wrap_fwd(handler: H) -> Self::Out {
        Fwd::new(handler)
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
