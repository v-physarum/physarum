//! Public runtime calls: [`Tick::init`](crate::Tick::init),
//! [`Tick::emit`](crate::Tick::emit), [`Tick::defer`](crate::Tick::defer).

use crate::bean::BeanInfo;
use crate::context::Context;
use crate::context::Owned;
use crate::context::ScopeHost;
use crate::context::list::{ConstEmit, Pass};
use crate::descriptor::ModuleDesc;
use crate::filter::HostedBy;
use crate::filter::NestOk;
use crate::filter::eval::Collect;
use crate::query::args::ClaimShape;
use crate::query::built::Built;
use crate::query::deferred::DeferInto;
use crate::query::schedule::Drive;
use crate::query::store::Stored;

/// Runs one module: makes an instance, then emits into it.
///
/// The pipeline holds only systems. The instance holds the state, the
/// provided states, and the defer queue. Injects are not owned by the
/// module: the caller passes them to `init` and to every `emit`.
/// Generics: `M` module ([`BeanInfo`]).
pub trait Update<M: BeanInfo>: Sized
where
    M::Injects: ClaimShape,
{
    /// What the pipeline does. See [`Tick::DESC`](crate::Tick::DESC).
    const DESC: ModuleDesc;
    /// One instance of `M`.
    type Instance;
    /// Makes an instance: the provided states, then the init system once.
    fn init(&self, injects: <M::Injects as ClaimShape>::Lent<'_>) -> Self::Instance;
    /// The module state of `instance`.
    fn state<'i>(&self, instance: &'i Self::Instance) -> &'i M::State;
    /// Runs maps whose filters match `N` over `instance`. Generics: `N` emit tree ([`Collect`]).
    fn emit<N: Collect + ScopeHost + HostedBy<M>>(
        &self,
        event: N,
        instance: &mut Self::Instance,
        injects: <M::Injects as ClaimShape>::Lent<'_>,
    );
    /// Stores `N` on its scope in `instance`. The next enter of that scope runs it.
    fn defer<N: Collect + NestOk + HostedBy<M> + DeferInto<M>>(
        &self,
        event: N,
        instance: &mut Self::Instance,
    );
    /// The parts of a new instance. A parent keeps them in its own instance
    /// for a bean it provides.
    #[doc(hidden)]
    fn boot_parts(
        &self,
        injects: <M::Injects as ClaimShape>::Lent<'_>,
    ) -> Owned<M::State, <M as Stored>::States>
    where
        M: Stored;
    /// Runs walk `W` over the provided states a parent keeps for `M`.
    #[doc(hidden)]
    fn drive_parts<W: Pass>(
        &self,
        states: &mut <M as Stored>::States,
        injects: <M::Injects as ClaimShape>::Lent<'_>,
        walk: W,
    ) where
        M: Stored;
}

// Generics: `E` builder event scope, `C` context, `M` module.
// `C::Bean = M`: a pipeline built for one bean is not a tick of another.
impl<E, C, M> Update<M> for Built<E, C>
where
    C: Context<Bean = M>,
    C::Tick: Drive<M>,
    M: Stored,
    M::Injects: ClaimShape,
{
    const DESC: ModuleDesc = <C::Tick as Drive<M>>::DESC;
    type Instance = Owned<M::State, M::States>;

    fn init(&self, injects: <M::Injects as ClaimShape>::Lent<'_>) -> Self::Instance {
        self.tick().boot(injects)
    }

    fn state<'i>(&self, instance: &'i Self::Instance) -> &'i M::State {
        instance.state()
    }

    fn emit<N: Collect + ScopeHost + HostedBy<M>>(
        &self,
        event: N,
        instance: &mut Self::Instance,
        injects: <M::Injects as ClaimShape>::Lent<'_>,
    ) {
        let _ = event;
        self.tick()
            .pass(&mut instance.provides, injects, ConstEmit::<N>::new());
    }

    fn defer<N: Collect + NestOk + HostedBy<M> + DeferInto<M>>(
        &self,
        event: N,
        instance: &mut Self::Instance,
    ) {
        let _ = event;
        self.tick().defer::<N>(instance);
    }

    fn boot_parts(
        &self,
        injects: <M::Injects as ClaimShape>::Lent<'_>,
    ) -> Owned<M::State, M::States> {
        self.tick().boot(injects)
    }

    fn drive_parts<W: Pass>(
        &self,
        states: &mut M::States,
        injects: <M::Injects as ClaimShape>::Lent<'_>,
        walk: W,
    ) {
        self.tick().pass(states, injects, walk);
    }
}
