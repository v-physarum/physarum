//! Proxy levels as one linear leaf.
//!
//! [`Rows`](LevelDesc::Rows) is `List<Leaf<(Direct, Outer, ...)>>`, the direct
//! proxy first. [`OK`](LevelDesc::OK) is `1` when the module `Beans` and those
//! proxies' `Provides` share no id. The same id panics: `bean is already on the proxy`.

use super::OpOk;

/// Levels of one bean's proxy chain, and whether a bean is already on that chain.
pub trait LevelDesc {
    /// Linear leaf of proxy levels, direct proxy first.
    type Rows;
    /// [`Valid`](crate::Valid) when the module list and the provide lists are disjoint.
    type Status: OpOk;
    /// `1` when the lists are disjoint. Panics when a bean is on both.
    const OK: u8;
}
