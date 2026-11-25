//! `.map` appends one handler, up to [`MAX_MAP_SYSTEMS`](crate::MAX_MAP_SYSTEMS).

use crate::context::Context;
use crate::context::ContextTick;
use crate::context::Ctx;
use crate::context::CtxConfig;
use crate::context::Sys;
use crate::context::maps::PushMap;
use crate::descriptor::live::AccSet;
use crate::query::apply::ApplyPatch;
use crate::query::claim::Claim;
use crate::query::init::Init;

/// Pushes one map on a concrete [`Ctx`] (hides `B`/`A`/`S`/`T` from [`CtxConfig`]).
/// Generics: `H` handler.
#[doc(hidden)]
pub trait SealMap<H>: Sized {
    /// Context after the seal.
    type Out: Context;
    /// Stores `handler`.
    fn seal_map(self, handler: H) -> Self::Out;
}

// The system reads the claim that is live at this `.map` ([`AccSet::Set`]).
// Generics: `B` bean, `A` args, `S` status, `T` tick, `H` handler.
impl<B, A, S, T, H> SealMap<H> for Ctx<B, A, S, T>
where
    A: AccSet<Set: Claim<B>>,
    T: ContextTick + PushMap<Sys<B, A::Set, H>>,
    H: for<'a> Fn(<A::Set as Claim<B>>::Arg<'a>),
{
    type Out = Ctx<B, A, S, T::Out>;

    fn seal_map(self, handler: H) -> Self::Out {
        Ctx::new(PushMap::push_map(self.tick, Sys::new(handler)))
    }
}

/// Config: push one map handler onto the tick.
/// Generics: `H` handler.
#[doc(hidden)]
pub struct SetMap<H>(H);

// Generics: `C` context / claim / child, `H` handler.
impl<C, H> CtxConfig<C> for SetMap<H>
where
    C: Context + SealMap<H>,
{
    type Out = C::Out;
    type ScopeOf<O> = O;

    fn apply(self, ctx: C) -> Self::Out {
        ctx.seal_map(self.0)
    }
}

/// Next pipeline after `.map`.
///
/// `Out` hides the tick tuple. The inherent method returns this associated type.
/// Generics: `H` handler.
pub trait MapStep<H> {
    /// Pipeline with `H` stored under the current filter.
    type Out;
    /// Stores the handler.
    fn map(self, handler: H) -> Self::Out;
}

// Generics: `E` event, `C` context / claim / child, `H` handler.
impl<E, C, H> MapStep<H> for Init<E, C>
where
    Self: ApplyPatch<SetMap<H>>,
{
    type Out = <Self as ApplyPatch<SetMap<H>>>::Out;

    fn map(self, handler: H) -> Self::Out {
        self.apply_patch(SetMap(handler))
    }
}

// Generics: `E` event, `C` context / claim / child.
impl<E, C> Init<E, C> {
    /// Stores a pure map `fn` (rule 18). Prefer a function item over a closure.
    /// Its argument is the view of the claim that is live here, `()` when none is.
    /// The 17th call is [`PipelineError`](crate::PipelineError)`<2>` (see `docs/errors.md`).
    pub fn map<H>(self, handler: H) -> <Self as MapStep<H>>::Out
    where
        Self: MapStep<H>,
    {
        MapStep::map(self, handler)
    }
}
