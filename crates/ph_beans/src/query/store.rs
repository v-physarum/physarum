//! Slots a module owns for its provided beans.
//!
//! [`StateOf`] names the owned tuple of [`Slot`]s through [`Shape`]: one per
//! entry of `Provides` ([`Provided`]). Two checks stand behind [`Stored`]:
//! every provide names its access ([`Marked`]), and every provided bean can
//! be created ([`Creates`]). The borrows of one call are in `query/lend.rs`.

use crate::bean::BeanInfo;
use crate::core::shape::{Shape, SlotType};
use crate::descriptor::{Exclusive, Named, Provided, Shared, Unmarked};
use crate::query::lend::Held;
use crate::query::slot::Slot;

/// Type map: the slot a module owns for one provided bean.
pub struct StateOf;

/// How a module keeps the slot of a provide with access `Self`.
#[doc(hidden)]
pub trait Keeps {
    /// The kept form of slot `S`.
    type Kept<S>;
}

// A shared provide is made by `init` and never changes.
impl Keeps for Shared {
    type Kept<S> = Frozen<S>;
}

impl Keeps for Exclusive {
    type Kept<S> = S;
}

// Never built: [`Marked`] rejects the module first.
impl Keeps for Unmarked {
    type Kept<S> = S;
}

// Generics: `E` entry of `Provides` ([`Provided`]).
impl<E> SlotType<E> for StateOf
where
    E: Provided<Bean: BeanInfo<State: Default> + Default, Access: Keeps>,
{
    type Out = <E::Access as Keeps>::Kept<Slot<E::Bean>>;
}

/// A slot provided under [`Get`](crate::Get): it is lent shared only. Generics: `S` slot.
#[derive(Default)]
pub struct Frozen<S>(S);

/// What a module owns for one provide, and the borrow one call takes of it.
///
/// Generics: `'a` the call.
#[doc(hidden)]
pub trait Owns<'a> {
    /// The borrow kept for the call.
    type Lent;
    /// Borrows for one call.
    fn lend(&'a mut self) -> Self::Lent;
}

// Generics: `P` bean.
impl<'a, P: BeanInfo + 'a> Owns<'a> for Slot<P> {
    type Lent = Held<'a, Self>;

    fn lend(&'a mut self) -> Self::Lent {
        Held::new(self)
    }
}

// Generics: `S` slot.
impl<'a, S: 'a> Owns<'a> for Frozen<S> {
    type Lent = &'a S;

    fn lend(&'a mut self) -> Self::Lent {
        &self.0
    }
}

/// Type map: an entry of `Provides` that names its access.
pub struct MarkOf;

// Generics: `E` entry of `Provides` ([`Provided`]).
impl<E: Provided<Access: Named>> SlotType<E> for MarkOf {
    type Out = ();
}

/// Every entry of [`BeanInfo::Provides`] is `Get<P>` or `Mut<P>`.
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `PipelineError<5>` (module `{Self}`)",
    label = "fix Descriptor::Status",
    note = "every provide names its access: `Get<P>` or `Mut<P>`, not a bare bean; PipelineError<5> — docs/errors.md"
)]
pub trait Marked: BeanInfo {}

// Generics: `B` module.
#[diagnostic::do_not_recommend]
impl<B> Marked for B where B: BeanInfo<Provides: Shape<MarkOf>> {}

/// A module can create the slots of its provides.
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `PipelineError<3>` (module `{Self}`)",
    label = "fix Descriptor::Status",
    note = "every provided bean and its state need Default; PipelineError<3> — docs/errors.md"
)]
pub trait Creates: BeanInfo {
    /// Tuple of the slots, in provide order.
    type Slots: Default + 'static;
}

// The compiler reports a failure here as `Creates`, with its code, and does
// not descend to the bound inside. Generics: `B` module, `S` its owned slots.
#[diagnostic::do_not_recommend]
impl<B, S> Creates for B
where
    B: BeanInfo<Provides: Shape<StateOf, Out = S>>,
    S: Default + 'static,
{
    type Slots = S;
}

/// The states a module owns: one per bean in [`BeanInfo::Provides`].
///
/// Two checks stand behind it, each with its own code: every provide names
/// its access ([`Marked`], `PipelineError<5>`), and every provided bean can
/// be created ([`Creates`], `PipelineError<3>`).
pub trait Stored: BeanInfo {
    /// Tuple of those states, in provide order.
    type States: Default + 'static;
}

// Generics: `B` module.
impl<B> Stored for B
where
    B: Creates + Marked,
{
    type States = B::Slots;
}
