//! Map walk over the shared list in [`crate::core::list`].
//!
//! [`Hand`] is the one walk. A tuple list (`()` and lengths 1..=8) hands each
//! child on. Every walk carries `&mut R`, the resources one call lends to each
//! map in turn.

pub use crate::core::list::{EmptySlots, End, Full, Group, Head, List};

pub use crate::context::carrier::CarrierDesc;
use crate::context::maps::MapEntry;
use crate::context::sys::MapSys;
use crate::filter::eval::{Collect, ConstMatch, MatchEmit};
use crate::filter::{Gate, PathStep, PathStrategy};

/// How one walk decides which maps run.
///
/// The const emit and the queued path are structs here.
/// Tuple fan-out is one [`Hand::hand`] (rule 8) — not a method per mode.
pub(crate) trait WalkStrategy {
    /// Whether a plain map under filter `F` runs.
    fn keep<F: Kept>(&self) -> bool;
}

/// Filter a walk strategy can test: the bound of [`WalkStrategy::keep`].
pub(crate) trait Kept: crate::filter::eval::MatchEmit + Collect {}

// Generics: `F` filter.
impl<F: crate::filter::eval::MatchEmit + Collect> Kept for F {}

/// Const plan: `F::Against::<E>::MATCHES`.
/// Generics: `E` emit event.
pub(crate) struct ConstEmit<E> {
    pub(crate) _markers: core::marker::PhantomData<E>,
}

// Generics: `E` emit event.
impl<E> Copy for ConstEmit<E> {}

// Generics: `E` emit event.
impl<E> Clone for ConstEmit<E> {
    fn clone(&self) -> Self {
        *self
    }
}

// Generics: `E` emit tree ([`Collect`]).
impl<E: Collect> WalkStrategy for ConstEmit<E> {
    fn keep<F: Kept>(&self) -> bool {
        let _ = self;
        const { F::Against::<E>::MATCHES }
    }
}

/// Presents a node to [`Visit`].
///
/// A tuple calls [`hand`](Self::hand) on each child, in order. Only `()` and
/// flat lengths 1..=8 do that: one impl per length (rules 11–12). Carriers
/// forward to [`CarrierDesc::Inner`]. Leaves call their [`Visit`] method.
/// Generics: `R` resources.
pub(crate) trait Hand<R> {
    /// Presents this node to `op`.
    fn hand<V>(&self, op: V, res: &mut R)
    where
        V: Copy + Visit<R>;
}

/// Behavior a list visit places in [`Hand`]. Generics: `R` resources.
pub(crate) trait Visit<R> {
    /// Plain `.map` leaf.
    fn on_entry<F, H>(&self, entry: &MapEntry<F, H>, res: &mut R)
    where
        F: MatchEmit + Collect,
        H: MapSys<R>;

    /// Path gate. `M` is the op's emit const, not a parameter here.
    fn on_gate<P, S, C>(&self, gate: &Gate<P, S, C>, res: &mut R)
    where
        P: PathStep,
        S: PathStrategy,
        C: Hand<R>;
}

// A walk strategy is the visit of a map list: it runs the maps it keeps.
// Generics: `R` resources, `S` walk strategy.
impl<R, S> Visit<R> for S
where
    S: WalkStrategy,
{
    fn on_entry<F, H>(&self, entry: &MapEntry<F, H>, res: &mut R)
    where
        F: MatchEmit + Collect,
        H: MapSys<R>,
    {
        if self.keep::<F>() {
            entry.handler.run(res);
        }
    }

    fn on_gate<P, S2, C>(&self, _gate: &Gate<P, S2, C>, _res: &mut R)
    where
        P: PathStep,
        S2: PathStrategy,
        C: Hand<R>,
    {
    }
}

// Generics: `R` resources, `F` filter, `H` handler.
impl<R, F, H> Hand<R> for MapEntry<F, H>
where
    F: MatchEmit + Collect,
    H: MapSys<R>,
{
    fn hand<V>(&self, op: V, res: &mut R)
    where
        V: Copy + Visit<R>,
    {
        op.on_entry(self, res);
    }
}

// Carriers are not the tuple list. They open [`CarrierDesc::Inner`].
// Generics: `R` resources, `I` inner payload.
impl<R, I: Hand<R>> Hand<R> for List<I> {
    fn hand<V>(&self, op: V, res: &mut R)
    where
        V: Copy + Visit<R>,
    {
        self.inner().hand(op, res);
    }
}

// Generics: `R` resources.
impl<R> Hand<R> for End {
    fn hand<V>(&self, _op: V, _res: &mut R)
    where
        V: Copy + Visit<R>,
    {
    }
}

// Generics: `R` resources, `I` items.
impl<R, I: Hand<R>> Hand<R> for Group<I> {
    fn hand<V>(&self, op: V, res: &mut R)
    where
        V: Copy + Visit<R>,
    {
        self.inner().hand(op, res);
    }
}

// Generics: `R` resources, `I` items.
impl<R, I: Hand<R>> Hand<R> for Full<I> {
    fn hand<V>(&self, op: V, res: &mut R)
    where
        V: Copy + Visit<R>,
    {
        self.inner().hand(op, res);
    }
}

// Generics: `R` resources, `I` closed groups.
impl<R, I: Hand<R>> Hand<R> for Head<I> {
    fn hand<V>(&self, op: V, res: &mut R)
    where
        V: Copy + Visit<R>,
    {
        self.inner().hand(op, res);
    }
}

// Generics: `R` resources.
impl<R> Hand<R> for () {
    fn hand<V>(&self, _op: V, _res: &mut R)
    where
        V: Copy + Visit<R>,
    {
    }
}
