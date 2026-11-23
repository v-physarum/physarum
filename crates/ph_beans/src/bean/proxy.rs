//! Proxy beans: a target is injected and initialized through a proxy chain.
//!
//! Manual impls match the derive surface ([`BeanInfo`] + [`ProxyInfo`] + [`At`]
//! for provides). Derive is deferred (rule 6). [`Get`] / [`Mut`] / [`Marker`]
//! stay as they are (rule 19). The wrapper is the side type [`Via`], like [`Dyn`].
//! A chain nests that wrapper, outer around inner.
//!
//! [`At`]: crate::At
//! [`Dyn`]: crate::Dyn
//! [`Get`]: crate::Get
//! [`Marker`]: crate::Marker
//! [`Mut`]: crate::Mut
//!
//! ```compile_fail
//! use ph_beans::{At, BeanInfo, Ix0, Marker, ProxyInfo, Query, Stop, Target};
//!
//! struct Tick;
//! struct Update;
//! struct App;
//!
//! impl BeanInfo for Tick {
//!     type Provides = ();
//!     type Injects = ();
//!     type Beans = ();
//!     type State = ();
//! }
//! impl Target for Tick {}
//! impl BeanInfo for Update {
//!     type Provides = ();
//!     type Injects = ();
//!     type Beans = ();
//!     type State = ();
//! }
//! impl ProxyInfo for Update {
//!     type Targets = Tick;
//!     type Link = ph_beans::Get<Update>;
//!     type Chain = Stop;
//! }
//! impl BeanInfo for App {
//!     type Provides = ();
//!     type Injects = ();
//!     type Beans = ();
//!     type State = ();
//! }
//! impl At<Update> for App {
//!     type Ix = Ix0;
//! }
//!
//! let _claim = App::query().inject_resources(Update::MUT);
//! ```
//!
//! ```compile_fail
//! use ph_beans::{BeanInfo, FromParent, Get, OfProvide, ProxyInfo, Stop, Target};
//!
//! struct Tick;
//! struct Update;
//! struct App;
//! struct Child;
//!
//! impl BeanInfo for Tick {
//!     type Provides = ();
//!     type Injects = ();
//!     type Beans = ();
//!     type State = ();
//! }
//! impl Target for Tick {}
//! impl BeanInfo for Update {
//!     type Provides = ();
//!     type Injects = ();
//!     type Beans = ();
//!     type State = ();
//! }
//! impl ProxyInfo for Update {
//!     type Targets = Tick;
//!     type Link = Get<Update>;
//!     type Chain = Stop;
//! }
//! impl BeanInfo for App {
//!     type Provides = Get<Update>;
//!     type Injects = ();
//!     type Beans = Get<Update>;
//!     type State = ();
//! }
//! impl BeanInfo for Child {
//!     type Provides = ();
//!     type Injects = Get<Update>;
//!     type Beans = Get<Update>;
//!     type State = ();
//! }
//!
//! fn inject<C: FromParent<App, OfProvide>>() {}
//! inject::<Get<Update>>();
//! ```
//!
//! ```compile_fail
//! use ph_beans::{At, BeanInfo, Claim, Ix0, ProxyInfo, Stop, Target};
//!
//! struct Tick;
//! struct Update;
//! struct App;
//!
//! impl BeanInfo for Tick {
//!     type Provides = ();
//!     type Injects = ();
//!     type Beans = ();
//!     type State = ();
//! }
//! impl Target for Tick {}
//! impl BeanInfo for Update {
//!     type Provides = ();
//!     type Injects = ();
//!     type Beans = ();
//!     type State = ();
//! }
//! impl ProxyInfo for Update {
//!     type Targets = Tick;
//!     type Link = ph_beans::Get<Update>;
//!     type Chain = Stop;
//! }
//! impl BeanInfo for App {
//!     type Provides = ();
//!     type Injects = ();
//!     type Beans = ();
//!     type State = ();
//! }
//! impl At<Update> for App {
//!     type Ix = Ix0;
//! }
//!
//! fn claim<C: Claim<App>>() {}
//! claim::<ph_beans::Mut<Update>>();
//! ```
//!
//! ```compile_fail
//! use ph_beans::{At, BeanInfo, Get, Ix0, Marker, More, Proxied, ProxyInfo, Query, Stop};
//!
//! struct Tick;
//! struct Inner;
//! struct Outer;
//! struct App;
//!
//! impl BeanInfo for Tick {
//!     type Provides = ();
//!     type Injects = ();
//!     type Beans = ();
//!     type State = ();
//! }
//! impl ph_beans::Target for Tick {}
//! impl BeanInfo for Inner {
//!     type Provides = ();
//!     type Injects = ();
//!     type Beans = ();
//!     type State = ();
//! }
//! impl ProxyInfo for Inner {
//!     type Targets = Tick;
//!     type Link = Get<Inner>;
//!     type Chain = More<Outer>;
//! }
//! impl Proxied for Inner {
//!     type Proxy = Outer;
//!     type Link = Get<Outer>;
//! }
//! impl BeanInfo for Outer {
//!     type Provides = ();
//!     type Injects = ();
//!     type Beans = ();
//!     type State = ();
//! }
//! impl ProxyInfo for Outer {
//!     type Targets = Inner;
//!     type Link = Get<Outer>;
//!     type Chain = Stop;
//! }
//! impl Proxied for Tick {
//!     type Proxy = Inner;
//!     type Link = Get<Inner>;
//! }
//! impl BeanInfo for App {
//!     type Provides = ();
//!     type Injects = ();
//!     type Beans = ();
//!     type State = ();
//! }
//! impl At<Inner> for App {
//!     type Ix = Ix0;
//! }
//!
//! let _claim = App::query().inject_resources(Inner::T);
//! ```

use core::any::TypeId;
use core::marker::PhantomData;

use crate::descriptor::{Get, Mut};

/// A bean or event. Proxy types do not implement this, so a proxy is not a
/// free-standing inject key even when an outer proxy wraps it.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not an injectable bean",
    note = "proxy types do not implement `Target`. Name the bean; the proxy chain wraps it"
)]
pub trait Target: crate::BeanInfo {}

/// `Get<P>` or `Mut<P>`: the `proxy = P::T` / `proxy = P::MUT` marker.
pub trait LinkMark<P> {}

impl<P> LinkMark<P> for Get<P> {}

impl<P> LinkMark<P> for Mut<P> {}

/// Outermost link of a proxy chain. This proxy has no further proxy.
///
/// Name pending owner approval. [`Proxy`] uses this too: it is not stacked
/// outside an explicit proxy.
pub struct Stop;

/// This proxy is wrapped by `P`.
///
/// Name pending owner approval. `P` is the next outer proxy.
pub struct More<P>(PhantomData<P>);

/// Meta for `#[config(type = proxy)]`. Data only (rule 14).
///
/// Associated item names are a proposal pending owner approval. Do not add
/// methods or further items without that OK.
///
/// [`Targets`] is the wrapped bean or the next inner proxy. [`Link`] is
/// `Get<Self>` (`T`) or `Mut<Self>` (`MUT`). [`Chain`] is [`Stop`] or
/// [`More`] of the next outer proxy.
///
/// [`Chain`]: ProxyInfo::Chain
/// [`Link`]: ProxyInfo::Link
/// [`Targets`]: ProxyInfo::Targets
pub trait ProxyInfo: crate::BeanInfo {
    /// Bean or proxy this proxy wraps.
    type Targets: crate::BeanInfo;
    /// `Get<Self>` or `Mut<Self>`.
    type Link: LinkMark<Self>;
    /// [`Stop`] when this proxy is outermost. [`More`] of the outer proxy otherwise.
    type Chain;
}

/// Manual stand-in for `proxy = P::T` / `proxy = P::MUT`.
///
/// Data only, same bar as [`ProxyInfo`] (rule 14). Names are a proposal
/// pending owner approval. A bean or a proxy may implement this, so chains
/// type-check. Injecting a proxy still fails: proxy types are not [`Target`].
///
/// No `proxy = ...` on a bean is the manual impl `Proxy = Proxy<Self>` and
/// `Link = Get<Proxy<Self>>`. The identity proxy is not [`Proxied`], so that
/// chain stops. An explicit proxy that names no further proxy is not
/// [`Proxied`] either: the identity proxy is not wrapped around it.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a proxied target",
    note = "inject the target bean; a proxy is not a free-standing key. A missing `proxy = ...` on a bean is `Proxy = Proxy<Self>` and `Link = Get<Proxy<Self>>`"
)]
pub trait Proxied: crate::BeanInfo {
    /// Proxy that wraps `Self`.
    type Proxy: ProxyInfo<Targets = Self>;
    /// `Get<Self::Proxy>` (`T`) or `Mut<Self::Proxy>` (`MUT`).
    type Link: LinkMark<Self::Proxy>;
}

/// Identity proxy. A bean with no `proxy = ...` names this as [`Proxied::Proxy`].
///
/// No provides, no events, and it is not itself proxied, so a chain that
/// reaches [`Proxy`] stops. It is only for a [`Target`]. It does not wrap a
/// proxy and it is not placed outside an explicit proxy.
pub struct Proxy<B>(PhantomData<B>);

impl<B: Target> crate::BeanInfo for Proxy<B> {
    /// Pass-through proxies expose no beans of their own.
    type Provides = ();
    /// Pass-through proxies inject nothing.
    type Injects = ();
    /// No provides and no injects.
    type Beans = ();
    /// The target keeps its own state. Proxy does not store a second copy.
    type State = ();
}

impl<B: Target> ProxyInfo for Proxy<B> {
    type Targets = B;
    type Link = Get<Proxy<B>>;
    type Chain = Stop;
}

/// Claim `C` wrapped by proxy `P`.
///
/// Side type, like [`Dyn`](crate::Dyn). Not a new parameter on [`Get`] / [`Mut`].
/// Call sites still pass `Get` / `Mut` for the **target**; [`ApplyProxy`] projects
/// this wrapper. A chain is `Via<Outer, Via<Inner, Get<T>>>`.
/// [`ArgsView`](crate::ArgsView) for the wrapper is the target reference — each
/// proxy forwards that argument.
pub struct Via<P, C>(PhantomData<(P, C)>);

/// Init handler `H` installed by proxy `P`.
///
/// The proxy is the path that stores the target's init. A chain nests this
/// outer to inner: `Fwd<Outer, Fwd<Inner, H>>`. `H` stays a function item
/// (rule 18).
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

/// Proxy `Self` initializes its direct target `B`.
///
/// Not a meta trait. [`Update`](crate::Update) stays on [`Built`](crate::Built)
/// only (rule 17). The chain walks [`ProxyInfo::Chain`] and calls [`enter`]
/// from the outside in, then [`default_state`] on the proxy that targets the bean.
///
/// [`default_state`]: InitPath::default_state
/// [`enter`]: InitPath::enter
pub trait InitPath<B: crate::BeanInfo>: ProxyInfo<Targets = B> {
    /// Stores `handler` as this proxy's own init layer.
    fn install<H>(handler: H) -> Fwd<Self, H>;
    /// Default state of `B`, produced by this proxy.
    fn default_state() -> B::State
    where
        B::State: Default;
    /// Runs before the next inner init step.
    fn enter() {}
}

impl<B> InitPath<B> for Proxy<B>
where
    B: Target + Proxied<Proxy = Proxy<B>>,
{
    fn install<H>(handler: H) -> Fwd<Self, H> {
        Fwd::new(handler)
    }

    fn default_state() -> B::State
    where
        B::State: Default,
    {
        B::State::default()
    }
}

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

impl<B: Proxied> ApplyProxy for Get<B>
where
    B::Proxy: Enfold<Get<B>, <B::Proxy as ProxyInfo>::Chain>,
{
    type Wrapped = <B::Proxy as Enfold<Get<B>, <B::Proxy as ProxyInfo>::Chain>>::Wrapped;
}

impl<B: Proxied> ApplyProxy for Mut<B>
where
    B::Proxy: Enfold<Mut<B>, <B::Proxy as ProxyInfo>::Chain>,
{
    type Wrapped = <B::Proxy as Enfold<Mut<B>, <B::Proxy as ProxyInfo>::Chain>>::Wrapped;
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

/// Calls [`InitPath::enter`] from the outermost proxy in to `Self`.
///
/// `B` is the direct target of `Self`, not a projection of [`ProxyInfo::Targets`].
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
    P: crate::BeanInfo + InitPath<B> + Proxied<Proxy = O>,
    O: InitPath<P> + EnterAll<P, O::Chain>,
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

/// A free-standing inject key. Proxy beans do not implement [`Target`], so
/// `Get<Proxy>` / `Mut<Proxy>` are not keys, including a proxy that itself
/// has an outer proxy.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be injected alone",
    note = "a proxy is not a free-standing inject key; name the target bean and the proxy chain wraps it"
)]
pub trait FreeKey {}

impl<B: Target + Proxied> FreeKey for Get<B> {}

impl<B: Target + Proxied> FreeKey for Mut<B> {}

/// Events of a proxy, walked when the target is scheduled.
///
/// [`Proxy`] contributes nothing. A proxy that is an event contributes its
/// [`crate::EventInfo::Parents`] and its own id is matched separately.
pub trait ParentList {
    /// `true` when `emitted` contains one of these parent events.
    fn listed(emitted: &[TypeId]) -> bool;
}

impl<B: Target> ParentList for Proxy<B> {
    fn listed(_emitted: &[TypeId]) -> bool {
        false
    }
}
