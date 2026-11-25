//! `.emit` records which events this pipeline activates.

use core::marker::PhantomData;

use crate::context::Context;
use crate::context::ContextTick;
use crate::context::Ctx;
use crate::context::CtxConfig;
use crate::context::Pipe;
use crate::context::maps::MapList;
use crate::descriptor::live::AccSet;
use crate::filter::HostedBy;
use crate::filter::NestOk;
use crate::filter::eval::UnionEmit;
use crate::query::apply::ApplyPatch;
use crate::query::init::Init;

/// Unions `T` into the emit parameter. Hides `EmitOf` + `UnionEmit` + `Reemit`.
/// Generics: `T` emit tree.
pub trait EmitUnion<T>: Sized {
    /// Tick after the union.
    type Out: ContextTick;
    /// Stores the unioned tree.
    fn union_emit(self) -> Self::Out;
}

// Generics: `T` emit tree, `I` inner path, `N` new entry, `E` event tip, `S` strategy.
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

/// Unions an emit tree on a concrete [`Ctx`] (hides `B`/`A`/`S`/`T` from [`CtxConfig`]).
/// Generics: `E` event tip.
#[doc(hidden)]
pub trait SealEmit<E>: Sized {
    /// Context after the seal.
    type Out: Context;
    /// Stores the unioned emit tree `E`.
    fn seal_emit(self) -> Self::Out;
}

// Generics: `B` module, `A` left child, `S` strategy, `T` emit tree, `E` event tip.
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
/// Generics: `E` event tip.
#[doc(hidden)]
pub struct SetEmit<E> {
    _markers: PhantomData<E>,
}

// Generics: `C` context, `E` event tip.
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

/// Next pipeline after `.emit`.
/// Generics: `T` emit tree.
pub trait EmitStep<T> {
    /// Pipeline with `T` unioned into the emit tree.
    type Out;
    /// Records `tree`.
    fn emit(self, tree: T) -> Self::Out;
}

// Generics: `E` event tip, `C` context, `T` emit tree.
impl<E, C, T> EmitStep<T> for Init<E, C>
where
    C: Context,
    Self: ApplyPatch<SetEmit<T>>,
    T: NestOk + HostedBy<C::Bean>,
{
    type Out = <Self as ApplyPatch<SetEmit<T>>>::Out;

    fn emit(self, tree: T) -> Self::Out {
        let _ = tree;
        self.apply_patch(SetEmit { _markers: PhantomData })
    }
}

// Generics: `E` event tip, `C` context.
impl<E, C> Init<E, C> {
    /// Unions `tree` into the emit set.
    ///
    /// A nested event must list its outer event in [`EventInfo::Parents`](crate::EventInfo::Parents).
    #[allow(clippy::needless_pass_by_value)]
    pub fn emit<T>(self, tree: T) -> <Self as EmitStep<T>>::Out
    where
        Self: EmitStep<T>,
    {
        EmitStep::emit(self, tree)
    }
}
