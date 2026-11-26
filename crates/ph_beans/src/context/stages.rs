//! The parts of a map list: the children to initialise and the systems.
//!
//! [`Stages`] is the first parameter of [`MapList`](crate::MapList). `.map`
//! appends to the systems. `.forward` appends to the systems and, the first
//! time it names a child, to the init part ([`Enlist`]). The init part keeps
//! the order of those first `.forward` steps: `Tick::init` makes the
//! children in that order.

use core::marker::PhantomData;

use crate::borrow::{At, Take};
use crate::context::list::{EmptySlots, Hand, Visit};
use crate::context::maps::Forward;
use crate::core::config::{Config, OfConfig};
use crate::core::list::PushEntry;
use crate::query::child::{Boot, Made};
use crate::query::marks::{Free, Fresh, Start};

/// Init part and systems of one pipeline.
///
/// Generics: `K` one mark per provide slot: the children in `I`, `I` init
/// part: the children in init order, `L` systems list.
pub struct Stages<K, I, L> {
    pub(crate) init: I,
    pub(crate) maps: L,
    _markers: PhantomData<K>,
}

/// Empty stages: no child to initialise, no system.
pub struct EmptyStages;

impl EmptyStages {
    /// Builds [`OfConfig`]`<Self>`.
    #[must_use]
    #[inline]
    pub fn empty() -> OfConfig<Self> {
        Stages { init: EmptySlots::empty(), maps: EmptySlots::empty(), _markers: PhantomData }
    }
}

impl Config for EmptyStages {
    type Type = Stages<<Fresh as Start>::Marks, OfConfig<EmptySlots>, OfConfig<EmptySlots>>;
}

// A system goes to the systems list. Generics: `K` marks, `I` init part,
// `L` systems, `N` new entry.
impl<K, I, L, N> PushEntry<N> for Stages<K, I, L>
where
    L: PushEntry<N>,
{
    type Out = Stages<K, I, L::Out>;

    fn push(self, entry: N) -> Self::Out {
        Stages { init: self.init, maps: self.maps.push(entry), _markers: PhantomData }
    }
}

/// The mark of a provide slot meets a `.forward` of child `P`.
///
/// Generics: `P` child bean, `B` its parent module, `Q` init part.
#[doc(hidden)]
pub trait Admit<P, B, Q> {
    /// The init part after the step.
    type Queue;
    /// Appends `P` unless the init part already holds it.
    fn admit(queue: Q) -> Self::Queue;
}

// The first `.forward` of `P`: it joins the init part, last.
// Generics: `P` child, `B` parent, `Q` init part.
impl<P, B, Q> Admit<P, B, Q> for Free
where
    Q: PushEntry<Forward<(), P, B>>,
{
    type Queue = Q::Out;

    fn admit(queue: Q) -> Self::Queue {
        queue.push(Forward::new())
    }
}

// A later `.forward` of `P`: the init part keeps its place.
// Generics: `P` child, `B` parent, `Q` init part.
impl<P, B, Q> Admit<P, B, Q> for Made<P> {
    type Queue = Q;

    fn admit(queue: Q) -> Self::Queue {
        queue
    }
}

/// Appends one `.forward` of child `P` under filter `F`.
///
/// Generics: `F` filter, `P` child bean, `B` its parent module.
#[doc(hidden)]
pub trait Enlist<F, P, B> {
    /// Stages with the entry in the systems and `P` in the init part.
    type Out;
    /// Stores the entry.
    fn enlist(self) -> Self::Out;
}

// Generics: `K` marks, `I` init part, `L` systems, `F` filter, `P` child,
// `B` parent, `M` the mark of `P`'s slot.
impl<K, I, L, F, P, B, M> Enlist<F, P, B> for Stages<K, I, L>
where
    B: At<P>,
    K: Take<B::Ix, Out = M>,
    M: Admit<P, B, I>,
    L: PushEntry<Forward<F, P, B>>,
{
    type Out = Stages<K::With<Made<P>>, M::Queue, L::Out>;

    fn enlist(self) -> Self::Out {
        let maps = self.maps.push(Forward::new());
        Stages { init: M::admit(self.init), maps, _markers: PhantomData }
    }
}

// A walk visits the systems. Generics: `R` resources, `K` marks, `I` init
// part, `L` systems.
impl<R, K, I, L: Hand<R>> Hand<R> for Stages<K, I, L> {
    fn hand<V>(&self, op: V, res: &mut R)
    where
        V: Copy + Visit<R>,
    {
        self.maps.hand(op, res);
    }
}

/// Entries whose init part can be run over the borrows `R`.
///
/// Generics: `R` borrows.
#[doc(hidden)]
pub trait Makes<R> {
    /// Runs the init system of every child, in init order.
    fn make_all(&self, res: &mut R);
}

// Generics: `R` borrows, `K` marks, `I` init part, `L` systems.
impl<R, K, I: Hand<R>, L> Makes<R> for Stages<K, I, L> {
    fn make_all(&self, res: &mut R) {
        self.init.hand(Boot, res);
    }
}
