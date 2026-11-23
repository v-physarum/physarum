//! `.inject_resources` replaces the claim set.

use core::marker::PhantomData;

use crate::bean::{ApplyProxy, BeanInfo};
use crate::context::Context;
use crate::context::ContextTick;
use crate::context::Ctx;
use crate::context::CtxConfig;
use crate::descriptor::live::AccSet;
use crate::descriptor::live::{Idle, Live};
use crate::filter::shape::HasMut;
use crate::filter::shape::No;
use crate::query::apply::ApplyPatch;
use crate::query::args::ClaimVia;
use crate::query::init::Init;

/// Replaces resource args on a concrete [`Ctx`] (hides `B`/`O`/`S`/`T` from [`CtxConfig`]).
#[doc(hidden)]
pub trait SealArgs<A>: Sized {
    /// Context after the seal.
    type Out: Context;
    /// Installs args `A`.
    fn seal_args(self) -> Self::Out;
}

impl<B, O, S, T, A> SealArgs<A> for Ctx<B, O, S, T>
where
    O: AccSet,
    A: AccSet,
    T: ContextTick,
{
    type Out = Ctx<B, A, S, T>;

    fn seal_args(self) -> Self::Out {
        Ctx::new(self.tick)
    }
}

/// Config: replace resource args with `A`.
#[doc(hidden)]
pub struct SetArgs<A>(pub(crate) PhantomData<A>);

impl<C, A> CtxConfig<C> for SetArgs<A>
where
    C: Context + SealArgs<A>,
{
    type Out = C::Out;
    type ScopeOf<E> = E;

    fn apply(self, ctx: C) -> Self::Out {
        let _ = self;
        ctx.seal_args()
    }
}

/// The next `.inject_resources` is allowed for these live accessors.
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `ErrNeedWaitFilter` (tip `{Self}`)",
    label = "fix Descriptor::Status",
    note = "a live Mut is still held; wait for it before the next .inject_resources"
)]
pub trait AllowInjectResources<N> {}

impl<N, C> AllowInjectResources<N> for Live<C> where C: HasMut<Mut = No> {}

impl<N, C> AllowInjectResources<N> for Idle<C> {}

/// Next pipeline after `.inject_resources`.
///
/// `Out` is the tip with the claim set replaced by the proxy wrapper. The
/// signature does not spell `Ctx` or `Pipe`. `K` is the inferred path
/// ([`Own`](crate::Own), [`FromProxy`](crate::FromProxy), or
/// [`FromChain`](crate::FromChain)); callers do not name it.
pub trait InjectResourcesStep<R, K> {
    /// Pipeline whose resource args are the wrapped claim.
    type Out;
    /// Replaces the claim set.
    fn inject_resources(self, resources: R) -> Self::Out;
}

impl<E, C, R, K> InjectResourcesStep<R, K> for Init<E, C>
where
    R: ApplyProxy,
    Self: ApplyPatch<SetArgs<Live<R::Wrapped>>>,
    C: Context,
    C::Bean: BeanInfo,
    R: ClaimVia<C::Bean, K>,
    C::Args: AllowInjectResources<R>,
{
    type Out = <Self as ApplyPatch<SetArgs<Live<R::Wrapped>>>>::Out;

    fn inject_resources(self, resources: R) -> Self::Out {
        let _ = resources;
        self.apply_patch(SetArgs(PhantomData))
    }
}

impl<E, C> Init<E, C> {
    /// Replaces the resource set. The argument is a marker.
    ///
    /// A live `Mut` refuses another inject until a wait clears it.
    /// See `a_live_mut_blocks_the_next_inject`.
    #[allow(clippy::needless_pass_by_value)]
    pub fn inject_resources<R, K>(self, resources: R) -> <Self as InjectResourcesStep<R, K>>::Out
    where
        Self: InjectResourcesStep<R, K>,
    {
        InjectResourcesStep::inject_resources(self, resources)
    }
}
