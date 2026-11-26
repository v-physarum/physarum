//! How one walk of the map list decides what runs ([`WalkStrategy`]), the
//! walks a parent hands down to a child ([`Pass`]), and the strategy of a
//! direct emit ([`ConstEmit`]).

use core::marker::PhantomData;

use crate::context::ScopeHost;
use crate::filter::eval::{Collect, ConstMatch, MatchEmit};
use crate::query::child::Child;

/// How one walk decides which maps run.
///
/// The const emit and the queued path are structs here.
/// Tuple fan-out is one [`Hand::hand`] (rule 8) — not a method per mode.
pub trait WalkStrategy {
    /// Whether a plain map under filter `F` runs.
    fn keep<F: Kept>(&self) -> bool;

    /// What a `.forward` of child `P` under filter `F` does. The default
    /// does nothing.
    ///
    /// Generics: `F` filter, `P` child, `B` its parent module, `R` borrows.
    fn forward<F: Kept, P: Child<B, R>, B, R>(&self, _res: &mut R) {}
}

/// A walk that runs one event through a module and, by `.forward`, through
/// its children: the direct emit ([`ConstEmit`]) or one deferred path
/// (`Queued`). One method takes either (rule 8): there is no method per mode.
pub trait Pass: WalkStrategy + Copy {
    /// The emit this walk belongs to.
    type Emit: Collect + ScopeHost;
    /// Whether the events deferred into [`Emit`](Self::Emit) run after this
    /// walk. The direct emit runs them; a deferred path is one of them.
    const DRAINS: bool;
}

/// Filter a walk strategy can test: the bound of [`WalkStrategy::keep`].
pub trait Kept: MatchEmit + Collect {}

// Generics: `F` filter.
impl<F: MatchEmit + Collect> Kept for F {}

/// Const plan: `F::Against::<E>::MATCHES`.
/// Generics: `E` emit event.
pub struct ConstEmit<E> {
    _markers: PhantomData<E>,
}

// Generics: `E` emit event.
impl<E> ConstEmit<E> {
    pub(crate) const fn new() -> Self {
        Self { _markers: PhantomData }
    }
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
impl<E: Collect + ScopeHost> WalkStrategy for ConstEmit<E> {
    fn keep<F: Kept>(&self) -> bool {
        const { F::Against::<E>::MATCHES }
    }

    // The child runs for the same emit: its own filters decide at build time.
    fn forward<F: Kept, P: Child<B, R>, B, R>(&self, res: &mut R) {
        if const { F::Against::<E>::MATCHES } {
            P::run(res, *self);
        }
    }
}

// Generics: `E` emit tree ([`Collect`]).
impl<E: Collect + ScopeHost> Pass for ConstEmit<E> {
    type Emit = E;
    const DRAINS: bool = true;
}
