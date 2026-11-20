//! Static provide, inject, and tick.
//!
//! A module owns the beans listed in `#[provider(..)]`. It receives the beans
//! listed in `#[inject(..)]` from its immediate parent only. [`Tick`] runs
//! after those provided modules. Slot selection is an index type, not a search.
//!
//! `ph_beans_derive` builds the state and the schedule. The author writes
//! [`Tick`] for a module and [`Default`] for a data bean.

#![forbid(unsafe_code)]
extern crate self as ph_beans;

mod take;

/// A value stored in a module, or a module stored in its parent.
///
/// `State` is the owned field. [`Bean::create`] builds it once, before the
/// first tick.
pub trait Bean {
    /// Owned value held by the parent that provides `Self`.
    type State;

    /// Builds the owned state.
    #[must_use]
    fn create() -> Self::State;
}

/// Work a module does after the modules it provides.
///
/// `Beans` is the inject list, then the provide list. Each element is
/// `&mut State` for that bean.
pub trait Tick {
    /// Injected beans, then provided beans, as mutable references.
    type Beans<'a>;

    /// Runs this module on `beans`.
    fn tick(beans: Self::Beans<'_>);
}

/// `Self` provides `P` at [`At::Ix`].
///
/// The index is a slot type ([`Ix0`] through [`Ix3`]), not a number.
#[diagnostic::on_unimplemented(message = "`{Self}` does not provide `{P}`")]
pub trait At<P> {
    /// Slot of `P` in `Self`'s provide list.
    type Ix;
}

/// Pulls slot `I` out of a fixed-width tuple.
///
/// [`Take::Rest`] has the same width as `Self`. The taken slot is [`Off`].
/// Later indexes still name the same positions.
#[doc(hidden)]
pub trait Take<I> {
    /// The value that occupied slot `I`.
    type Out;
    /// `Self` with [`Off`] in slot `I`.
    type Rest;

    /// Splits `self` into the slot and the tuple with that hole.
    #[must_use]
    fn take(self) -> (Self::Out, Self::Rest);
}

/// Hole left in a tuple after [`Take`].
#[derive(Clone, Copy, Debug)]
pub struct Off;

/// First provide slot.
#[derive(Clone, Copy, Debug)]
pub struct Ix0;

/// Second provide slot.
#[derive(Clone, Copy, Debug)]
pub struct Ix1;

/// Third provide slot.
#[derive(Clone, Copy, Debug)]
pub struct Ix2;

/// Fourth provide slot.
#[derive(Clone, Copy, Debug)]
pub struct Ix3;

#[cfg(test)]
mod tests {
    use ph_beans::{Ix0, Ix1, Ix2, Ix3, Off, Take};

    #[test]
    fn take_leaves_a_hole_at_the_same_index() {
        let mut first = 1;
        let mut second = 2;
        let mut third = 3;
        let mut fourth = 4;
        let tuple = (&mut first, &mut second, &mut third, &mut fourth);
        let (taken, rest) = Take::<Ix3>::take(tuple);
        assert_eq!(*taken, 4);
        let (taken, rest) = Take::<Ix0>::take(rest);
        assert_eq!(*taken, 1);
        let (taken, rest) = Take::<Ix2>::take(rest);
        assert_eq!(*taken, 3);
        let (taken, rest) = Take::<Ix1>::take(rest);
        assert_eq!(*taken, 2);
        let _: (Off, Off, Off, Off) = rest;
    }

    #[test]
    fn take_width_one_replaces_the_only_slot() {
        let mut only = 7;
        let (taken, rest) = Take::<Ix0>::take((&mut only,));
        assert_eq!(*taken, 7);
        let _: (Off,) = rest;
    }
}
