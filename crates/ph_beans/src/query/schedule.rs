//! Runs the map handlers kept by the emit const, plus the deferred mask.
//!
//! [`Drive`] asks each map `F::Against::<Ev>::MATCHES` for the event passed to
//! [`Tick::emit`](crate::Tick::emit). A miss is deleted, a hit is a direct call.
//! The deferred buffer is a runtime [`MatchEmit::matches`] walk before that.
//! A direct child uses `const { F::MASK == P::MASK }`.
//! Path gates use the same const ([`crate::Gate::path_plan`]).

use crate::context::Pipe;
use crate::context::direct::{Direct, DirectFn, TipEvent};
use crate::context::list::{ExactDrive, Lane, Walk, WalkStrategy};
use crate::context::maps::{MapAt, MapEntry, MapList};
use crate::filter::eval::{Collect, EmitMask, MatchEmit};

// Generics: `L` list, `F` filter path, `H` handler.
impl<L, F, H> Walk<L> for MapEntry<F, H>
where
    F: MatchEmit + Collect,
    H: Fn(),
{
    fn walk<S: WalkStrategy + Copy>(&self, strategy: S, lane: &Lane<L>)
    where
        L: ExactDrive,
    {
        let _ = lane;
        if strategy.keep::<F>() {
            (self.handler)();
        }
    }
}

// Generics: `L` list, `F` filter path, `H` handler ([`DirectFn`]).
impl<L, F, H> Walk<L> for MapAt<F, H>
where
    F: MatchEmit + Collect + TipEvent,
    H: DirectFn<F::Tip, F>,
{
    fn walk<S: WalkStrategy + Copy>(&self, strategy: S, lane: &Lane<L>)
    where
        L: ExactDrive,
    {
        if strategy.enter_at::<F>() {
            self.handler.run(&Direct::from_lane(lane));
        }
    }
}

// Generics: `N` entries, `E` emit tree, `S` filter scope; `G` walk strategy.
impl<N, E, S> MapList<N, E, S> {
    pub(crate) fn drive<G: WalkStrategy + Copy>(&self, strategy: G)
    where
        N: Walk<N> + ExactDrive,
    {
        let lane = Lane { root: &self.entries, scopes: &self.scopes };
        self.entries.walk(strategy, &lane);
    }

    pub(crate) fn defer_into(&self, bit: u32, mask: EmitMask) {
        self.scopes.or_bit(bit, mask);
    }

    pub(crate) fn take_scope(&self, bit: u32) -> EmitMask {
        self.scopes.take_bit(bit)
    }
}

/// Runs the handlers kept by the event passed to [`Tick::emit`](crate::Tick::emit).
pub(crate) trait Drive {
    /// Calls the handlers `strategy` keeps.
    ///
    /// Generics: `G` walk strategy ([`WalkStrategy`]).
    fn drive<G: WalkStrategy + Copy>(&self, strategy: G);
    /// Ors `mask` into the scope event's buffer.
    fn defer_into(&self, bit: u32, mask: EmitMask);
    /// Takes one scope buffer and leaves it empty.
    fn take_scope(&self, bit: u32) -> EmitMask;
}

impl<I, N, E, S> Drive for Pipe<I, MapList<N, E, S>>
where
    N: Walk<N> + ExactDrive,
{
    fn drive<G: WalkStrategy + Copy>(&self, strategy: G) {
        self.list.drive(strategy);
    }

    fn defer_into(&self, bit: u32, mask: EmitMask) {
        self.list.defer_into(bit, mask);
    }

    fn take_scope(&self, bit: u32) -> EmitMask {
        self.list.take_scope(bit)
    }
}
