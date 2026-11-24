//! Linear context: bean, args, status, and systems.

use core::marker::PhantomData;

use crate::descriptor::Descriptor;
use crate::descriptor::live::AccSet;

pub(crate) mod direct;
mod event_buf;
pub(crate) mod list;
pub mod maps;
mod patch;
mod push;
pub(crate) mod scope;
mod start;
pub mod tick;

pub use direct::{ChildOf, Direct, DirectFn, PushInner, WithChild};
pub use event_buf::EventBuf;
pub use list::{CarrierDesc, ExactDrive, Grow, Leaf, List};
pub use maps::{MAX_MAP_SYSTEMS, MapEntry, MapList};
pub use patch::CtxConfig;
#[doc(hidden)]
pub use scope::{ScopeBufs, ScopeHost};
pub use start::StartDesc;
pub use tick::{ContextTick, DefaultState, FreshInit, NoSystems, Pipe, WithDefault, WithInit};

/// Bean, resource args, pipeline status, and the systems.
///
/// Operators stay flat. A step replaces fields through one config and
/// [`With`](Context::With) instead of nesting another operator inside the context.
/// Generics: `B` bean, `A` args, `S` status, `T` tick.
pub struct Ctx<B, A, S, T> {
    pub(crate) tick: T,
    _markers: PhantomData<(B, S, A)>,
}

impl<B, A, S, T> Ctx<B, A, S, T> {
    pub(crate) fn new(tick: T) -> Self {
        Self { tick, _markers: PhantomData }
    }
}

impl<B, A, S, T> Descriptor for Ctx<B, A, S, T>
where
    A: AccSet,
{
    type Status = S;
    type Accessors = A::Set;
}

/// The context half of a descriptor: which bean, which args, which systems.
///
/// The next tip is always [`With`](Context::With)`<G>` for a [`CtxConfig`].
/// There is no separate `WithArgs` / `WithStatus` / `WithTick` GAT.
pub trait Context: Descriptor + Sized {
    /// Bean the pipeline builds.
    type Bean;
    /// Resource claims recorded by `.inject_resources`.
    type Args;
    /// Systems stored for this pipeline.
    type Tick: ContextTick;
    /// Context after `G` — [`CtxConfig::Out`].
    type With<G: CtxConfig<Self>>: Context;

    /// Borrows the systems.
    fn tick(&self) -> &Self::Tick;
    /// Borrows the systems exclusively.
    fn tick_mut(&mut self) -> &mut Self::Tick;
    /// Applies `cfg` and yields [`With`](Context::With).
    fn configure<G: CtxConfig<Self>>(self, cfg: G) -> Self::With<G>;
}

// Generics: `B` right / bean, `A` left / args, `S` status / state / strategy, `T` tick / type / tip.
impl<B, A, S, T> Context for Ctx<B, A, S, T>
where
    A: AccSet,
    T: ContextTick,
{
    type Bean = B;
    type Args = A;
    type Tick = T;
    type With<G: CtxConfig<Self>> = G::Out;

    fn tick(&self) -> &Self::Tick {
        &self.tick
    }

    fn tick_mut(&mut self) -> &mut Self::Tick {
        &mut self.tick
    }

    fn configure<G: CtxConfig<Self>>(self, cfg: G) -> Self::With<G> {
        cfg.apply(self)
    }
}
