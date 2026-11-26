//! Map entries of a pipeline, kept in [`Stages`](super::stages::Stages). At most [`MAX_MAP_SYSTEMS`] handlers.

use core::marker::PhantomData;

use crate::context::ContextTick;
use crate::context::Pipe;
use crate::core::list::PushEntry;

use crate::context::stages::{EmptyStages, Enlist};
use crate::core::config::Config;

/// Hard cap on `.map` handlers in one pipeline.
pub const MAX_MAP_SYSTEMS: usize = 16;

/// One `.map` handler under filter `F`.
///
/// Also a leaf under [`Gate`](crate::Gate) ([`PathNode`](crate::PathNode) for
/// [`Sparse`](crate::Sparse) / [`Dense`](crate::Dense) against an emit const).
/// Generics: `F` filter, `H` handler.
pub struct MapEntry<F, H> {
    pub(crate) handler: H,
    _markers: PhantomData<F>,
}

// Generics: `F` filter, `H` handler.
impl<F, H> MapEntry<F, H> {
    /// Builds a map leaf (filter `F`, handler `H`).
    ///
    /// Used by `.map` and by [`Gate`](crate::Gate) path trees.
    #[must_use]
    pub const fn new(handler: H) -> Self {
        Self { handler, _markers: PhantomData }
    }
}

/// One `.forward`: under filter `F`, the pipeline of bean `P` runs for the
/// event module `B` was emitted with.
///
/// Generics: `F` filter, `P` provided bean, `B` the module that provides it.
pub struct Forward<F, P, B> {
    _markers: PhantomData<(F, P, B)>,
}

// Generics: `F` filter, `P` provided bean, `B` module.
impl<F, P, B> Forward<F, P, B> {
    pub(crate) const fn new() -> Self {
        Self { _markers: PhantomData }
    }
}

/// The entries of a pipeline, the emit tree, and the filter in scope.
///
/// `N` is [`Stages`](super::stages::Stages): the children `Tick::init`
/// makes, and the systems. The systems are a [`List`](super::list::List): a
/// [`Head`](super::list::Head) of closed [`Full`](super::list::Full) groups
/// plus one open slot. It is not a cons spine.
/// Generics: `N` entries, `E` event tip, `S` scope status.
pub struct MapList<N, E, S> {
    pub(crate) entries: N,
    _markers: PhantomData<(E, S)>,
}

// Generics: `L` entries ([`Config::Type`](crate::core::config::Config::Type)).
impl<L> MapList<L, (), ()>
where
    EmptyStages: Config<Type = L>,
{
    pub(crate) fn empty() -> Self {
        Self { entries: EmptyStages::empty(), _markers: PhantomData }
    }
}

// Generics: `N` new / next, `E` event, `S` status / state / strategy.
impl<N, E, S> MapList<N, E, S> {
    pub(crate) fn push<H>(self, handler: H) -> MapList<N::Out, E, S>
    where
        N: PushEntry<MapEntry<S, H>>,
    {
        MapList { entries: self.entries.push(MapEntry::new(handler)), _markers: PhantomData }
    }

    // Generics: `P` child, `B` its parent.
    pub(crate) fn push_forward<P, B>(self) -> MapList<N::Out, E, S>
    where
        N: Enlist<S, P, B>,
    {
        MapList { entries: self.entries.enlist(), _markers: PhantomData }
    }

    pub(crate) fn rescope<F>(self) -> MapList<N, E, F> {
        MapList { entries: self.entries, _markers: PhantomData }
    }

    pub(crate) fn reemit<T>(self) -> MapList<N, T, S> {
        MapList { entries: self.entries, _markers: PhantomData }
    }
}

// Generics: `N` new / next, `E` event, `S` status / state / strategy.
impl<N, E, S> ContextTick for MapList<N, E, S> {}

/// Appends one map under the list's current filter.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "at most 16 map systems are supported (Status = PipelineError<2>)",
    label = "fix Descriptor::Status",
    note = "MAX_MAP_SYSTEMS = 16; PipelineError<2> — docs/errors.md"
)]
// Generics: `H` handler.
pub trait PushMap<H> {
    /// Tick after the push.
    type Out: ContextTick;
    /// Stores `handler` on the map tuple.
    fn push_map(self, handler: H) -> Self::Out;
}

// Generics: `H` handler, `I` inner / index, `N` new / next, `E` event, `S` status / state / strategy.
impl<H, I, N, E, S> PushMap<H> for Pipe<I, MapList<N, E, S>>
where
    I: ContextTick,
    N: PushEntry<MapEntry<S, H>>,
{
    type Out = Pipe<I, MapList<N::Out, E, S>>;

    fn push_map(self, handler: H) -> Self::Out {
        Pipe::new(self.init, self.list.push(handler))
    }
}

/// Replaces the filter that the next `.map` will capture.
/// Generics: `F` filter.
#[doc(hidden)]
pub trait Rescope<F> {
    /// Tick after the scope change.
    type Out: ContextTick;
    /// Sets the current filter to `F`.
    fn rescope(self) -> Self::Out;
}

// Generics: `F` filter, `I` inner / index, `N` new / next, `E` event, `S` status / state / strategy.
impl<F, I, N, E, S> Rescope<F> for Pipe<I, MapList<N, E, S>>
where
    I: ContextTick,
{
    type Out = Pipe<I, MapList<N, E, F>>;

    fn rescope(self) -> Self::Out {
        Pipe::new(self.init, self.list.rescope())
    }
}

/// The emit parameter stored on this tick.
#[doc(hidden)]
pub trait EmitOf {
    /// Emit tree recorded so far.
    type Emit;
}

// Generics: `I` inner / index, `N` new / next, `E` event, `S` status / state / strategy.
impl<I, N, E, S> EmitOf for Pipe<I, MapList<N, E, S>> {
    type Emit = E;
}

/// Replaces the emit parameter. `T` is the unioned tree, not the delta.
/// Generics: `T` tick emit.
#[doc(hidden)]
pub trait Reemit<T> {
    /// Tick after the union.
    type Out: ContextTick;
    /// Stores the unioned emit tree.
    fn reemit(self) -> Self::Out;
}

// Generics: `T` tick emit, `I` inner / index, `N` new / next, `E` event, `S` status / state / strategy.
impl<T, I, N, E, S> Reemit<T> for Pipe<I, MapList<N, E, S>>
where
    I: ContextTick,
{
    type Out = Pipe<I, MapList<N, T, S>>;

    fn reemit(self) -> Self::Out {
        Pipe::new(self.init, self.list.reemit())
    }
}
