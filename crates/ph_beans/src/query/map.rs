//! `.map` appends one handler, up to [`MAX_MAP_SYSTEMS`](crate::MAX_MAP_SYSTEMS).

use crate::context::Context;
use crate::context::ContextTick;
use crate::context::Ctx;
use crate::context::CtxConfig;
use crate::context::maps::{PushAt, PushMap};
use crate::descriptor::live::AccSet;
use crate::query::apply::ApplyPatch;
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

impl<B, A, S, T, H> SealMap<H> for Ctx<B, A, S, T>
where
    A: AccSet,
    T: ContextTick + PushMap<H>,
{
    type Out = Ctx<B, A, S, T::Out>;

    fn seal_map(self, handler: H) -> Self::Out {
        Ctx::new(PushMap::push_map(self.tick, handler))
    }
}

/// Config: push one map handler onto the tick.
/// Generics: `H` handler.
#[doc(hidden)]
pub struct SetMap<H>(H);

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
    /// The 17th call is [`PipelineError`](crate::PipelineError)`<2>` (see `docs/errors.md`).
    pub fn map<H>(self, handler: H) -> <Self as MapStep<H>>::Out
    where
        H: Fn(),
        Self: MapStep<H>,
    {
        MapStep::map(self, handler)
    }

    /// Stores one handler that can emit a child or defer an event.
    ///
    /// The argument is [`Direct`](crate::Direct) for the innermost event of the
    /// current filter. The 17th map still has no push impl.
    pub fn map_at<H>(self, handler: H) -> <Self as MapAtStep<H>>::Out
    where
        Self: MapAtStep<H>,
    {
        MapAtStep::map_at(self, handler)
    }
}

/// Next pipeline after `.map_at`.
/// Generics: `H` handler.
pub trait MapAtStep<H> {
    /// Pipeline with `H` stored under the current filter.
    type Out;
    /// Stores the handler.
    fn map_at(self, handler: H) -> Self::Out;
}

/// Pushes one `.map_at` handler on a concrete [`Ctx`].
/// Generics: `H` handler.
#[doc(hidden)]
pub trait SealAt<H>: Sized {
    /// Context after the seal.
    type Out: Context;
    /// Stores `handler`.
    fn seal_at(self, handler: H) -> Self::Out;
}

impl<B, A, S, T, H> SealAt<H> for Ctx<B, A, S, T>
where
    A: AccSet,
    T: ContextTick + PushAt<H>,
{
    type Out = Ctx<B, A, S, T::Out>;

    fn seal_at(self, handler: H) -> Self::Out {
        Ctx::new(PushAt::push_at(self.tick, handler))
    }
}

/// Config: push one `.map_at` handler.
/// Generics: `H` handler.
#[doc(hidden)]
pub struct SetAt<H>(H);

impl<C, H> CtxConfig<C> for SetAt<H>
where
    C: Context + SealAt<H>,
{
    type Out = C::Out;
    type ScopeOf<O> = O;

    fn apply(self, ctx: C) -> Self::Out {
        ctx.seal_at(self.0)
    }
}

// Generics: `E` event, `C` context / claim / child, `H` handler / head.
impl<E, C, H> MapAtStep<H> for Init<E, C>
where
    Self: ApplyPatch<SetAt<H>>,
{
    type Out = <Self as ApplyPatch<SetAt<H>>>::Out;

    fn map_at(self, handler: H) -> Self::Out {
        self.apply_patch(SetAt(handler))
    }
}
