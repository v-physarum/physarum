//! `setup` returns a pipeline whose status is ready.

use crate::bean::BeanInfo;
use crate::descriptor::Descriptor;
use crate::descriptor::ModuleDesc;
use crate::descriptor::PipelineOk;
use crate::filter::eval::Collect;
use crate::filter::{HostedBy, NestOk};
use crate::query::args::ClaimShape;
use crate::query::deferred::DeferInto;
use crate::query::update::Update;

/// Finished pipeline for module `M`. Needs [`PipelineOk`] status and [`Update`].
///
/// Generics: `M` module ([`BeanInfo`]).
pub trait Tick<M: BeanInfo>: Descriptor + Update<M>
where
    M::Injects: ClaimShape,
{
    /// The descriptor of the module's pipeline: what it declares with `.emit`,
    /// whether an emit reads the defer queue. The pipeline type stays hidden
    /// behind `impl Tick<M>`; this constant is what a caller can ask at build
    /// time.
    const DESC: ModuleDesc = <Self as Update<M>>::DESC;

    /// Makes one instance: the provided states, then the init system once over
    /// its claim. `injects` are the states the caller lends (`()` with none).
    fn init(&self, injects: <M::Injects as ClaimShape>::Lent<'_>) -> Self::Instance {
        Update::init(self, injects)
    }

    /// The module state of `instance`: what the init function returned.
    ///
    /// `impl Tick<M>` hides the instance type, so the state is read through
    /// the tick.
    fn state<'i>(&self, instance: &'i Self::Instance) -> &'i M::State {
        Update::state(self, instance)
    }

    /// Runs maps for emit tree `N` over `instance`.
    ///
    /// Each map borrows its claim from the provided states of `instance` and
    /// from `injects`.
    fn emit<N: Collect + crate::ScopeHost + HostedBy<M>>(
        &self,
        event: N,
        instance: &mut Self::Instance,
        injects: <M::Injects as ClaimShape>::Lent<'_>,
    ) {
        Update::emit(self, event, instance, injects);
    }

    /// Stores `N` on its scope in `instance`. The next enter of that scope runs it.
    fn defer<N: Collect + NestOk + HostedBy<M> + DeferInto<M>>(
        &self,
        event: N,
        instance: &mut Self::Instance,
    ) {
        Update::defer(self, event, instance);
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
