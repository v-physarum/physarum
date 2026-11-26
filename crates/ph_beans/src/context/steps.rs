//! The descriptor of a map list: [`Steps`] folds the entries into the
//! constant [`ModuleDesc`]. Nothing is stored in the pipeline value.
//!
//! The fold follows the carriers of [`crate::core::list`]: a tuple joins
//! its children (one impl per length a carrier holds, rules 11–12), a leaf
//! names its own step.
//!
//! This is a second table beside [`Hand`](crate::context::list::Hand): a
//! constant cannot be folded by a walk over values. The owner allowed it
//! as an exception to rule 23.

use crate::context::list::{End, Full, Group, Head, List};
use crate::context::maps::{Forward, MapEntry};
use crate::context::stages::Stages;
use crate::descriptor::ModuleDesc;
use crate::filter::eval::Collect;

/// A node of a map list, described at build time.
#[doc(hidden)]
pub trait Steps {
    /// What the steps under this node hold.
    const DESC: ModuleDesc;
}

// Generics: `F` filter, `H` handler.
impl<F: Collect, H> Steps for MapEntry<F, H> {
    const DESC: ModuleDesc = ModuleDesc::EMPTY.system(F::MASK);
}

// Generics: `F` filter, `P` child bean, `B` its parent.
impl<F: Collect, P, B> Steps for Forward<F, P, B> {
    const DESC: ModuleDesc = ModuleDesc::EMPTY.forward(F::MASK);
}

// The children are the entries of the init part: one per child.
// Generics: `K` marks, `I` init part, `L` systems.
impl<K, I: Steps, L: Steps> Steps for Stages<K, I, L> {
    const DESC: ModuleDesc = L::DESC.making(I::DESC);
}

// Generics: `I` inner payload.
impl<I: Steps> Steps for List<I> {
    const DESC: ModuleDesc = I::DESC;
}

// Generics: `I` closed groups.
impl<I: Steps> Steps for Head<I> {
    const DESC: ModuleDesc = I::DESC;
}

// Generics: `I` items.
impl<I: Steps> Steps for Group<I> {
    const DESC: ModuleDesc = I::DESC;
}

// Generics: `I` items.
impl<I: Steps> Steps for Full<I> {
    const DESC: ModuleDesc = I::DESC;
}

impl Steps for End {
    const DESC: ModuleDesc = ModuleDesc::EMPTY;
}

impl Steps for () {
    const DESC: ModuleDesc = ModuleDesc::EMPTY;
}

// Generics: `E0` child slot.
impl<E0: Steps> Steps for (E0,) {
    const DESC: ModuleDesc = E0::DESC;
}

// Generics: `E0`…`E1` child slots.
impl<E0: Steps, E1: Steps> Steps for (E0, E1) {
    const DESC: ModuleDesc = E0::DESC.join(E1::DESC);
}

// Generics: `E0`…`E2` child slots.
impl<E0: Steps, E1: Steps, E2: Steps> Steps for (E0, E1, E2) {
    const DESC: ModuleDesc = E0::DESC.join(E1::DESC).join(E2::DESC);
}

// Generics: `E0`…`E3` child slots.
impl<E0: Steps, E1: Steps, E2: Steps, E3: Steps> Steps for (E0, E1, E2, E3) {
    const DESC: ModuleDesc = E0::DESC.join(E1::DESC).join(E2::DESC).join(E3::DESC);
}
