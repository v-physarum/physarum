//! Runs the map handlers whose filters match the emit set.

use core::any::TypeId;

use crate::context::Pipe;
use crate::context::list::{List, Walk};
use crate::context::maps::{MAX_MAP_SYSTEMS, MapEntry, MapList};
use crate::filter::eval::{Collect, MatchEmit};

#[doc(hidden)]
pub trait Run {
    /// Calls matching handlers. `emitted` is the emit tree for this tick.
    fn run(&mut self, emitted: &[TypeId]);
}

impl<F, H> Walk for MapEntry<F, H>
where
    F: MatchEmit,
    H: Fn(),
{
    fn walk(&self, emitted: &[TypeId]) {
        if F::matches(emitted) {
            (self.handler)();
        }
    }
}

impl<I> Run for List<I>
where
    I: Walk,
{
    fn run(&mut self, emitted: &[TypeId]) {
        self.0.walk(emitted);
    }
}

impl<N, E, S> MapList<N, E, S>
where
    N: Run,
    E: Collect,
{
    pub(crate) fn drive(&mut self) {
        let mut buf = [TypeId::of::<()>(); MAX_MAP_SYSTEMS];
        let count = E::write(&mut buf, 0);
        self.entries.run(&buf[..count]);
    }
}

/// Runs every stored map against the emit tree.
#[doc(hidden)]
pub trait Drive {
    /// Calls the handlers whose filters match.
    fn drive(&mut self);
}

impl<I, N, E, S> Drive for Pipe<I, MapList<N, E, S>>
where
    N: Run,
    E: Collect,
{
    fn drive(&mut self) {
        self.list.drive();
    }
}
