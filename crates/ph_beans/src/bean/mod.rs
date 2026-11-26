//! Bean and event descriptors.
//!
//! This slice uses manual impls. A later derive emits **only** [`BeanInfo`],
//! [`At`](crate::At) (per `Beans` slot), [`EventInfo`] for events, and
//! `ProxyInfo` for `#[config(type = proxy)]` — never `Query` / `Setup` /
//! `StartDesc` / `Module` / steps (design: Derive + Proxy).
//!
//! [`BeanInfo`] / [`EventInfo`] (and later `ProxyInfo`) are **meta** traits
//! (rule 14): do not add methods or associated types without owner approval.
//! Entry points such as [`Query`](crate::query::Query) stay on separate traits.

mod info;
mod inject;
mod state;

pub use info::{BeanInfo, EventInfo, Info};
pub use inject::{FromParent, Module, OfInject, OfParent, OfProvide};
pub use state::{Blank, NoState};
