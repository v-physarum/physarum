//! One defer buffer per event bit. The event is the scope that later runs it.

use core::cell::Cell;

use crate::filter::On;
use crate::filter::eval::{EmitBit, EmitMask};

/// Buffers owned by events. Index is [`EmitBit::BIT`].
#[doc(hidden)]
pub struct ScopeBufs {
    slots: [Cell<EmitMask>; EmitMask::BITS as usize],
}

impl ScopeBufs {
    /// One slot per [`EmitMask::BITS`].
    pub const SLOTS: usize = EmitMask::BITS as usize;

    pub(crate) const fn empty() -> Self {
        Self { slots: [const { Cell::new(EmitMask::EMPTY) }; Self::SLOTS] }
    }

    pub(crate) fn or_bit(&self, bit: u32, mask: EmitMask) {
        let slot = &self.slots[bit as usize];
        slot.set(slot.get().or(mask));
    }

    pub(crate) fn take_bit(&self, bit: u32) -> EmitMask {
        self.slots[bit as usize].replace(EmitMask::EMPTY)
    }
}

/// Outermost event of an emit tree. `()` has no scope.
#[doc(hidden)]
pub trait ScopeHost {
    /// Whether [`BIT`](Self::BIT) names a buffer.
    const SCOPED: bool;
    /// [`EmitBit::BIT`](EmitBit::BIT) of the scope event.
    const BIT: u32;
}

impl ScopeHost for () {
    const SCOPED: bool = false;
    const BIT: u32 = 0;
}

// Generics: `E` scope event, `I` nested emit tree.
impl<E, I> ScopeHost for On<E, I>
where
    E: EmitBit,
{
    const SCOPED: bool = true;
    const BIT: u32 = E::BIT;
}
