//! The scope an emit belongs to: its outermost event.

use crate::filter::On;
use crate::filter::{EmitBit, EmitIx};

/// Outermost event of an emit tree. `()` has no scope.
#[doc(hidden)]
pub trait ScopeHost {
    /// Whether [`BIT`](Self::BIT) names a scope event.
    const SCOPED: bool;
    /// [`EmitIx::BIT`] of the scope event.
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
    const BIT: u32 = <E as EmitIx>::BIT;
}
