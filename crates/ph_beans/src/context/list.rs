//! Shared list carrier.
//!
//! A leaf is a tuple of at most eight items. Past that, items sit in groups
//! of those leaves. Push and the map walk both use this carrier.
//!
//! The trait and the carrier cannot both be named `List` (E0428). The carrier
//! is [`List`]. Traversal is [`Walk`]. Append is [`PushShape`](super::push::PushShape).

use core::any::TypeId;

/// Tuple of at most eight items, or a tuple of such groups.
pub struct List<I>(pub(crate) I);

/// Flat leaf while the list is growing through lengths 0..=8.
///
/// Distinct from a bare tuple so [`PushShape`](super::push::PushShape) can use
/// opaque element params (`E0`, …) without overlapping `(List, Grow)`.
pub struct Leaf<I>(pub(crate) I);

/// Second group while the list is growing through lengths 9..=15.
///
/// Distinct from [`List`] so `(List<H>, Grow<R>)` can have one
/// [`PushShape`](super::push::PushShape) blanket (`R: AddItem`) without
/// overlapping the sixteen-item reshape (E0119). At sixteen the shape
/// becomes four [`List`] leaves again.
pub struct Grow<I>(pub(crate) I);

/// Walks a leaf, then each group, in order.
#[doc(hidden)]
pub trait Walk {
    /// Visits every leaf item.
    fn walk(&self, emitted: &[TypeId]);
}

impl Walk for () {
    fn walk(&self, _emitted: &[TypeId]) {
        let () = self;
    }
}

impl<E0> Walk for (E0,)
where
    E0: Walk,
{
    fn walk(&self, emitted: &[TypeId]) {
        self.0.walk(emitted);
    }
}

impl<E0, E1> Walk for (E0, E1)
where
    E0: Walk,
    E1: Walk,
{
    fn walk(&self, emitted: &[TypeId]) {
        self.0.walk(emitted);
        self.1.walk(emitted);
    }
}

impl<E0, E1, E2> Walk for (E0, E1, E2)
where
    E0: Walk,
    E1: Walk,
    E2: Walk,
{
    fn walk(&self, emitted: &[TypeId]) {
        self.0.walk(emitted);
        self.1.walk(emitted);
        self.2.walk(emitted);
    }
}

impl<E0, E1, E2, E3> Walk for (E0, E1, E2, E3)
where
    E0: Walk,
    E1: Walk,
    E2: Walk,
    E3: Walk,
{
    fn walk(&self, emitted: &[TypeId]) {
        self.0.walk(emitted);
        self.1.walk(emitted);
        self.2.walk(emitted);
        self.3.walk(emitted);
    }
}

impl<E0, E1, E2, E3, E4> Walk for (E0, E1, E2, E3, E4)
where
    E0: Walk,
    E1: Walk,
    E2: Walk,
    E3: Walk,
    E4: Walk,
{
    fn walk(&self, emitted: &[TypeId]) {
        self.0.walk(emitted);
        self.1.walk(emitted);
        self.2.walk(emitted);
        self.3.walk(emitted);
        self.4.walk(emitted);
    }
}

impl<E0, E1, E2, E3, E4, E5> Walk for (E0, E1, E2, E3, E4, E5)
where
    E0: Walk,
    E1: Walk,
    E2: Walk,
    E3: Walk,
    E4: Walk,
    E5: Walk,
{
    fn walk(&self, emitted: &[TypeId]) {
        self.0.walk(emitted);
        self.1.walk(emitted);
        self.2.walk(emitted);
        self.3.walk(emitted);
        self.4.walk(emitted);
        self.5.walk(emitted);
    }
}

impl<E0, E1, E2, E3, E4, E5, E6> Walk for (E0, E1, E2, E3, E4, E5, E6)
where
    E0: Walk,
    E1: Walk,
    E2: Walk,
    E3: Walk,
    E4: Walk,
    E5: Walk,
    E6: Walk,
{
    fn walk(&self, emitted: &[TypeId]) {
        self.0.walk(emitted);
        self.1.walk(emitted);
        self.2.walk(emitted);
        self.3.walk(emitted);
        self.4.walk(emitted);
        self.5.walk(emitted);
        self.6.walk(emitted);
    }
}

impl<E0, E1, E2, E3, E4, E5, E6, E7> Walk for (E0, E1, E2, E3, E4, E5, E6, E7)
where
    E0: Walk,
    E1: Walk,
    E2: Walk,
    E3: Walk,
    E4: Walk,
    E5: Walk,
    E6: Walk,
    E7: Walk,
{
    fn walk(&self, emitted: &[TypeId]) {
        self.0.walk(emitted);
        self.1.walk(emitted);
        self.2.walk(emitted);
        self.3.walk(emitted);
        self.4.walk(emitted);
        self.5.walk(emitted);
        self.6.walk(emitted);
        self.7.walk(emitted);
    }
}

impl<I> Walk for List<I>
where
    I: Walk,
{
    fn walk(&self, emitted: &[TypeId]) {
        self.0.walk(emitted);
    }
}

impl<I> Walk for Leaf<I>
where
    I: Walk,
{
    fn walk(&self, emitted: &[TypeId]) {
        self.0.walk(emitted);
    }
}

impl<I> Walk for Grow<I>
where
    I: Walk,
{
    fn walk(&self, emitted: &[TypeId]) {
        self.0.walk(emitted);
    }
}
