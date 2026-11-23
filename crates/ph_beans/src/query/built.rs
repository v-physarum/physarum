//! `.build()` seals the pipeline with [`Valid`](crate::Valid).

use core::marker::PhantomData;

use crate::bean::BeanInfo;
use crate::context::Context;
use crate::context::ContextTick;
use crate::context::Ctx;
use crate::context::CtxConfig;
use crate::context::DefaultState;
use crate::descriptor::Descriptor;
use crate::descriptor::NeedBuild;
use crate::descriptor::OpOk;
use crate::descriptor::OpShape;
use crate::descriptor::Valid;
use crate::descriptor::live::AccSet;
use crate::query::init::Init;
use crate::query::schedule::Drive;

/// Replaces status on a concrete [`Ctx`] (hides `B`/`A`/`O`/`T` from [`CtxConfig`]).
#[doc(hidden)]
pub trait SealStatus<S>: Sized {
    /// Context after the seal.
    type Out: Context;
    /// Installs status `S`.
    fn seal_status(self) -> Self::Out;
}

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
#[doc(hidden)]
pub struct SetStatus<S>(pub(crate) PhantomData<S>);

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
#[must_use = "the pipeline is ready to update"]
pub struct Built<E, C> {
    context: C,
    _markers: PhantomData<E>,
}

impl<E, C> Built<E, C> {
    fn new(context: C) -> Self {
        Self { context, _markers: PhantomData }
    }
}

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

impl<E, C> BuildStep for Init<E, C>
where
    C: Context<Status = NeedBuild>,
    C::Bean: BeanInfo,
    C::Args: OpShape<Status: OpOk>,
    SetStatus<Valid>: CtxConfig<C>,
{
    type Out = Built<E, C::With<SetStatus<Valid>>>;

    fn build(self) -> Self::Out {
        Built::new(self.into_context().configure(SetStatus(PhantomData)))
    }
}

impl<E, C> Init<E, C> {
    /// Seals the pipeline. Status becomes [`Valid`](crate::Valid).
    pub fn build(self) -> <Self as BuildStep>::Out
    where
        Self: BuildStep,
    {
        BuildStep::build(self)
    }
}

impl<E, C> Built<E, C>
where
    C: Context<Status = Valid>,
    C::Bean: BeanInfo<State = <C::Tick as DefaultState>::State>,
    C::Tick: DefaultState,
{
    /// State created by [`init_state_default`](crate::Init::init_state_default).
    #[must_use]
    pub fn state(&self) -> &<C::Bean as BeanInfo>::State {
        self.context.tick().state()
    }
}

impl<E, C> Built<E, C>
where
    C: Context,
    C::Tick: Drive,
{
    pub(crate) fn drive_maps(&mut self) {
        self.context.tick_mut().drive();
    }
}
