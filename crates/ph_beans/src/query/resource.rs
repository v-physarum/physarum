//! `.inject_resources` replaces the claim set.

use core::marker::PhantomData;

use crate::bean::BeanInfo;
use crate::context::Context;
use crate::context::ContextTick;
use crate::context::Ctx;
use crate::context::CtxConfig;
use crate::descriptor::live::AccSet;
use crate::descriptor::live::{Idle, Live};
use crate::descriptor::{OpDesc, OpOk};
use crate::filter::shape::{HasMut, JoinBoth, No, NodeForm, PairBody, Pass, Read, Yes};
use crate::query::apply::ApplyPatch;
use crate::query::claim::Claim;
use crate::query::init::Init;

/// A claim `.inject_resources` accepts: leaves (`X::T`, `*X::MUT`, …) joined by `&`.
pub trait StateGate<E> {}

/// [`NodeForm::Body`](NodeForm::Body) is allowed under scope `E`.
#[doc(hidden)]
pub trait GateParts<E> {}

// Generics: `E` scope.
impl<E> GateParts<E> for Pass {}

// Generics: `E` scope.
impl<E> GateParts<E> for Read {}

// Generics: `E` scope.
impl<E> GateParts<E> for Yes {}

// Generics: `E` scope, `A` left operator, `B` right operator — And only ([`JoinBoth`]).
impl<E, A, B> GateParts<E> for PairBody<A, B, JoinBoth>
where
    A: StateGate<E>,
    B: StateGate<E>,
{
}

// Generics: `E` scope, `T` operator ([`NodeForm`]).
impl<E, T> StateGate<E> for T
where
    T: NodeForm,
    T::Body: GateParts<E>,
{
}

/// Replaces resource args on a concrete [`Ctx`] (hides `B`/`O`/`S`/`T` from [`CtxConfig`]).
/// Generics: `A` args.
#[doc(hidden)]
pub trait SealArgs<A>: Sized {
    /// Context after the seal.
    type Out: Context;
    /// Installs args `A`.
    fn seal_args(self) -> Self::Out;
}

// Generics: `B` right operator — And only, `O` its result, `S` strategy, `T` operator, `A` new args.
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
/// Replaces context args with claim set `A`. Generics: `A` new args.
#[doc(hidden)]
pub struct SetArgs<A> {
    pub(crate) _markers: PhantomData<A>,
}

// Generics: `C` context, `A` new args.
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

/// Live set may take the next `.inject_resources`. Generics: `N` new claim.
/// ```compile_fail
/// use ph_beans::{At, BeanInfo, EventInfo, Ix, Marker, Query};
/// struct Keys; struct Main; struct App;
/// impl BeanInfo for Keys { type Provides=(); type Injects=(); type Beans=(); type State=(); }
/// impl BeanInfo for Main { type Provides=(); type Injects=(); type Beans=(); type State=(); }
/// impl EventInfo for Main { type Parents=(); } impl BeanInfo for App { type Provides=(); type Injects=(); type Beans=(); type State=(); }
/// impl At<Keys> for App { type Ix = Ix<0>; }
/// let _ = App::query().inject_resources(Keys::MUT).filter(Main::ON[Keys::T]).inject_resources(Keys::T);
/// ```
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `FilterError<2>` (tip `{Self}`)",
    label = "fix Descriptor::Status",
    note = "a live Mut is still held; wait before the next .inject_resources (FilterError<2>)"
)]
// Generics: `N` new claim.
pub trait AllowInjectResources<N> {}

// Generics: `N` new claim, `C` inner claim set.
impl<N, C> AllowInjectResources<N> for Live<C> where C: HasMut<Mut = No> {}

// Generics: `N` new claim, `C` remembered claim set.
impl<N, C> AllowInjectResources<N> for Idle<C> {}

/// Next pipeline after `.inject_resources`.
///
/// `Out` is the tip with the claim set replaced. The signature does not
/// spell `Ctx` or `Pipe`.
/// Generics: `R` claim set.
pub trait InjectResourcesStep<R> {
    /// Pipeline whose resource args are `R`.
    type Out;
    /// Replaces the claim set.
    fn inject_resources(self, resources: R) -> Self::Out;
}

// Generics: `E` event scope, `C` context, `R` claim set.
impl<E, C, R> InjectResourcesStep<R> for Init<E, C>
where
    Self: ApplyPatch<SetArgs<Live<R>>>,
    C: Context,
    C::Bean: BeanInfo,
    R: Claim<C::Bean> + OpDesc<C::Bean, Status: OpOk> + StateGate<E>,
    C::Args: AllowInjectResources<R>,
{
    type Out = <Self as ApplyPatch<SetArgs<Live<R>>>>::Out;

    fn inject_resources(self, resources: R) -> Self::Out {
        let _ = resources;
        self.apply_patch(SetArgs { _markers: PhantomData })
    }
}

// Generics: `E` event, `C` context / claim / child.
impl<E, C> Init<E, C> {
    /// Replaces the claim set. `*E::MUT` needs `E` in scope.
    /// ```compile_fail
    /// use ph_beans::{BeanInfo, EventInfo, Marker, Query};
    /// struct Update; struct App;
    /// impl BeanInfo for Update { type Provides=(); type Injects=(); type Beans=(); type State=(); }
    /// impl EventInfo for Update { type Parents=(); }
    /// impl BeanInfo for App { type Provides=(); type Injects=(); type Beans=(); type State=(); }
    /// let _ = App::query().inject_resources(*Update::MUT);
    /// ```
    #[allow(clippy::needless_pass_by_value)]
    pub fn inject_resources<R>(self, resources: R) -> <Self as InjectResourcesStep<R>>::Out
    where
        Self: InjectResourcesStep<R>,
    {
        InjectResourcesStep::inject_resources(self, resources)
    }
}
