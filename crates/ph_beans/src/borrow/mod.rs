//! Slot indexes and the provide check.
//!
//! [`Off`] and [`Ix`]`<N>` are zero-sized. [`At`] names which slot a bean provides.

mod take;

pub use take::{Take, TakeMany};

/// Hole left in a tuple after a slot is taken.
#[doc(hidden)]
pub struct Off;

/// Index of one provided bean. `V` is the slot (0-based).
/// Generics: `V` slot index.
#[doc(hidden)]
pub struct Ix<const V: usize>;

/// `Self` provides `P` at [`At::Ix`] ([`Ix`]`<N>`, not a raw number).
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
