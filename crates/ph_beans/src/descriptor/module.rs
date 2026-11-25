//! The descriptor a tick carries: what its module's pipeline does.

use crate::context::ScopeHost;
use crate::filter::eval::{Collect, EmitMask};

/// Facts about one module's pipeline, as a constant on its
/// [`Tick`](crate::Tick).
///
/// A tick hides its pipeline type. This descriptor is what stays visible:
/// [`Tick::DESC`](crate::Tick::DESC) is a constant, so every question asked of
/// it is answered at build time.
#[derive(Clone, Copy)]
pub struct ModuleDesc {
    emits: EmitMask,
}

impl ModuleDesc {
    pub(crate) const fn new(emits: EmitMask) -> Self {
        Self { emits }
    }

    /// Events the pipeline declares with `.emit`: the events it can defer.
    pub const fn emits(&self) -> EmitMask {
        self.emits
    }

    /// Whether `.emit` declares the path `N`, so it can be deferred.
    ///
    /// Generics: `N` event path.
    #[must_use]
    pub const fn declares<N: Collect>(&self) -> bool {
        self.emits.contains(N::MASK)
    }

    /// Whether an emit of `M` reads the defer queue: `.emit` declares an
    /// event under `M`'s scope event that is not on `M`'s own path.
    ///
    /// A pipeline with no `.emit` defers nothing and never reads the queue.
    /// Generics: `M` emitted event.
    #[must_use]
    pub const fn reads_queue<M: Collect + ScopeHost>(&self) -> bool {
        M::SCOPED
            && self.emits.contains(EmitMask::from_bit(M::BIT))
            && !M::MASK.contains(self.emits)
    }
}
