//! Bean and event descriptors.
//!
//! This slice uses manual impls. A later derive emits **only** [`BeanInfo`],
//! [`At`](crate::At) (per `Beans` slot), [`EventInfo`] for events, and
//! [`ProxyInfo`] for `#[config(type = proxy)]` — never `Query` / `Setup` /
//! `StartDesc` / `Module` / steps (design: Derive + Proxy).
//!
//! [`BeanInfo`] / [`EventInfo`] / [`ProxyInfo`] are **meta** traits (rule 14):
//! do not add methods or associated types without owner approval. Entry points
//! such as [`Query`](crate::query::Query) stay on separate traits.

mod inject;
mod proxy;

pub use inject::{
    FromParent, Module, OfInject, OfInject0, OfInject1, OfParent, OfProvide, OfProvide0, OfProvide1,
};
pub use proxy::{
    ApplyProxy, FreeKey, Fwd, InitPath, LinkMark, More, ParentList, Proxied, Proxy, ProxyInfo,
    Stop, Target, Via,
};
pub use proxy::{DriveInit, Enfold, EnterAll, ForwardInit, WrapFwd};

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
