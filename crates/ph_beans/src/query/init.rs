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
use crate::descriptor::Descriptor;
use crate::descriptor::NeedBuild;
use crate::descriptor::OpOk;
use crate::descriptor::live::AccSet;
use crate::query::apply::ApplyPatch;
use crate::query::args::{Claim, ClaimShape};

/// Pipeline before `.build()`.
///
/// `E` is the event path. `()` means no event scope yet.
/// Generics: `E` event, `C` context.
#[must_use = "call inject_resources, init_state, and build"]
pub struct Init<E, C> {
    context: C,
    _markers: PhantomData<E>,
}

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
    /// Installs `handler` and moves status to [`NeedBuild`].
    fn seal_init(self, handler: H) -> Self::Out;
}

impl<B, A, S, T, H> SealInit<H> for Ctx<B, A, S, T>
where
    A: AccSet,
    T: FreshInit,
{
    type Out = Ctx<B, A, NeedBuild, T::WithInit<H>>;

    fn seal_init(self, handler: H) -> Self::Out {
        Ctx::new(FreshInit::install_init(self.tick, handler))
    }
}

/// Config: store init function `H` and move status to [`NeedBuild`].
/// Generics: `H` init handler.
#[doc(hidden)]
pub struct SetInit<H>(pub(crate) H);

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

/// Stores [`Default`] bean state on a concrete [`Ctx`].
///
/// `D` is [`BeanInfo::State`] of the context bean — not a type param on the
/// config (unlike [`SetInit`], which carries a real handler value).
#[doc(hidden)]
pub trait SealDefault: Sized {
    /// Context after the seal.
    type Out: Context;
    /// Installs [`Default::default`] for the bean state and moves status to [`NeedBuild`].
    fn seal_default(self) -> Self::Out;
}

// Generics: `B` bean, `A` args, `S` status, `T` tick.
impl<B, A, S, T> SealDefault for Ctx<B, A, S, T>
where
    A: AccSet,
    T: FreshInit,
    B: BeanInfo,
    B::State: Default,
{
    type Out = Ctx<B, A, NeedBuild, T::WithDefault<B::State>>;

    fn seal_default(self) -> Self::Out {
        Ctx::new(FreshInit::install_default(self.tick, Default::default()))
    }
}

/// Config: store [`Default`] state of the context bean and move status to [`NeedBuild`].
#[doc(hidden)]
pub struct SetDefault;

// Generics: `C` context.
impl<C> CtxConfig<C> for SetDefault
where
    C: Context + SealDefault,
{
    type Out = C::Out;
    type ScopeOf<E> = E;

    fn apply(self, ctx: C) -> Self::Out {
        ctx.seal_default()
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

impl<E, C, H> InitStateStep<H> for Init<E, C>
where
    Self: ApplyPatch<SetInit<H>>,
    C: Context,
    C::Bean: BeanInfo,
    C::Tick: FreshInit,
    C::Args: Claim<C::Bean, Status: OpOk>,
    H: for<'a> Fn(<C::Args as ClaimShape>::View<'a>) -> <C::Bean as BeanInfo>::State,
{
    type Out = <Self as ApplyPatch<SetInit<H>>>::Out;

    fn init_state(self, handler: H) -> Self::Out {
        self.apply_patch(SetInit(handler))
    }
}

/// Next pipeline after `.init_state_default`.
pub trait InitStateDefaultStep {
    /// Pipeline with `Default` state stored.
    type Out;
    /// Stores that state.
    fn init_state_default(self) -> Self::Out;
}

// Generics: `E` event, `C` context.
impl<E, C> InitStateDefaultStep for Init<E, C>
where
    C: Context,
    C::Args: ClaimShape<Status: OpOk>,
    Self: ApplyPatch<SetDefault>,
{
    type Out = <Self as ApplyPatch<SetDefault>>::Out;

    fn init_state_default(self) -> Self::Out {
        self.apply_patch(SetDefault)
    }
}

impl<E, C> Init<E, C> {
    /// Stores a pure init `fn` (rule 18). Status → [`NeedBuild`](crate::NeedBuild).
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

    /// Stores [`Default`] state. Status → [`NeedBuild`](crate::NeedBuild).
    /// ```compile_fail
    /// use ph_beans::{BeanInfo, Query};
    /// struct App; impl BeanInfo for App { type Provides=(); type Injects=(); type Beans=(); type State=u8; }
    /// let _ = App::query().init_state_default().init_state_default();
    /// ```
    pub fn init_state_default(self) -> <Self as InitStateDefaultStep>::Out
    where
        Self: InitStateDefaultStep,
    {
        InitStateDefaultStep::init_state_default(self)
    }
}
