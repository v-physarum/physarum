//! A [`CtxConfig`] rewrites a [`Context`]. Its result type is [`OfConfig`].

use core::marker::PhantomData;

use super::Context;
use crate::core::config::Config;

/// Rewrites a context. The next context type is [`OfConfig`]`<`[`ApplyCfg`]`<Self, _>>`.
///
/// [`ScopeOf`](CtxConfig::ScopeOf) is the operator event path after the config
/// (unchanged, or replaced for a filter).
/// Generics: `C` context.
pub trait CtxConfig<C: Context> {
    /// Context after this config.
    type Out: Context;
    /// Event scope on `Init` after this config, given the previous scope.
    type ScopeOf<E>;
    /// Applies the config.
    fn apply(self, ctx: C) -> Self::Out;
}

/// [`CtxConfig`] `G` applied to context `C`. The product is [`Config::Type`].
///
/// Generics: `G` config, `C` context.
pub struct ApplyCfg<G, C>(PhantomData<(G, C)>);

// Generics: `G` config, `C` context.
impl<G, C> Config for ApplyCfg<G, C>
where
    C: Context,
    G: CtxConfig<C>,
{
    type Type = G::Out;
}
