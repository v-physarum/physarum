//! `.build()` seals the pipeline with [`Valid`](crate::Valid).

use core::marker::PhantomData;

use crate::bean::{BeanInfo, NoState};
use crate::context::ApplyCfg;
use crate::context::Context;
use crate::context::ContextTick;
use crate::context::Ctx;
use crate::context::CtxConfig;
use crate::context::{FreshInit, Sys};
use crate::core::config::OfConfig;
use crate::descriptor::Descriptor;
use crate::descriptor::NeedBuild;
use crate::descriptor::NeedInit;
use crate::descriptor::Valid;
use crate::descriptor::live::AccSet;
use crate::query::boot::NoInit;
use crate::query::init::Init;

/// Seals a concrete [`Ctx`] for `.build()` (hides `B`/`A`/`T` from
/// [`CtxConfig`]).
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `NeedInit` (tip `{Self}`)",
    label = "fix Descriptor::Status",
    note = "the pipeline of a module with a state has no init function"
)]
pub trait SealBuilt: Sized {
    /// Context after the seal: status [`Valid`].
    type Out: Context;
    /// Installs [`Valid`].
    fn seal_built(self) -> Self::Out;
}

// `init_state` has run. Generics: `B` module, `A` args, `T` tick.
impl<B, A, T> SealBuilt for Ctx<B, A, NeedBuild, T>
where
    A: AccSet,
    T: ContextTick,
{
    type Out = Ctx<B, A, Valid, T>;

    fn seal_built(self) -> Self::Out {
        Ctx::new(self.tick)
    }
}

// A module without state has no init function: the library makes its
// `NoState`. Generics: `B` module, `A` args, `T` tick.
impl<B, A, T> SealBuilt for Ctx<B, A, NeedInit, T>
where
    A: AccSet,
    T: FreshInit,
    B: BeanInfo<State = NoState>,
{
    type Out = Ctx<B, A, Valid, T::Ready<Sys<B, A, NoInit>>>;

    fn seal_built(self) -> Self::Out {
        Ctx::new(self.tick.install(Sys::new(NoInit)))
    }
}

/// Config: seal the pipeline.
#[doc(hidden)]
pub struct SetBuilt;

// Generics: `C` context.
impl<C> CtxConfig<C> for SetBuilt
where
    C: Context + SealBuilt,
{
    type Out = C::Out;
    type ScopeOf<E> = E;

    fn apply(self, ctx: C) -> Self::Out {
        ctx.seal_built()
    }
}

/// Pipeline after `.build()`. Status is [`Valid`](crate::Valid).
///
/// It holds only systems. [`Tick::init`](crate::Tick::init) makes an instance
/// with the state; one pipeline can run several instances.
/// Generics: `E` event, `C` context.
#[must_use = "call init, then emit into the instance"]
pub struct Built<E, C> {
    context: C,
    _markers: PhantomData<E>,
}

// Generics: `E` event, `C` context.
impl<E, C> Built<E, C> {
    fn new(context: C) -> Self {
        Self { context, _markers: PhantomData }
    }
}

// Generics: `E` event, `C` context.
impl<E, C> Descriptor for Built<E, C>
where
    C: Descriptor,
{
    type Status = C::Status;
    type Accessors = C::Accessors;
}

/// Next pipeline after `.build`.
pub trait BuildStep {
    /// Sealed pipeline. Status is [`Valid`](crate::Valid).
    type Out;
    /// Seals the pipeline.
    fn build(self) -> Self::Out;
}

// Generics: `E` event, `C` context.
impl<E, C> BuildStep for Init<E, C>
where
    C: Context<With<SetBuilt> = OfConfig<ApplyCfg<SetBuilt, C>>>,
    SetBuilt: CtxConfig<C>,
{
    type Out = Built<E, OfConfig<ApplyCfg<SetBuilt, C>>>;

    fn build(self) -> Self::Out {
        Built::new(self.into_context().configure(SetBuilt))
    }
}

// Generics: `E` event, `C` context.
impl<E, C> Init<E, C> {
    /// Seals the pipeline. Status becomes [`Valid`](crate::Valid).
    ///
    /// A module with a state calls `.init_state` first. A module whose
    /// state is [`NoState`](crate::NoState) is built without it.
    pub fn build(self) -> <Self as BuildStep>::Out
    where
        Self: BuildStep,
    {
        BuildStep::build(self)
    }
}

// Generics: `E` event, `C` context.
impl<E, C> Built<E, C>
where
    C: Context,
{
    pub(crate) fn tick(&self) -> &C::Tick {
        self.context.tick()
    }
}
