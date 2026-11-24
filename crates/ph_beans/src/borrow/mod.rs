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

/// `Self` provides `P` at [`At::Ix`] ([`Ix0`]…[`Ix3`], not a number).
/// ```compile_fail
/// use ph_beans::{BeanInfo, Marker, Query};
/// struct Id; impl BeanInfo for Id { type Provides=(); type Injects=(); type Beans=(); type State=(); }
/// struct App; impl BeanInfo for App { type Provides=(Id,); type Injects=(); type Beans=(Id,); type State=(); }
/// let _ = App::query().inject_resources(Id::T);
/// ```
/// Generics: `P` provided bean.
#[diagnostic::on_unimplemented(message = "`{Self}` does not provide `{P}`")]
pub trait At<P> {
    /// Slot of `P` in `Self`'s provide list.
    type Ix;
}
