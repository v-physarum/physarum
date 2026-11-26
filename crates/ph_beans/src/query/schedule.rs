//! Runs the map handlers kept by the emit const, then the deferred events.
//!
//! [`Drive`] asks each map `F::Against::<M>::MATCHES` for the event passed to
//! [`Tick::emit`](crate::Tick::emit). A miss is deleted, a hit is a direct call
//! (PERF-1, `docs/performance.md`). Then the buffer of the event runs, in
//! defer order: each entry is one walk ([`Queued`]) whose only run-time check
//! is the entry's path against the map's filter.
//! Path gates use the same const ([`crate::Gate::path_plan`]).

use crate::context::list::{Hand, Pass};
use crate::context::maps::MapList;
use crate::context::stages::Makes;
use crate::context::{Owned, Pipe, Sys};
use crate::descriptor::ModuleDesc;
use crate::filter::eval::Collect;
use crate::filter::shape::NestOk;
use crate::query::args::ClaimShape;
use crate::query::boot::InitSys;
use crate::query::deferred::{DeferInto, Drain, Queued};
use crate::query::lend::Beans;
use crate::query::sees::Sees;
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

    /// Runs walk `W` over the provided `states` of one instance: the maps
    /// the walk keeps, then, after a direct emit, the maps of the events
    /// deferred into its scope (run-time bits). Each map borrows from
    /// `states` and `injects`.
    ///
    /// Generics: `W` walk ([`Pass`]).
    fn pass<W: Pass>(
        &self,
        states: &mut B::States,
        injects: <B::Injects as ClaimShape>::Lent<'_>,
        walk: W,
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
    H: InitSys<B, A>,
    for<'s, 'x> A: Sees<B, <B as Beans<'s, 'x>>::Full>,
    N: for<'s, 'x> Hand<<B as Beans<'s, 'x>>::Full>,
    N: for<'s, 'x> Makes<<B as Beans<'s, 'x>>::Full>,
{
    const DESC: ModuleDesc = ModuleDesc::new(E::MASK);

    // The provided states start at their defaults. Each forwarded child
    // then runs its own init system once, in the order the `.forward`
    // steps first name them. The module's init function runs last: it
    // sees what the children made.
    fn boot(&self, injects: <B::Injects as ClaimShape>::Lent<'_>) -> Owned<B::State, B::States> {
        let mut states = B::States::default();
        let state = {
            let mut full = B::full(&mut states, injects);
            self.list.entries.make_all(&mut full);
            self.init.handler.start(&mut full)
        };
        Owned::new(state, states)
    }

    fn pass<W: Pass>(
        &self,
        states: &mut B::States,
        injects: <B::Injects as ClaimShape>::Lent<'_>,
        walk: W,
    ) {
        let mut full = B::full(states, injects);
        self.list.entries.hand(walk, &mut full);
        // The descriptor answers at build time: with no declared event to
        // defer into the emit, no buffer is read.
        if const { W::DRAINS && Self::DESC.reads_queue::<W::Emit>() } {
            E::drain::<W::Emit, _>(&mut full, &mut |full, path| {
                self.list.entries.hand(Queued::<W::Emit>::new(path), full);
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
