//! Linear context: bean, args, status, and systems.

use core::marker::PhantomData;

use crate::core::config::OfConfig;
use crate::descriptor::Descriptor;
use crate::descriptor::live::AccSet;

mod carrier;
mod event_buf;
mod hand_tuples;
pub(crate) mod list;
pub mod maps;
mod patch;
pub(crate) mod scope;
pub(crate) mod stages;
mod start;
mod strategy;
pub(crate) mod sys;
pub mod tick;

pub use event_buf::{DeferState, EventBuf};
pub use list::{CarrierDesc, End, Full, Group, Head, List};
pub use maps::{MAX_MAP_SYSTEMS, MapEntry, MapList};
pub use patch::{ApplyCfg, CtxConfig};
#[doc(hidden)]
pub use scope::ScopeHost;
pub use start::StartDesc;
pub use sys::{MapSys, Sys};
pub use tick::{ContextTick, FreshInit, NoSystems, Owned, Pipe};

/// Bean, resource args, pipeline status, and the systems.
///
/// Operators stay flat. A step replaces fields through one config and
/// [`With`](Context::With) instead of nesting another operator inside the context.
/// Generics: `B` bean, `A` args, `S` status, `T` tick.
pub struct Ctx<B, A, S, T> {
    pub(crate) tick: T,
    _markers: PhantomData<(B, S, A)>,
}

// Generics: `B` bean, `A` args, `S` status, `T` tick.
impl<B, A, S, T> Ctx<B, A, S, T> {
    pub(crate) fn new(tick: T) -> Self {
        Self { tick, _markers: PhantomData }
    }
}

// Generics: `B` bean, `A` args, `S` status, `T` tick.
impl<B, A, S, T> Descriptor for Ctx<B, A, S, T>
where
    A: AccSet,
{
    type Status = S;
    type Accessors = A::Set;
}

/// The context half of a descriptor: which bean, which args, which systems.
///
/// The next tip is [`OfConfig`]`<`[`ApplyCfg`]`<G, Self>>`.
/// There is no separate `WithArgs` / `WithStatus` / `WithTick` GAT.
pub trait Context: Descriptor + Sized {
    /// Bean the pipeline builds.
    type Bean;
    /// Resource claims recorded by `.inject_resources`.
    type Args;
    /// Systems stored for this pipeline.
    type Tick: ContextTick;
    /// Context after `G` — [`OfConfig`]`<`[`ApplyCfg`]`<G, Self>>`.
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
    type With<G: CtxConfig<Self>> = OfConfig<ApplyCfg<G, Self>>;

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
