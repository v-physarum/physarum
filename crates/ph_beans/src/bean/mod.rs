//! Bean and event descriptors.
//!
//! This slice uses manual impls. A later derive emits **only** [`BeanInfo`],
//! [`At`](crate::At) (per `Beans` slot), and [`EventInfo`] for events —
//! never `Query` / `Setup` / `StartDesc` / `Module` / steps.
//!
//! [`BeanInfo`] / [`EventInfo`] are **meta** traits (rule 14): do not add
//! methods or associated types without owner approval. Entry points such as
//! [`Query`](crate::query::Query) stay on separate traits.

mod inject;
mod levels;
mod proxy;
mod route;

pub use inject::{
    FromParent, Module, OfInject, OfInject0, OfInject1, OfParent, OfProvide, OfProvide0, OfProvide1,
};
pub use proxy::Marked;
pub use proxy::{ApplyProxy, FreeKey, Fwd, InitPath, ParentList, Proxied, Proxy, Stop, Target};
pub use proxy::{DriveInit, Enfold, EnterAll, ForwardInit, WrapFwd};
pub use route::Corresponds;

use crate::filter::On;

/// Standard bean descriptor.
///
/// Provides, injects, the combined bean list, and the owned state.
///
/// Meta trait — frozen without owner approval (rule 14). Use [`Query`](crate::Query)
/// to start a pipeline.
pub trait BeanInfo: Sized {
    /// Beans this type provides.
    type Provides;
    /// Beans this type injects from its parent.
    type Injects;
    /// Provides and injects together.
    type Beans;
    /// Owned state of this bean only.
    type State;
    /// `(proxy, link, chain)`.
    ///
    /// `proxy` is [`Proxy<Self>`](crate::Proxy) or a named proxy. `link` is
    /// `Get<Self>` or `Mut<Self>`. `chain` is [`Stop`](crate::Stop), or the
    /// outer proxies, nearest first. [`Parts`] reads the three back out.
    type Gate: Open<Self>;
}

/// Splits a [`BeanInfo::Gate`] tuple into proxy, link, and chain.
#[doc(hidden)]
pub trait Open<B> {
    /// Wrapping proxy.
    type Proxy: BeanInfo;
    /// `Get<B>` or `Mut<B>`.
    type Link: Marked<Target = B>;
    /// [`Stop`](crate::Stop), or the outer proxies, nearest first.
    type Chain;
}

impl<B, P, L, C> Open<B> for (P, L, C)
where
    P: BeanInfo,
    L: Marked<Target = B>,
{
    type Proxy = P;
    type Link = L;
    type Chain = C;
}

/// [`BeanInfo::Proxy`](Parts::Proxy), [`Link`](Parts::Link), and [`Chain`](Parts::Chain) read off [`BeanInfo::Gate`].
#[doc(hidden)]
pub trait Parts: BeanInfo {
    /// First element of [`Gate`](BeanInfo::Gate).
    type Proxy: BeanInfo;
    /// Second element of [`Gate`](BeanInfo::Gate).
    type Link: Marked<Target = Self>;
    /// Third element of [`Gate`](BeanInfo::Gate).
    type Chain;
}

impl<B> Parts for B
where
    B: BeanInfo,
{
    type Proxy = <B::Gate as Open<B>>::Proxy;
    type Link = <B::Gate as Open<B>>::Link;
    type Chain = <B::Gate as Open<B>>::Chain;
}

/// Extra descriptor for an event, beside [`BeanInfo`].
///
/// An event may list several parents. Meta trait — frozen without owner
/// approval (rule 14).
pub trait EventInfo: BeanInfo {
    /// Parent events of this event.
    type Parents;

    /// `Ev::ON[inner]` starts an event node.
    const ON: On<Self, ()> = On::new();
}
