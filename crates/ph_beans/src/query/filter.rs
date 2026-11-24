//! `.filter` replaces `E` when the argument is an `On` spine.

use core::marker::PhantomData;

use crate::context::Context;
use crate::context::ContextTick;
use crate::context::Ctx;
use crate::context::CtxConfig;
use crate::context::maps::Rescope;
use crate::descriptor::live::AccSet;
use crate::filter::ApplyFilter;
use crate::query::apply::ApplyPatch;
use crate::query::init::Init;

/// Rescopes the tick on a concrete [`Ctx`] (hides `B`/`A`/`S`/`T` from [`CtxConfig`]).
/// Generics: `F` filter, `E` event.
#[doc(hidden)]
pub trait SealFilter<F, E>: Sized {
    /// Context after the seal.
    type Out: Context;
    /// Stores filter `F` as the scope for the next map.
    fn seal_filter(self) -> Self::Out;
}

impl<B, A, S, T, F, E> SealFilter<F, E> for Ctx<B, A, S, T>
where
    A: AccSet,
    T: ContextTick + Rescope<F>,
    F: ApplyFilter<E, A>,
    F::NextArgs: AccSet,
{
    type Out = Ctx<B, F::NextArgs, S, T::Out>;

    fn seal_filter(self) -> Self::Out {
        Ctx::new(Rescope::<F>::rescope(self.tick))
    }
}

/// Config: new event path and the filter's rescoped tick.
/// Generics: `F` filter, `E` event.
#[doc(hidden)]
pub struct SetFilter<F, E>(PhantomData<(F, E)>);

impl<C, F, E> CtxConfig<C> for SetFilter<F, E>
where
    C: Context + SealFilter<F, E>,
    F: ApplyFilter<E, C::Args>,
{
    type Out = C::Out;
    type ScopeOf<O> = F::NextE;

    fn apply(self, ctx: C) -> Self::Out {
        let _ = self;
        ctx.seal_filter()
    }
}

/// Next pipeline after `.filter`.
/// Generics: `F` filter.
pub trait FilterStep<F> {
    /// Pipeline with this filter applied.
    type Out;
    /// Installs the filter.
    fn filter(self, filter: F) -> Self::Out;
}

impl<E, C, F> FilterStep<F> for Init<E, C>
where
    Self: ApplyPatch<SetFilter<F, E>>,
{
    type Out = <Self as ApplyPatch<SetFilter<F, E>>>::Out;

    fn filter(self, filter: F) -> Self::Out {
        let _ = filter;
        self.apply_patch(SetFilter(PhantomData))
    }
}

// Generics: `E` event, `C` context / claim / child.
impl<E, C> Init<E, C> {
    /// Installs a filter. An `On` spine replaces `E`. A predicate leaves `E`.
    ///
    /// A `Mut` marker in the filter clears the live accessor set.
    #[allow(clippy::needless_pass_by_value)]
    pub fn filter<F>(self, filter: F) -> <Self as FilterStep<F>>::Out
    where
        Self: FilterStep<F>,
    {
        FilterStep::filter(self, filter)
    }
}
