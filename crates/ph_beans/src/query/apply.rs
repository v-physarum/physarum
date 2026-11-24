//! Applies a [`CtxConfig`](crate::context::CtxConfig) and rebuilds [`Init`](super::init::Init).

use crate::context::Context;
use crate::context::CtxConfig;
use crate::query::init::Init;

/// Applies a [`CtxConfig`](crate::context::CtxConfig) and rebuilds [`Init`](super::init::Init)
/// with [`CtxConfig::ScopeOf`](crate::context::CtxConfig::ScopeOf).
/// Generics: `G` config.
#[doc(hidden)]
pub trait ApplyPatch<G> {
    /// Pipeline tip after the config.
    type Out;
    /// Applies the config to the inner context.
    fn apply_patch(self, cfg: G) -> Self::Out;
}

impl<E, C, G> ApplyPatch<G> for Init<E, C>
where
    C: Context,
    G: CtxConfig<C>,
{
    type Out = Init<G::ScopeOf<E>, C::With<G>>;

    fn apply_patch(self, cfg: G) -> Self::Out {
        Init::new(self.into_context().configure(cfg))
    }
}
