//! `setup` returns a pipeline whose status is ready.

use crate::bean::BeanInfo;
use crate::descriptor::Descriptor;
use crate::descriptor::PipelineOk;
use crate::query::args::ArgsView;
use crate::query::update::Update;

/// A finished pipeline for bean `M`.
///
/// Implemented for every descriptor whose [`Descriptor::Status`] is
/// [`PipelineOk`](crate::PipelineOk) and that can [`Tick::update`]. A missing
/// `.build()` leaves [`NeedBuild`](crate::NeedBuild), so `impl Tick` fails
/// on `setup`.
pub trait Tick<M: BeanInfo>: Descriptor + Update<M>
where
    M::Injects: ArgsView,
{
    /// Runs this module on its inject references.
    ///
    /// Init already ran in the builder. This call does not initialize state.
    fn update(&mut self, injects: <M::Injects as ArgsView>::View<'_>) {
        Update::update(self, injects);
    }
}

impl<M, T> Tick<M> for T
where
    M: BeanInfo,
    M::Injects: ArgsView,
    T: Descriptor + Update<M> + Sized,
    T::Status: PipelineOk<T>,
{
}

/// Builds the pipeline for a bean.
pub trait Setup: BeanInfo + Sized
where
    Self::Injects: ArgsView,
{
    /// Resources, init, then `.build()`.
    ///
    /// Returns a [`Tick`] only when status is [`PipelineOk`](crate::PipelineOk).
    /// A missing `.build()` fails. See `setup_without_build_is_rejected`.
    fn setup(self) -> impl Tick<Self>;
}
