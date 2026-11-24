//! Map entries in the shared [`List`](super::list::List). At most [`MAX_MAP_SYSTEMS`] handlers.

use core::marker::PhantomData;

use crate::context::ContextTick;
use crate::context::Pipe;
use crate::context::direct::{DirectFn, TipEvent};
use crate::context::push::PushEntry;
use crate::context::scope::ScopeBufs;

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

impl<F, H> MapEntry<F, H> {
    /// Builds a map leaf (filter `F`, handler `H`).
    ///
    /// Used by `.map` and by [`Gate`](crate::Gate) path trees.
    #[must_use]
    pub const fn new(handler: H) -> Self {
        Self { handler, _markers: PhantomData }
    }
}

/// Shared [`List`](super::list::List) of entries, the emit tree, and the filter in scope.
///
/// `N` is `List<Leaf<()>>` or a longer [`List`](super::list::List): one leaf
/// through eight entries, then groups (eight plus the rest, and four leaves of
/// four at sixteen). It is not a cons spine.
/// Generics: `N` entries list, `E` event tip, `S` scope status.
pub struct MapList<N, E, S> {
    pub(crate) entries: N,
    pub(crate) scopes: ScopeBufs,
    _markers: PhantomData<(E, S)>,
}

impl MapList<super::list::List<super::list::Leaf<()>>, (), ()> {
    pub(crate) const fn empty() -> Self {
        Self {
            entries: super::list::List(super::list::Leaf(())),
            scopes: ScopeBufs::empty(),
            _markers: PhantomData,
        }
    }
}

// Generics: `N` new / next, `E` event, `S` status / state / strategy.
impl<N, E, S> MapList<N, E, S> {
    pub(crate) fn push<H>(self, handler: H) -> MapList<N::Out, E, S>
    where
        N: PushEntry<MapEntry<S, H>>,
    {
        MapList {
            entries: self.entries.push(MapEntry::new(handler)),
            scopes: self.scopes,
            _markers: PhantomData,
        }
    }

    pub(crate) fn push_at<H>(self, handler: H) -> MapList<N::Out, E, S>
    where
        N: PushEntry<MapAt<S, H>>,
    {
        MapList {
            entries: self.entries.push(MapAt::new(handler)),
            scopes: self.scopes,
            _markers: PhantomData,
        }
    }

    pub(crate) fn rescope<F>(self) -> MapList<N, E, F> {
        MapList { entries: self.entries, scopes: self.scopes, _markers: PhantomData }
    }

    pub(crate) fn reemit<T>(self) -> MapList<N, T, S> {
        MapList { entries: self.entries, scopes: self.scopes, _markers: PhantomData }
    }
}

/// One `.map_at` handler under filter `F`. Receives [`Direct`] for that scope.
/// Generics: `F` filter, `H` handler.
pub struct MapAt<F, H> {
    pub(crate) handler: H,
    _markers: PhantomData<F>,
}

impl<F, H> MapAt<F, H> {
    pub(crate) const fn new(handler: H) -> Self {
        Self { handler, _markers: PhantomData }
    }
}

impl<N, E, S> ContextTick for MapList<N, E, S> {}

/// Appends one map under the list's current filter.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "at most 16 map systems are supported (Status = ErrTooManySystems)",
    label = "fix Descriptor::Status",
    note = "MAX_MAP_SYSTEMS = 16; split the pipeline or drop a map"
)]
// Generics: `H` handler.
pub trait PushMap<H> {
    /// Tick after the push.
    type Out: ContextTick;
    /// Stores `handler` on the map tuple.
    fn push_map(self, handler: H) -> Self::Out;
}

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

/// Appends one `.map_at` handler under the list's current filter.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "map_at handler must implement DirectFn for this filter",
    label = "impl DirectFn for the handler",
    note = "direct emit from that handler reaches only a child of the current event"
)]
// Generics: `H` handler.
pub trait PushAt<H> {
    /// Tick after the push.
    type Out: ContextTick;
    /// Stores `handler` on the map tuple.
    fn push_at(self, handler: H) -> Self::Out;
}

// Generics: `H` handler, `I` init slot, `N` list, `E` emit tree, `S` filter scope.
impl<H, I, N, E, S> PushAt<H> for Pipe<I, MapList<N, E, S>>
where
    I: ContextTick,
    N: PushEntry<MapAt<S, H>>,
    S: TipEvent,
    H: DirectFn<S::Tip, S>,
{
    type Out = Pipe<I, MapList<N::Out, E, S>>;

    fn push_at(self, handler: H) -> Self::Out {
        Pipe::new(self.init, self.list.push_at(handler))
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

impl<T, I, N, E, S> Reemit<T> for Pipe<I, MapList<N, E, S>>
where
    I: ContextTick,
{
    type Out = Pipe<I, MapList<N, T, S>>;

    fn reemit(self) -> Self::Out {
        Pipe::new(self.init, self.list.reemit())
    }
}
