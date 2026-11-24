//! Runs the map handlers kept by the emit const, plus the deferred mask.
//!
//! [`Drive`] asks each map `F::Against::<Ev>::MATCHES` for the event passed to
//! [`Tick::emit`](crate::Tick::emit). A miss is deleted, a hit is a direct call.
//! The deferred buffer is a runtime [`MatchEmit::matches`] walk before that.
//! A direct child uses `const { F::MASK == P::MASK }`.
//! Path gates use the same const ([`crate::Gate::path_plan`]).

use crate::context::Pipe;
use crate::context::list::{ExactDrive, Invoke, Lane, WalkOp, WalkStrategy};
use crate::context::maps::MapList;
use crate::filter::eval::EmitMask;

// Generics: `N` entries, `E` emit tree, `S` filter scope; `G` walk strategy.
impl<N, E, S> MapList<N, E, S> {
    pub(crate) fn drive<G: WalkStrategy + Copy>(&self, strategy: G)
    where
        N: Invoke + ExactDrive,
    {
        let lane = Lane { root: &self.entries, scopes: &self.scopes };
        self.entries.invoke(WalkOp { strategy, lane: &lane });
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
    N: Invoke + ExactDrive,
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
