//! `.build()` seals the pipeline with [`Valid`](crate::Valid).

use core::marker::PhantomData;

use crate::bean::BeanInfo;
use crate::context::Context;
use crate::context::ContextTick;
use crate::context::Ctx;
use crate::context::CtxConfig;
use crate::context::DefaultState;
use crate::context::list::WalkStrategy;
use crate::descriptor::Descriptor;
use crate::descriptor::NeedBuild;
use crate::descriptor::OpOk;
use crate::descriptor::Valid;
use crate::descriptor::live::AccSet;
use crate::filter::eval::EmitMask;
use crate::query::args::ClaimShape;
use crate::query::init::Init;
use crate::query::schedule::Drive;

/// Replaces status on a concrete [`Ctx`] (hides `B`/`A`/`O`/`T` from [`CtxConfig`]).
/// Generics: `S` status.
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
/// Generics: `S` status.
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
/// Generics: `E` event, `C` context.
#[must_use = "the pipeline is ready to emit"]
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

// Generics: `E` event, `C` context.
impl<E, C> BuildStep for Init<E, C>
where
    C: Context<Status = NeedBuild>,
    C::Bean: BeanInfo,
    C::Args: ClaimShape<Status: OpOk>,
    SetStatus<Valid>: CtxConfig<C>,
{
    type Out = Built<E, C::With<SetStatus<Valid>>>;

    fn build(self) -> Self::Out {
        Built::new(self.into_context().configure(SetStatus(PhantomData)))
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

// Generics: `E` event, `C` context.
impl<E, C> Built<E, C>
where
    C: Context,
{
    pub(crate) fn drive<G: WalkStrategy + Copy>(&mut self, strategy: G)
    where
        C::Tick: Drive,
    {
        self.context.tick().drive(strategy);
    }

    pub(crate) fn defer_into(&mut self, bit: u32, mask: EmitMask)
    where
        C::Tick: Drive,
    {
        self.context.tick().defer_into(bit, mask);
    }

    pub(crate) fn take_scope(&mut self, bit: u32) -> EmitMask
    where
        C::Tick: Drive,
    {
        self.context.tick().take_scope(bit)
    }
}
