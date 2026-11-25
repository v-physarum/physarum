//! Applies a [`CtxConfig`](crate::context::CtxConfig) and rebuilds [`Init`](super::init::Init).

use crate::context::ApplyCfg;
use crate::context::Context;
use crate::context::CtxConfig;
use crate::core::config::OfConfig;
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

// Generics: `E` event, `C` context, `G` config.
impl<E, C, G> ApplyPatch<G> for Init<E, C>
where
    C: Context<With<G> = OfConfig<ApplyCfg<G, C>>>,
    G: CtxConfig<C>,
{
    type Out = Init<G::ScopeOf<E>, OfConfig<ApplyCfg<G, C>>>;

    fn apply_patch(self, cfg: G) -> Self::Out {
        Init::new(self.into_context().configure(cfg))
    }
}
