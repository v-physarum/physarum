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

#[doc(hidden)]
mod arity;
#[doc(hidden)]
mod ix;

#[doc(hidden)]
#[doc = "Index markers."]
pub use ix::{Ix0, Ix1, Ix2, Ix3, Off};

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
/// The index is a slot type (`Ix0` through `Ix3`), not a number.
///
/// A parent that does not provide an injected bean fails here:
///
/// ```compile_fail
/// use ph_beans::Bean;
/// use ph_beans::Tick;
/// use ph_beans_derive::Component;
///
/// #[derive(Component, Default)]
/// struct Count(u32);
///
/// #[derive(Component)]
/// #[inject(Count)]
/// struct Child;
///
/// impl Tick for Child {
///     type Beans<'a> = ChildBeans<'a>;
///     fn tick(_: Self::Beans<'_>) {}
/// }
///
/// #[derive(Component)]
/// #[provider(Child)]
/// struct Parent;
///
/// impl Tick for Parent {
///     type Beans<'a> = ParentBeans<'a>;
///     fn tick(_: Self::Beans<'_>) {}
/// }
///
/// let mut parent = Parent::create();
/// parent.tick();
/// ```
#[diagnostic::on_unimplemented(message = "`{Self}` does not provide `{P}`")]
pub trait At<P> {
    /// Slot of `P` in `Self`'s provide list.
    type Ix;
}

/// Pulls slot `I` out of a fixed-width tuple.
///
/// [`Take::Rest`] has the same width as `Self`. The taken slot is `Off`.
/// Later indexes still name the same positions.
#[doc(hidden)]
pub trait Take<I> {
    /// The value that occupied slot `I`.
    type Out;
    /// `Self` with `Off` in slot `I`.
    type Rest;

    /// Splits `self` into the slot and the tuple with that hole.
    #[must_use]
    fn take(self) -> (Self::Out, Self::Rest);
}
