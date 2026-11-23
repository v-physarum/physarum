//! One config struct drives [`Context::With`](super::Context::With).

use super::Context;

/// Rewrites a context. [`Context::With`] is this config's [`Out`](CtxConfig::Out).
///
/// [`ScopeOf`](CtxConfig::ScopeOf) is the operator event path after the config
/// (unchanged, or replaced for a filter). [`crate::query::init::ApplyPatch`]
/// builds `Init<ScopeOf<E>, C::With<G>>`.
pub trait CtxConfig<C: Context> {
    /// Context after this config.
    type Out: Context;
    /// Event scope on `Init` after this config, given the previous scope.
    type ScopeOf<E>;
    /// Applies the config.
    fn apply(self, ctx: C) -> Self::Out;
}
