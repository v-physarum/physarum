//! Runs the map handlers kept by the emit const, then the deferred events.
//!
//! [`Drive`] asks each map `F::Against::<M>::MATCHES` for the event passed to
//! [`Tick::emit`](crate::Tick::emit). A miss is deleted, a hit is a direct call
//! (PERF-1, `docs/performance.md`). Then the buffer of the event runs, in
//! defer order: each entry is one walk ([`Queued`]) whose only run-time check
//! is the entry's path against the map's filter.
//! Path gates use the same const ([`crate::Gate::path_plan`]).

use core::marker::PhantomData;

use crate::context::ScopeHost;
use crate::context::list::{ConstEmit, Hand};
use crate::context::maps::MapList;
use crate::context::{Owned, Pipe, Sys};
use crate::descriptor::ModuleDesc;
use crate::filter::eval::Collect;
use crate::filter::shape::NestOk;
use crate::query::args::ClaimShape;
use crate::query::boot::InitSys;
use crate::query::deferred::{DeferInto, Drain, Queued};
use crate::query::lend::Beans;
use crate::query::store::Stored;

/// The systems of module `B`, run over one instance at a time.
///
/// Generics: `B` module.
pub(crate) trait Drive<B>
where
    B: Stored<Injects: ClaimShape>,
{
    /// What this pipeline does, for [`Tick::DESC`](crate::Tick::DESC).
    const DESC: ModuleDesc;

    /// Makes one instance: provided states, then the init system once.
    fn boot(&self, injects: <B::Injects as ClaimShape>::Lent<'_>) -> Owned<B::State, B::States>;

    /// Runs emit `M` over `owned`: first the maps `M` matches (const), then
    /// the maps of events deferred into `M`'s scope (run-time bits). Each map
    /// borrows from the provided states of `owned` and from `injects`.
    ///
    /// Generics: `M` emit tree ([`Collect`]).
    fn drive<M: Collect + ScopeHost>(
        &self,
        owned: &mut Owned<B::State, B::States>,
        injects: <B::Injects as ClaimShape>::Lent<'_>,
    );

    /// Queues `M` in `owned`. The pipeline's `.emit` must declare it (`FilterError<6>`).
    fn defer<M>(&self, owned: &mut Owned<B::State, B::States>)
    where
        M: Collect + NestOk + DeferInto<B>;
}

// Generics: `B` module, `A` init claim, `H` init handler, `N` entries,
// `E` emit tree, `S` filter scope.
impl<B, A, H, N, E, S> Drive<B> for Pipe<Sys<B, A, H>, MapList<N, E, S>>
where
    E: Collect + for<'s, 'x> Drain<B, <B as Beans<'s, 'x>>::Full>,
    B: Stored<Injects: ClaimShape> + for<'s, 'x> Beans<'s, 'x>,
    for<'x> H:
        InitSys<B, A, States = B::States, Injects<'x> = <B::Injects as ClaimShape>::Lent<'x>>,
    N: for<'s, 'x> Hand<<B as Beans<'s, 'x>>::Full>,
{
    const DESC: ModuleDesc = ModuleDesc::new(E::MASK);

    fn boot(&self, injects: <B::Injects as ClaimShape>::Lent<'_>) -> Owned<B::State, B::States> {
        self.init.handler.boot(injects)
    }

    fn drive<M: Collect + ScopeHost>(
        &self,
        owned: &mut Owned<B::State, B::States>,
        injects: <B::Injects as ClaimShape>::Lent<'_>,
    ) {
        let mut full = B::full(&mut owned.provides, injects);
        self.list
            .entries
            .hand(ConstEmit::<M> { _markers: PhantomData }, &mut full);
        // The descriptor answers at build time: with no declared event to
        // defer into `M`, no buffer is read.
        if const { Self::DESC.reads_queue::<M>() } {
            E::drain::<M, _>(&mut full, &mut |full, path| {
                self.list.entries.hand(Queued::<M>::new(path), full);
            });
        }
    }

    fn defer<M>(&self, owned: &mut Owned<B::State, B::States>)
    where
        M: Collect + NestOk + DeferInto<B>,
    {
        let _ = self;
        const {
            assert!(
                Self::DESC.declares::<M>(),
                "pipeline not ready: Descriptor::Status = `FilterError<6>`: a deferred event \
                 must be declared by `.emit` of this pipeline; FilterError<6> — docs/errors.md"
            );
        }
        M::defer_into(&mut owned.provides, M::MASK);
    }
}
