//! Map entries in the shared [`List`](super::list::List). At most [`MAX_MAP_SYSTEMS`] handlers.

use core::marker::PhantomData;

use crate::context::ContextTick;
use crate::context::Pipe;
use crate::context::push::PushEntry;

/// Hard cap on `.map` handlers in one pipeline.
pub const MAX_MAP_SYSTEMS: usize = 16;

/// One `.map` handler under filter `F`.
pub struct MapEntry<F, H> {
    pub(crate) handler: H,
    _markers: PhantomData<F>,
}

impl<F, H> MapEntry<F, H> {
    pub(crate) fn new(handler: H) -> Self {
        Self { handler, _markers: PhantomData }
    }
}

/// Shared [`List`](super::list::List) of entries, the emit tree, and the filter in scope.
///
/// `N` is `List<Leaf<()>>` or a longer [`List`](super::list::List): one leaf
/// through eight entries, then groups (eight plus the rest, and four leaves of
/// four at sixteen). It is not a cons spine.
pub struct MapList<N, E, S> {
    pub(crate) entries: N,
    _markers: PhantomData<(E, S)>,
}

impl MapList<super::list::List<super::list::Leaf<()>>, (), ()> {
    pub(crate) const fn empty() -> Self {
        Self { entries: super::list::List(super::list::Leaf(())), _markers: PhantomData }
    }
}

impl<N, E, S> MapList<N, E, S> {
    pub(crate) fn push<H>(self, handler: H) -> MapList<N::Out, E, S>
    where
        N: PushEntry<MapEntry<S, H>>,
    {
        MapList { entries: self.entries.push(MapEntry::new(handler)), _markers: PhantomData }
    }

    pub(crate) fn rescope<F>(self) -> MapList<N, E, F> {
        MapList { entries: self.entries, _markers: PhantomData }
    }

    pub(crate) fn reemit<T>(self) -> MapList<N, T, S> {
        MapList { entries: self.entries, _markers: PhantomData }
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

/// Replaces the filter that the next `.map` will capture.
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

impl<I, N, E, S> EmitOf for Pipe<I, MapList<N, E, S>> {
    type Emit = E;
}

/// Replaces the emit parameter. `T` is the unioned tree, not the delta.
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
