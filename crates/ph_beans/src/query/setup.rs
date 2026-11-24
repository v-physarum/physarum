//! `setup` returns a pipeline whose status is ready.

use crate::bean::BeanInfo;
use crate::descriptor::Descriptor;
use crate::descriptor::PipelineOk;
use crate::filter::eval::Collect;
use crate::query::args::ClaimShape;
use crate::query::update::Update;

/// Finished pipeline for module `M`. Needs [`PipelineOk`] status and [`Update`].
///
/// Generics: `M` module ([`BeanInfo`]).
pub trait Tick<M: BeanInfo>: Descriptor + Update<M>
where
    M::Injects: ClaimShape,
{
    /// Runs maps for emit tree `N`.
    ///
    /// TODO: the init function is stored and not called yet, and `injects`
    /// is not passed to the maps yet.
    fn emit<N: Collect + crate::ScopeHost>(
        &mut self,
        event: N,
        injects: <M::Injects as ClaimShape>::View<'_>,
    ) {
        Update::emit(self, event, injects);
    }

    /// Stores `N` on its scope. The next enter of that scope runs it.
    fn defer<N: Collect + crate::filter::NestOk + crate::ScopeHost>(&mut self, event: N) {
        Update::defer(self, event);
    }
}

// Generics: `M` mask / match, `T` tick / type / tip.
impl<M, T> Tick<M> for T
where
    M: BeanInfo,
    M::Injects: ClaimShape,
    T: Descriptor + Update<M> + Sized,
    T::Status: PipelineOk<T>,
{
}

/// Builds the pipeline for a bean.
pub trait Setup: BeanInfo + Sized
where
    Self::Injects: ClaimShape,
{
    /// Resources, init, then `.build()`. Needs [`PipelineOk`] for [`Tick`].
    /// ```compile_fail
    /// use ph_beans::{BeanInfo, Query, Setup, Tick};
    /// struct App; impl BeanInfo for App { type Provides=(); type Injects=(); type Beans=(); type State=u8; }
    /// fn boot(_: ()) -> u8 { 1 }
    /// impl Setup for App { fn setup(self) -> impl Tick<Self> { App::query().init_state(boot) } }
    /// ```
    /// A pipeline built for another bean is rejected.
    /// ```compile_fail
    /// use ph_beans::{BeanInfo, Query, Setup, Tick};
    /// struct App; impl BeanInfo for App { type Provides=(); type Injects=(); type Beans=(); type State=u8; }
    /// struct Other; impl BeanInfo for Other { type Provides=(); type Injects=(); type Beans=(); type State=u8; }
    /// fn boot(_: ()) -> u8 { 1 }
    /// impl Setup for Other { fn setup(self) -> impl Tick<Self> { App::query().init_state(boot).build() } }
    /// ```
    fn setup(self) -> impl Tick<Self>;
}
