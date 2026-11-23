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
///
/// A resource claim that the bean does not provide fails here:
///
/// ```compile_fail
/// use ph_beans::{BeanInfo, DefaultProxy, Get, Marker, Proxied, Query, Target};
///
/// struct Id;
///
/// impl BeanInfo for Id {
///     type Provides = ();
///     type Injects = ();
///     type Beans = ();
///     type State = ();
/// }
///
/// impl Target for Id {}
/// impl Proxied for Id {
///     type Proxy = DefaultProxy<Id>;
///     type Link = Get<DefaultProxy<Id>>;
/// }
///
/// struct App;
///
/// impl BeanInfo for App {
///     type Provides = (Id,);
///     type Injects = ();
///     type Beans = (Id,);
///     type State = ();
/// }
///
/// let _claim = App::query().inject_resources(Id::T);
/// ```
#[diagnostic::on_unimplemented(message = "`{Self}` does not provide `{P}`")]
pub trait At<P> {
    /// Slot of `P` in `Self`'s provide list.
    type Ix;
}
