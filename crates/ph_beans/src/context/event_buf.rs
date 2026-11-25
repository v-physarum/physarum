//! The defer buffer of an event, kept in the event's own state.

use crate::filter::eval::EmitMask;

/// Events deferred into one scope event, in the order they were deferred.
///
/// An event that can take deferred events keeps this in its
/// [`BeanInfo::State`](crate::BeanInfo::State): the state is `EventBuf`, or a
/// struct that holds one and implements [`DeferState`]. The emit of that
/// event runs the entries, oldest first. The storage keeps its capacity:
/// only a `defer` can allocate.
#[derive(Default)]
pub struct EventBuf {
    paths: Vec<EmitMask>,
}

impl EventBuf {
    /// An empty buffer.
    #[must_use]
    pub const fn new() -> Self {
        Self { paths: Vec::new() }
    }

    /// Events waiting now.
    #[must_use]
    #[inline]
    pub const fn len(&self) -> usize {
        self.paths.len()
    }

    /// Whether no event waits.
    #[must_use]
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    #[inline]
    pub(crate) fn push(&mut self, path: EmitMask) {
        self.paths.push(path);
    }

    /// Path of entry `index`, oldest first.
    #[inline]
    pub(crate) fn path_at(&self, index: usize) -> Option<EmitMask> {
        self.paths.get(index).copied()
    }

    /// Drops every entry and keeps the allocation.
    #[inline]
    pub(crate) fn clear(&mut self) {
        self.paths.clear();
    }
}

/// The state of an event that can take deferred events.
///
/// The event's [`BeanInfo::State`](crate::BeanInfo::State) implements this,
/// so `Tick::defer` and the emit of that event reach its buffer.
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `FilterError<7>` (state `{Self}`)",
    label = "fix Descriptor::Status",
    note = "the state of a scope event must hold an `EventBuf` (`DeferState`); FilterError<7> — docs/errors.md"
)]
pub trait DeferState {
    /// The buffer of deferred events.
    fn deferred(&mut self) -> &mut EventBuf;
}

impl DeferState for EventBuf {
    #[inline]
    fn deferred(&mut self) -> &mut EventBuf {
        self
    }
}
