//! State of an event bean, including that event's defer buffer.

use core::cell::Cell;

use crate::filter::eval::EmitMask;

/// Owned state `S` plus the defer buffer for one event.
///
/// [`*Ev::MUT`](crate::Mut) is `Mut<State<Ev>>` and claims `&mut EventBuf<Ev::State>`.
/// Generics: `S` state value.
pub struct EventBuf<S> {
    /// This event's [`BeanInfo::State`](crate::BeanInfo::State).
    pub state: S,
    pending: Cell<EmitMask>,
}

impl<S> EventBuf<S> {
    /// State `state` and an empty defer buffer.
    #[must_use]
    pub const fn new(state: S) -> Self {
        Self { state, pending: Cell::new(EmitMask::EMPTY) }
    }

    /// Events deferred into this scope. They run the next time the scope is entered.
    pub fn pending(&self) -> EmitMask {
        self.pending.get()
    }
}
