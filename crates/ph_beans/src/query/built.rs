//! `.build()` seals the pipeline with [`Valid`](crate::Valid).

use core::marker::PhantomData;

use crate::bean::BeanInfo;
use crate::context::ApplyCfg;
use crate::context::Context;
use crate::context::ContextTick;
use crate::context::Ctx;
use crate::context::CtxConfig;
use crate::core::config::OfConfig;
use crate::descriptor::Descriptor;
use crate::descriptor::NeedBuild;
use crate::descriptor::Valid;
use crate::descriptor::live::AccSet;
use crate::query::init::Init;

/// Replaces status on a concrete [`Ctx`] (hides `B`/`A`/`O`/`T` from [`CtxConfig`]).
/// Generics: `S` status.
#[doc(hidden)]
pub trait SealStatus<S>: Sized {
    /// Context after the seal.
    type Out: Context;
    /// Installs status `S`.
    fn seal_status(self) -> Self::Out;
}

// Generics: `B` module, `A` left child, `O` its result, `T` operator, `S` status.
impl<B, A, O, T, S> SealStatus<S> for Ctx<B, A, O, T>
where
    A: AccSet,
    T: ContextTick,
{
    type Out = Ctx<B, A, S, T>;

    fn seal_status(self) -> Self::Out {
        Ctx::new(self.tick)
    }
}

/// Config: replace status with `S`.
/// Generics: `S` status.
#[doc(hidden)]
pub struct SetStatus<S> {
    pub(crate) _markers: PhantomData<S>,
}

// Generics: `C` context, `S` status.
impl<C, S> CtxConfig<C> for SetStatus<S>
where
    C: Context + SealStatus<S>,
{
    type Out = C::Out;
    type ScopeOf<E> = E;

    fn apply(self, ctx: C) -> Self::Out {
        let _ = self;
        ctx.seal_status()
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
    C: Context<
            Status = NeedBuild,
            With<SetStatus<Valid>> = OfConfig<ApplyCfg<SetStatus<Valid>, C>>,
        >,
    C::Bean: BeanInfo,
    SetStatus<Valid>: CtxConfig<C>,
{
    type Out = Built<E, OfConfig<ApplyCfg<SetStatus<Valid>, C>>>;

    fn build(self) -> Self::Out {
        Built::new(
            self.into_context()
                .configure(SetStatus { _markers: PhantomData }),
        )
    }
}

// Generics: `E` event, `C` context.
impl<E, C> Init<E, C> {
    /// Seals the pipeline. Status becomes [`Valid`](crate::Valid).
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
