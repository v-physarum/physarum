//! The borrows of one call: what a module lends to its systems.
//!
//! The shared walk [`Each`](crate::core::each::Each) splits the owned slots
//! with [`Borrow`] and lends them to one system with [`Reborrow`]. Injects
//! are what the caller lends ([`ClaimShape::Lent`]); they join the borrows
//! with [`AddItem`].

use crate::core::each::{Each, SlotFn};
use crate::core::list::AddItem;
use crate::query::args::ClaimShape;
use crate::query::store::{Owns, Stored};

/// An exclusive borrow held for one call. Generics: `S` borrowed state.
///
/// The per-call tuple keeps this and not a bare `&mut S`, so lending a slot
/// again is `&mut Held<S>` and never `&mut &mut S`.
pub struct Held<'x, S: ?Sized>(&'x mut S);

// Generics: `S` borrowed state.
impl<'x, S: ?Sized> Held<'x, S> {
    pub(crate) fn new(state: &'x mut S) -> Self {
        Self(state)
    }
}

/// The borrow of a slot provided under [`Get`](crate::Get), held for one
/// call. A claim reads it shared. Only `.forward` opens it: the child in
/// that slot is run over its own node. Generics: `S` borrowed slot.
pub struct Cold<'x, S: ?Sized>(&'x mut S);

// Generics: `S` borrowed slot.
impl<'x, S: ?Sized> Cold<'x, S> {
    pub(crate) fn new(slot: &'x mut S) -> Self {
        Self(slot)
    }

    pub(crate) fn open(self) -> &'x mut S {
        self.0
    }

    pub(crate) fn read(self) -> &'x S {
        self.0
    }
}

/// Walk: one borrow per owned slot, exclusive unless the slot is [`Frozen`].
pub struct Borrow;

// Generics: `S` owned slot ([`Owns`]).
impl<'a, S: Owns<'a>> SlotFn<&'a mut S> for Borrow {
    type Out = S::Lent;

    fn slot(item: &'a mut S) -> Self::Out {
        item.lend()
    }
}

/// Walk: lends each borrow of a call to one system.
pub struct Reborrow;

// Generics: `S` exclusively borrowed state.
impl<'a, S: ?Sized> SlotFn<&'a mut Held<'_, S>> for Reborrow {
    type Out = &'a mut S;

    fn slot(item: &'a mut Held<'_, S>) -> Self::Out {
        item.0
    }
}

// Generics: `S` slot provided under `Get`.
impl<'a, S: ?Sized> SlotFn<&'a mut Cold<'_, S>> for Reborrow {
    type Out = Cold<'a, S>;

    fn slot(item: &'a mut Cold<'_, S>) -> Self::Out {
        Cold(item.0)
    }
}

// Generics: `S` shared state of an inject.
impl<'a, S: ?Sized> SlotFn<&'a mut &S> for Reborrow {
    type Out = &'a S;

    fn slot(item: &'a mut &S) -> Self::Out {
        item
    }
}

/// A borrow an inject view is made of: `&S` or `&mut S`.
#[doc(hidden)]
pub trait Borrowed {
    /// How the per-call tuple keeps it.
    type Kept;
    /// Wraps the borrow for the per-call tuple.
    fn keep(self) -> Self::Kept;
}

// Generics: `S` injected state.
impl<S: ?Sized> Borrowed for &S {
    type Kept = Self;

    fn keep(self) -> Self::Kept {
        self
    }
}

// Generics: `S` injected state.
impl<'x, S: ?Sized> Borrowed for &'x mut S {
    type Kept = Held<'x, S>;

    fn keep(self) -> Self::Kept {
        Held(self)
    }
}

/// An inject view placed after the borrows `R` of the provided states.
///
/// `B::Beans` is provides, then injects, so [`At`](crate::At) indexes the
/// joined tuple. Generics: `R` provided borrows.
pub trait Join<R> {
    /// `R`, then one borrow per inject.
    type Out;
    /// Appends this view to `refs`.
    fn join(self, refs: R) -> Self::Out;
}

// Generics: `R` provided borrows.
impl<R> Join<R> for () {
    type Out = R;

    fn join(self, refs: R) -> Self::Out {
        refs
    }
}

// Generics: `R` provided borrows, `V` one injected borrow.
impl<R, V> Join<R> for V
where
    V: Borrowed,
    R: AddItem<V::Kept>,
{
    type Out = R::Out;

    fn join(self, refs: R) -> Self::Out {
        refs.add_item(self.keep())
    }
}

// Generics: `R` provided borrows, `V0`…`V1` views of two injects.
impl<R, V0, V1> Join<R> for (V0, V1)
where
    V0: Join<R>,
    V1: Join<V0::Out>,
{
    type Out = V1::Out;

    fn join(self, refs: R) -> Self::Out {
        self.1.join(self.0.join(refs))
    }
}

/// The borrows one `emit` lends to its maps: owned states, then injects.
///
/// Generics: `'s` the owned states, `'x` the inject view.
pub trait Beans<'s, 'x>: Stored<Injects: ClaimShape + 'x> {
    /// One borrow per bean in [`BeanInfo::Beans`].
    type Full;
    /// Borrows the states and appends `injects`.
    fn full(
        states: &'s mut Self::States,
        injects: <Self::Injects as ClaimShape>::Lent<'x>,
    ) -> Self::Full;
}

// Generics: `B` module, `R` provided borrows, `V` inject view, `O` joined borrows.
impl<'s, 'x, B, R, V, O> Beans<'s, 'x> for B
where
    B: Stored<Injects: ClaimShape<Lent<'x> = V> + 'x>,
    &'s mut B::States: Each<Borrow, Out = R>,
    V: Join<R, Out = O>,
{
    type Full = O;

    fn full(states: &'s mut Self::States, injects: V) -> O {
        injects.join(states.each())
    }
}
