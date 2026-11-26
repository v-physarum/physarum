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
    listens: EmitMask,
    systems: usize,
    forwards: usize,
    children: usize,
}

impl ModuleDesc {
    /// A pipeline with no step.
    pub(crate) const EMPTY: Self = Self {
        emits: EmitMask::EMPTY,
        listens: EmitMask::EMPTY,
        systems: 0,
        forwards: 0,
        children: 0,
    };

    /// With one `.map` under a filter that names `events`.
    pub(crate) const fn system(self, events: EmitMask) -> Self {
        Self { systems: self.systems + 1, listens: self.listens.or(events), ..self }
    }

    /// With one `.forward` of a child under a filter that names `events`.
    pub(crate) const fn forward(self, events: EmitMask) -> Self {
        Self { forwards: self.forwards + 1, listens: self.listens.or(events), ..self }
    }

    /// The steps of `self`, then the steps of `next`.
    pub(crate) const fn join(self, next: Self) -> Self {
        Self {
            emits: self.emits.or(next.emits),
            listens: self.listens.or(next.listens),
            systems: self.systems + next.systems,
            forwards: self.forwards + next.forwards,
            children: self.children + next.children,
        }
    }

    /// With the children of the init part `init`: one entry per child.
    pub(crate) const fn making(self, init: Self) -> Self {
        Self { children: init.forwards, ..self }
    }

    /// With the events `.emit` declares.
    pub(crate) const fn declaring(self, emits: EmitMask) -> Self {
        Self { emits, ..self }
    }

    /// Systems stored by `.map`.
    #[inline]
    #[must_use]
    pub const fn systems(&self) -> usize {
        self.systems
    }

    /// `.forward` entries: one per child named by a `.forward` step.
    #[inline]
    #[must_use]
    pub const fn forwards(&self) -> usize {
        self.forwards
    }

    /// Children the pipeline runs: each is made once by `Tick::init`.
    #[inline]
    #[must_use]
    pub const fn children(&self) -> usize {
        self.children
    }

    /// Whether the filters of the systems and of the `.forward` steps name
    /// every event on the path `N`.
    ///
    /// Generics: `N` event path.
    #[must_use]
    pub const fn listens_to<N: Collect>(&self) -> bool {
        self.listens.contains(N::MASK)
    }

    /// Events the pipeline declares with `.emit`: the events it can defer.
    #[inline]
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
