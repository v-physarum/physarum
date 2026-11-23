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

pub use inject::{
    FromParent, Module, OfInject, OfInject0, OfInject1, OfParent, OfProvide, OfProvide0, OfProvide1,
};
pub use proxy::Marked;
pub use proxy::{
    ApplyProxy, FreeKey, Fwd, InitPath, ParentList, Proxied, Proxy, Stop, Target, Via,
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
    /// Wrapping proxy. [`Proxy<Self>`](crate::Proxy) when none is named.
    type Proxy: BeanInfo;
    /// `Get<Self>` or `Mut<Self>` when `Self` wraps another bean.
    type Link: Marked<Target = Self>;
    /// [`Stop`](crate::Stop) when nothing wraps `Self`. A tuple of outer proxies, nearest first, otherwise.
    type Chain;
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
