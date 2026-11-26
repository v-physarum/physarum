//! `.forward` runs the pipeline of a provided bean under the current filter.

use core::marker::PhantomData;

use crate::context::Context;
use crate::context::ContextTick;
use crate::context::Ctx;
use crate::context::CtxConfig;
use crate::context::Pipe;
use crate::context::maps::MapList;
use crate::context::stages::Enlist;
use crate::descriptor::live::AccSet;
use crate::query::apply::ApplyPatch;
use crate::query::forward_list::Forwards;
use crate::query::init::Init;

/// Appends one forwarded child under the list's current filter.
/// Generics: `P` child bean, `B` its parent module.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "at most 16 map systems are supported (Status = PipelineError<2>)",
    label = "fix Descriptor::Status",
    note = "MAX_MAP_SYSTEMS = 16; PipelineError<2> — docs/errors.md"
)]
pub trait PushForward<P, B> {
    /// Tick after the push.
    type Out: ContextTick;
    /// Stores the entry on the map list.
    fn push_forward(self) -> Self::Out;
}

// Generics: `P` child, `B` parent, `I` init slot, `N` entries, `E` emit tree,
// `S` filter scope.
impl<P, B, I, N, E, S> PushForward<P, B> for Pipe<I, MapList<N, E, S>>
where
    I: ContextTick,
    N: Enlist<S, P, B>,
{
    type Out = Pipe<I, MapList<N::Out, E, S>>;

    fn push_forward(self) -> Self::Out {
        Pipe::new(self.init, self.list.push_forward())
    }
}

/// Pushes one forwarded child on a concrete [`Ctx`] (hides `B`/`A`/`S`/`T`
/// from [`CtxConfig`]). Generics: `P` child bean.
#[doc(hidden)]
pub trait SealForward<P>: Sized {
    /// Context after the seal.
    type Out: Context;
    /// Stores the entry.
    fn seal_forward(self) -> Self::Out;
}

// Generics: `B` bean, `A` args, `S` status, `T` tick, `P` child bean.
impl<B, A, S, T, P> SealForward<P> for Ctx<B, A, S, T>
where
    A: AccSet,
    T: ContextTick + PushForward<P, B>,
{
    type Out = Ctx<B, A, S, T::Out>;

    fn seal_forward(self) -> Self::Out {
        Ctx::new(PushForward::push_forward(self.tick))
    }
}

/// Config: push the forwarded children `R` onto the tick, left to right.
/// Generics: `R` the children: `P::T` or `P::MUT`, or several joined with `&`.
#[doc(hidden)]
pub struct SetForward<R> {
    _markers: PhantomData<R>,
}

// Generics: `C` context, `R` children.
impl<C, R> CtxConfig<C> for SetForward<R>
where
    C: Context,
    R: Forwards<C>,
{
    type Out = R::Out;
    type ScopeOf<O> = O;

    fn apply(self, ctx: C) -> Self::Out {
        R::push_all(ctx)
    }
}

/// Next pipeline after `.forward`.
/// Generics: `R` the children's markers.
pub trait ForwardStep<R> {
    /// Pipeline with the children stored under the current filter.
    type Out;
    /// Stores the children.
    fn forward(self, children: R) -> Self::Out;
}

// Generics: `E` event, `C` context, `R` children.
impl<E, C, R> ForwardStep<R> for Init<E, C>
where
    Self: ApplyPatch<SetForward<R>>,
{
    type Out = <Self as ApplyPatch<SetForward<R>>>::Out;

    fn forward(self, _children: R) -> Self::Out {
        self.apply_patch(SetForward { _markers: PhantomData })
    }
}

// Generics: `E` event, `C` context.
impl<E, C> Init<E, C> {
    /// Sends the events of the current filter on to beans this module
    /// provides: `children` is `P::T` or `P::MUT`, or several joined with
    /// `&` (`P::T & Q::MUT`). The marker is the marker of the provide:
    /// `P::T` names a child provided under `Get`, which changes nothing of
    /// its own, `P::MUT` a child provided under `Mut`.
    ///
    /// The pipeline of each child (`Setup`) runs for the event the module
    /// was emitted with, in the order written, after the systems placed
    /// before this step.
    ///
    /// `Tick::init` runs the init system of each child once, in the order
    /// the `.forward` steps first name them.
    ///
    /// A child injects only beans of this module. A child is not reached
    /// without this step.
    #[allow(clippy::needless_pass_by_value)]
    pub fn forward<R>(self, children: R) -> <Self as ForwardStep<R>>::Out
    where
        Self: ForwardStep<R>,
    {
        <Self as ForwardStep<R>>::forward(self, children)
    }
}
