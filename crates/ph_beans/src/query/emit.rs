//! `.emit` records which events this pipeline activates.

use core::marker::PhantomData;

use crate::bean::BeanInfo;
use crate::bean::EventInfo;
use crate::context::Context;
use crate::context::ContextTick;
use crate::context::Ctx;
use crate::context::CtxConfig;
use crate::context::Pipe;
use crate::context::maps::MapList;
use crate::descriptor::live::AccSet;
use crate::filter::NestOk;
use crate::filter::On;
use crate::filter::eval::UnionEmit;
use crate::filter::shape::ParentsOk;
use crate::query::apply::ApplyPatch;
use crate::query::init::Init;

/// Unions `T` into the emit parameter. Hides `EmitOf` + `UnionEmit` + `Reemit`.
pub trait EmitUnion<T>: Sized {
    /// Tick after the union.
    type Out: ContextTick;
    /// Stores the unioned tree.
    fn union_emit(self) -> Self::Out;
}

impl<T, I, N, E, S> EmitUnion<T> for Pipe<I, MapList<N, E, S>>
where
    I: ContextTick,
    E: UnionEmit<T>,
{
    type Out = Pipe<I, MapList<N, E::Out, S>>;

    fn union_emit(self) -> Self::Out {
        Pipe::new(self.init, self.list.reemit())
    }
}

/// The `On` node for this bean's provider event.
#[doc(hidden)]
pub trait ProviderEvent {
    /// `On<Provides, ()>`.
    type Tree: NestOk;
}

impl<B> ProviderEvent for B
where
    B: BeanInfo<Provides: EventInfo>,
    On<B::Provides, ()>: NestOk,
{
    type Tree = On<B::Provides, ()>;
}

/// Unions an emit tree on a concrete [`Ctx`] (hides `B`/`A`/`S`/`T` from [`CtxConfig`]).
#[doc(hidden)]
pub trait SealEmit<E>: Sized {
    /// Context after the seal.
    type Out: Context;
    /// Stores the unioned emit tree `E`.
    fn seal_emit(self) -> Self::Out;
}

impl<B, A, S, T, E> SealEmit<E> for Ctx<B, A, S, T>
where
    A: AccSet,
    T: ContextTick + EmitUnion<E>,
{
    type Out = Ctx<B, A, S, T::Out>;

    fn seal_emit(self) -> Self::Out {
        Ctx::new(EmitUnion::<E>::union_emit(self.tick))
    }
}

/// Config: union `E` into the emit tree.
#[doc(hidden)]
pub struct SetEmit<E>(PhantomData<E>);

impl<C, E> CtxConfig<C> for SetEmit<E>
where
    C: Context + SealEmit<E>,
{
    type Out = C::Out;
    type ScopeOf<O> = O;

    fn apply(self, ctx: C) -> Self::Out {
        let _ = self;
        ctx.seal_emit()
    }
}

/// Emits the provider event on a concrete [`Ctx`] (hides `B`/`A`/`S`/`T` from [`CtxConfig`]).
#[doc(hidden)]
pub trait SealProviders: Sized {
    /// Context after the seal.
    type Out: Context;
    /// Stores the module's provider event.
    fn seal_providers(self) -> Self::Out;
}

impl<B, A, S, T> SealProviders for Ctx<B, A, S, T>
where
    A: AccSet,
    B: ProviderEvent,
    T: ContextTick + EmitUnion<B::Tree>,
{
    type Out = Ctx<B, A, S, T::Out>;

    fn seal_providers(self) -> Self::Out {
        Ctx::new(EmitUnion::<B::Tree>::union_emit(self.tick))
    }
}

/// Config: emit the module's provider event.
#[doc(hidden)]
pub struct SetProviders;

impl<C> CtxConfig<C> for SetProviders
where
    C: Context + SealProviders,
{
    type Out = C::Out;
    type ScopeOf<O> = O;

    fn apply(self, ctx: C) -> Self::Out {
        ctx.seal_providers()
    }
}

/// Next pipeline after `.emit`.
///
/// `K` is the inferred parent path. Call sites do not name it. A proxy's
/// parents satisfy the check without replacing [`EventInfo::Parents`](crate::EventInfo::Parents).
pub trait EmitStep<T, K> {
    /// Pipeline with `T` unioned into the emit tree.
    type Out;
    /// Records `tree`.
    fn emit(self, tree: T) -> Self::Out;
}

impl<E, C, T, K> EmitStep<T, K> for Init<E, C>
where
    Self: ApplyPatch<SetEmit<T>>,
    T: ParentsOk<K>,
{
    type Out = <Self as ApplyPatch<SetEmit<T>>>::Out;

    fn emit(self, tree: T) -> Self::Out {
        let _ = tree;
        self.apply_patch(SetEmit(PhantomData))
    }
}

/// Next pipeline after `.emit_providers`.
pub trait EmitProvidersStep {
    /// Pipeline that emits the module's provider event.
    type Out;
    /// Records that event.
    fn emit_providers(self) -> Self::Out;
}

impl<E, C> EmitProvidersStep for Init<E, C>
where
    Self: ApplyPatch<SetProviders>,
{
    type Out = <Self as ApplyPatch<SetProviders>>::Out;

    fn emit_providers(self) -> Self::Out {
        self.apply_patch(SetProviders)
    }
}

impl<E, C> Init<E, C> {
    /// Unions `tree` into the emit set.
    ///
    /// A nested event must list its outer event in [`EventInfo::Parents`].
    #[allow(clippy::needless_pass_by_value)]
    pub fn emit<T, K>(self, tree: T) -> <Self as EmitStep<T, K>>::Out
    where
        Self: EmitStep<T, K>,
    {
        EmitStep::emit(self, tree)
    }

    /// Emits the module's provider event.
    pub fn emit_providers(self) -> <Self as EmitProvidersStep>::Out
    where
        Self: EmitProvidersStep,
    {
        EmitProvidersStep::emit_providers(self)
    }
}
