//! `Init`: record resource claims and one init system.
//!
//! Seal + [`CtxConfig`](crate::context::CtxConfig) for init live here; unpacking
//! [`Ctx`](crate::context::Ctx) stays in the seal traits.

use core::marker::PhantomData;

use crate::bean::BeanInfo;
use crate::context::Context;
use crate::context::Ctx;
use crate::context::CtxConfig;
use crate::context::FreshInit;
use crate::context::Sys;
use crate::descriptor::Descriptor;
use crate::descriptor::NeedBuild;
use crate::descriptor::live::AccSet;
use crate::query::apply::ApplyPatch;
use crate::query::boot::InitSys;
use crate::query::claim::Claim;

/// Pipeline before `.build()`.
///
/// `E` is the event path. `()` means no event scope yet.
/// Generics: `E` event, `C` context.
#[must_use = "call inject_resources, init_state, and build"]
pub struct Init<E, C> {
    context: C,
    _markers: PhantomData<E>,
}

// Generics: `E` event, `C` context.
impl<E, C> Init<E, C> {
    pub(crate) fn new(context: C) -> Self {
        Self { context, _markers: PhantomData }
    }

    pub(crate) fn into_context(self) -> C {
        self.context
    }
}

// Generics: `E` event, `C` context.
impl<E, C> Descriptor for Init<E, C>
where
    C: Descriptor,
{
    type Status = C::Status;
    type Accessors = C::Accessors;
}

/// Stores init on a concrete [`Ctx`] (hides `B`/`A`/`S`/`T` from [`CtxConfig`]).
/// Generics: `H` init handler.
#[doc(hidden)]
pub trait SealInit<H>: Sized {
    /// Context after the seal.
    type Out: Context;
    /// Stores `handler` with the live claim and moves status to [`NeedBuild`].
    fn seal_init(self, handler: H) -> Self::Out;
}

// Generics: `B` bean, `A` args, `S` status, `T` tick, `H` init handler.
impl<B, A, S, T, H> SealInit<H> for Ctx<B, A, S, T>
where
    A: AccSet,
    T: FreshInit,
    B: BeanInfo,
    H: InitSys<B, A>,
{
    type Out = Ctx<B, A, NeedBuild, T::Ready<Sys<B, A, H>>>;

    fn seal_init(self, handler: H) -> Self::Out {
        Ctx::new(self.tick.install(Sys::new(handler)))
    }
}

/// Config: store init function `H` and move status to [`NeedBuild`].
/// Generics: `H` init handler.
#[doc(hidden)]
pub struct SetInit<H>(pub(crate) H);

// Generics: `C` context, `H` init handler.
impl<C, H> CtxConfig<C> for SetInit<H>
where
    C: Context + SealInit<H>,
{
    type Out = C::Out;
    type ScopeOf<E> = E;

    fn apply(self, ctx: C) -> Self::Out {
        ctx.seal_init(self.0)
    }
}

/// Next pipeline after `.init_state`.
/// Generics: `H` init handler.
pub trait InitStateStep<H> {
    /// Pipeline with the init function stored.
    type Out;
    /// Stores the function item.
    fn init_state(self, handler: H) -> Self::Out;
}

/// What an init function returns, `Self`, is the state `S` of its module.
///
/// A module whose state is [`NoState`](crate::NoState) has no such
/// function: nothing outside the library makes that value.
/// Generics: `S` state of the module.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `PipelineError<9>` (init returns `{Self}`)",
    label = "fix Descriptor::Status",
    note = "the init function does not return the module state `{S}`; PipelineError<9> — docs/errors.md"
)]
pub trait InitOf<S> {}

// Generics: `S` state.
impl<S> InitOf<S> for S {}

// Generics: `E` event, `C` context, `H` init handler, `R` what it returns.
impl<E, C, H, R> InitStateStep<H> for Init<E, C>
where
    Self: ApplyPatch<SetInit<H>>,
    C: Context,
    C::Bean: BeanInfo,
    C::Tick: FreshInit,
    C::Args: Claim<C::Bean>,
    H: for<'a> Fn(<C::Args as Claim<C::Bean>>::Arg<'a>) -> R,
    R: InitOf<<C::Bean as BeanInfo>::State>,
{
    type Out = <Self as ApplyPatch<SetInit<H>>>::Out;

    fn init_state(self, handler: H) -> Self::Out {
        self.apply_patch(SetInit(handler))
    }
}

// Generics: `E` event, `C` context.
impl<E, C> Init<E, C> {
    /// Stores a pure init function (rule 18) with the live claim. Status → [`NeedBuild`](crate::NeedBuild).
    ///
    /// The function runs once per instance, in [`Tick::init`](crate::Tick::init):
    /// it sees the provided states and the injects passed there.
    /// A default state is `.init_state(|_| Default::default())`.
    /// A module with `State = NoState` takes no init function
    /// ([`PipelineError`](crate::PipelineError)`<9>`) and is built without one.
    /// A second call is [`PipelineError`](crate::PipelineError)`<1>` (see `docs/errors.md`).
    /// ```compile_fail
    /// use ph_beans::{BeanInfo, Query};
    /// struct App; impl BeanInfo for App { type Provides=(); type Injects=(); type Beans=(); type State=u8; }
    /// fn boot(_: ()) -> u8 { 1 }
    /// let _ = App::query().init_state(boot).init_state(boot);
    /// ```
    pub fn init_state<H>(self, handler: H) -> <Self as InitStateStep<H>>::Out
    where
        Self: InitStateStep<H>,
    {
        InitStateStep::init_state(self, handler)
    }
}
