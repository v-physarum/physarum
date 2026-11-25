//! `.emit_providers` records the event of a module that provides one.

use crate::bean::BeanInfo;
use crate::bean::EventInfo;
use crate::context::Context;
use crate::context::ContextTick;
use crate::context::Ctx;
use crate::context::CtxConfig;
use crate::descriptor::Provided;
use crate::descriptor::live::AccSet;
use crate::filter::NestOk;
use crate::filter::On;
use crate::query::apply::ApplyPatch;
use crate::query::emit::EmitUnion;
use crate::query::init::Init;

/// The `On` node for this bean's provider event.
#[doc(hidden)]
pub trait ProviderEvent {
    /// `On<E, ()>` for the one event in `Provides`.
    type Tree: NestOk;
}

// `Provides` is always a list. A module with one provided event names it
// as a list of one entry. Generics: `B` bean, `X` its one entry, `E` the
// provided event.
impl<B, X, E> ProviderEvent for B
where
    B: BeanInfo<Provides = (X,)>,
    X: Provided<Bean = E>,
    E: EventInfo,
    On<E, ()>: NestOk,
{
    type Tree = On<E, ()>;
}

/// Emits the provider event on a concrete [`Ctx`] (hides `B`/`A`/`S`/`T` from [`CtxConfig`]).
#[doc(hidden)]
pub trait SealProviders: Sized {
    /// Context after the seal.
    type Out: Context;
    /// Stores the module's provider event.
    fn seal_providers(self) -> Self::Out;
}

// Generics: `B` bean, `A` args, `S` status, `T` emit tree.
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

// Generics: `C` context.
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

/// Next pipeline after `.emit_providers`.
pub trait EmitProvidersStep {
    /// Pipeline that emits the module's provider event.
    type Out;
    /// Records that event.
    fn emit_providers(self) -> Self::Out;
}

// Generics: `E` event, `C` context.
impl<E, C> EmitProvidersStep for Init<E, C>
where
    Self: ApplyPatch<SetProviders>,
{
    type Out = <Self as ApplyPatch<SetProviders>>::Out;

    fn emit_providers(self) -> Self::Out {
        self.apply_patch(SetProviders)
    }
}

// Generics: `E` event, `C` context.
impl<E, C> Init<E, C> {
    /// Emits the module's provider event.
    pub fn emit_providers(self) -> <Self as EmitProvidersStep>::Out
    where
        Self: EmitProvidersStep,
    {
        EmitProvidersStep::emit_providers(self)
    }
}
