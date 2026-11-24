//! Slot indexes and the provide check.
//!
//! [`Off`] and [`Ix0`] through [`Ix3`] are zero-sized. [`At`] names which
//! slot a bean provides.

mod take;

pub use take::{Take, TakeMany};

/// Hole left in a tuple after a slot is taken.
#[doc(hidden)]
pub struct Off;

/// Index of one provided bean.
#[doc(hidden)]
pub struct Ix0;

/// Index of one provided bean.
#[doc(hidden)]
pub struct Ix1;

/// Index of one provided bean.
#[doc(hidden)]
pub struct Ix2;

/// Index of one provided bean.
#[doc(hidden)]
pub struct Ix3;

/// `Self` provides `P` at [`At::Ix`].
///
/// The index is a slot type ([`Ix0`] through [`Ix3`]), not a number.
/// A missing provide fails. See `a_missing_provide_is_rejected`.
#[diagnostic::on_unimplemented(message = "`{Self}` does not provide `{P}`")]
pub trait At<P> {
    /// Slot of `P` in `Self`'s provide list.
    type Ix;
}
