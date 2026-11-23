//! Proxy beans: a target is injected and initialized through a proxy.
//!
//! Manual impls match the derive surface ([`BeanInfo`] + [`ProxyInfo`] + [`At`]
//! for provides). Derive is deferred (rule 6). [`Get`] / [`Mut`] / [`Marker`]
//! stay as they are (rule 19). The wrapper is the side type [`Via`], like [`Dyn`].
//!
//! [`At`]: crate::At
//! [`Dyn`]: crate::Dyn
//! [`Get`]: crate::Get
//! [`Marker`]: crate::Marker
//! [`Mut`]: crate::Mut
//!
//! ```compile_fail
//! use ph_beans::{At, BeanInfo, Ix0, Marker, ProxyInfo, Query, Target};
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
//! use ph_beans::{BeanInfo, FromParent, Get, OfProvide, ProxyInfo, Target};
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
//! use ph_beans::{At, BeanInfo, Claim, Ix0, ProxyInfo, Target};
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
//! use ph_beans::{BeanInfo, Get, Proxied, ProxyInfo, Target};
//!
//! struct Tick;
//! struct Update;
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
//! }
//! impl Proxied for Update {
//!     type Proxy = Update;
//!     type Link = Get<Update>;
//! }
//! ```
//!
//! ```compile_fail
//! use ph_beans::{BeanInfo, Pass, ProxyInfo};
//!
//! struct Update;
//!
//! impl BeanInfo for Update {
//!     type Provides = ();
//!     type Injects = ();
//!     type Beans = ();
//!     type State = ();
//! }
//!
//! fn default_proxy<T: ProxyInfo>() {}
//! default_proxy::<Pass<Update>>();
//! ```

use core::any::TypeId;
use core::marker::PhantomData;

use crate::descriptor::{Get, Mut};

/// A bean or event that may sit under a proxy.
///
/// Proxy types do **not** implement this. [`Proxied`] requires it, so a proxy
/// cannot declare `proxy = ...` and does not receive the default [`Pass`] proxy.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot declare `proxy = ...`",
    note = "only a non-proxy bean or event implements `Target`. A `#[config(type = proxy)]` type has no proxy of its own"
)]
pub trait Target: crate::BeanInfo {}

/// `Get<P>` or `Mut<P>`: the `proxy = P::T` / `proxy = P::MUT` marker.
pub trait LinkMark<P> {}

impl<P> LinkMark<P> for Get<P> {}

impl<P> LinkMark<P> for Mut<P> {}

/// Meta for `#[config(type = proxy)]`. Data only (rule 14).
///
/// Associated item names are a proposal pending owner approval. Do not add
/// methods or further items without that OK.
///
/// [`Targets`] is the wrapped bean. [`Link`] is `Get<Self>` (`T`) or
/// `Mut<Self>` (`MUT`).
///
/// [`Link`]: ProxyInfo::Link
/// [`Targets`]: ProxyInfo::Targets
pub trait ProxyInfo: crate::BeanInfo {
    /// Bean this proxy wraps. Not itself a proxy (`Targets: Target`).
    type Targets: Target;
    /// `Get<Self>` or `Mut<Self>`.
    type Link: LinkMark<Self>;
}

/// Manual stand-in for `proxy = P::T` / `proxy = P::MUT` on a target.
///
/// Data only, same bar as [`ProxyInfo`] (rule 14). Names are a proposal
/// pending owner approval.
///
/// No `proxy = ...` in source is the manual impl whose [`Proxy`] is [`Pass`]
/// and whose [`Link`] is `Get<Pass<Self>>`. A proxy does not implement this
/// (it is not a [`Target`]), so the default does not apply to proxies and a
/// proxy cannot declare another proxy.
///
/// [`Link`]: Proxied::Link
/// [`Proxy`]: Proxied::Proxy
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a proxied target",
    note = "inject the target bean; a proxy is not a free-standing key. A missing `proxy = ...` is `Proxy = Pass<Self>` and `Link = Get<Pass<Self>>`"
)]
pub trait Proxied: Target {
    /// Proxy that wraps `Self`.
    type Proxy: ProxyInfo<Targets = Self>;
    /// `Get<Self::Proxy>` (`T`) or `Mut<Self::Proxy>` (`MUT`).
    type Link: LinkMark<Self::Proxy>;
}

/// Identity proxy. A bean with no `proxy = ...` names this as [`Proxied::Proxy`].
///
/// Pass-through: no provides, no events, and it is not itself proxied.
/// The name and the `Pass<B>` shape are pending owner approval.
pub struct Pass<B>(PhantomData<B>);

impl<B: Target> crate::BeanInfo for Pass<B> {
    /// Pass-through proxies expose no beans of their own.
    type Provides = ();
    /// Pass-through proxies inject nothing.
    type Injects = ();
    /// No provides and no injects.
    type Beans = ();
    /// The target keeps its own state. Pass does not store a second copy.
    type State = ();
}

impl<B: Target> ProxyInfo for Pass<B> {
    type Targets = B;
    type Link = Get<Pass<B>>;
}

/// Claim `C` wrapped by proxy `P`.
///
/// Side type, like [`Dyn`](crate::Dyn). Not a new parameter on [`Get`] / [`Mut`].
/// Call sites still pass `Get` / `Mut` for the **target**; [`ApplyProxy`] projects
/// this wrapper. [`ArgsView`](crate::ArgsView) for the wrapper is the target
/// reference — the proxy forwards that argument.
pub struct Via<P, C>(PhantomData<(P, C)>);

/// Init handler `H` installed by proxy `P`.
///
/// The proxy is the path that stores the target's init. `H` stays a function
/// item (rule 18).
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

/// Proxy `Self` initializes bean `B`.
///
/// Not a meta trait. [`Update`](crate::Update) stays on [`Built`](crate::Built)
/// only (rule 17). This path is the init stage ([`Init`](crate::Init) /
/// [`InitStateStep`](crate::InitStateStep)).
pub trait InitPath<B: crate::BeanInfo>: ProxyInfo<Targets = B> {
    /// Stores `handler` as the target's init.
    fn install<H>(handler: H) -> Fwd<Self, H>;
    /// Default state of `B`, produced by the proxy.
    fn default_state() -> B::State
    where
        B::State: Default;
}

impl<B> InitPath<B> for Pass<B>
where
    B: Proxied<Proxy = Pass<B>>,
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
    /// [`Via`] around the target, or the same shape for a list of claims.
    type Wrapped;
}

impl<B: Proxied> ApplyProxy for Get<B> {
    type Wrapped = Via<B::Proxy, Get<B>>;
}

impl<B: Proxied> ApplyProxy for Mut<B> {
    type Wrapped = Via<B::Proxy, Mut<B>>;
}

/// A free-standing inject key. Proxy beans do not implement [`Proxied`], so
/// `Get<Proxy>` / `Mut<Proxy>` are not keys.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be injected alone",
    note = "a proxy is not a free-standing inject key; name the target bean and the proxy wraps it"
)]
pub trait FreeKey {}

impl<B: Proxied> FreeKey for Get<B> {}

impl<B: Proxied> FreeKey for Mut<B> {}

/// Events of a proxy, walked when the target is scheduled.
///
/// [`Pass`] contributes nothing. A proxy that is an event contributes its
/// [`crate::EventInfo::Parents`] and its own id is matched separately.
pub trait ParentList {
    /// `true` when `emitted` contains one of these parent events.
    fn listed(emitted: &[TypeId]) -> bool;
}

impl<B: Target> ParentList for Pass<B> {
    fn listed(_emitted: &[TypeId]) -> bool {
        false
    }
}
