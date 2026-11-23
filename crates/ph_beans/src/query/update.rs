//! Public runtime call: [`Tick::update`](crate::Tick::update).

use crate::bean::BeanInfo;
use crate::context::Context;
use crate::query::args::ArgsView;
use crate::query::built::Built;
use crate::query::schedule::Drive;

/// Runs one module on its inject references.
pub trait Update<M: BeanInfo>: Sized
where
    M::Injects: ArgsView,
{
    /// Applies `injects`. `Get` is shared and `Mut` is exclusive.
    fn update(&mut self, injects: <M::Injects as ArgsView>::View<'_>);
}

impl<E, C, M> Update<M> for Built<E, C>
where
    C: Context,
    C::Tick: Drive,
    M: BeanInfo,
    M::Injects: ArgsView,
{
    fn update(&mut self, injects: <M::Injects as ArgsView>::View<'_>) {
        let _ = injects;
        self.drive_maps();
    }
}
