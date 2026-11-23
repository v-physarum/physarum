//! Manual provide / inject against a parent surface.
//!
//! Membership is [`ClaimMark`]. A pair uses `OfProvide0` / `OfProvide1` (and
//! the inject twins). A longer list is one impl `for (A0, A1, A2, …)`: the
//! elements are generics, not slot markers. The inject tuple is collected on
//! [`Collected`](crate::Collected). A nested tail such as
//! `(Get<T>, (Get<U>, ()))` is not a list.

mod tuple;

use core::marker::PhantomData;

use crate::bean::BeanInfo;
use crate::descriptor::{Get, Mut};

/// The child is injecting the parent bean itself.
pub struct OfParent;

/// The child is injecting the parent's single provide.
pub struct OfProvide;

/// The child is injecting the parent's inject list.
///
/// A single claim uses [`OfInject`]. A flat tuple uses `OfInject<(A0, A1, …)>`.
pub struct OfInject<T = ()>(PhantomData<T>);

/// First claim of a flat provide pair.
pub struct OfProvide0;

/// Second claim of a flat provide pair.
pub struct OfProvide1;

/// First claim of a flat inject pair.
pub struct OfInject0;

/// Second claim of a flat inject pair.
pub struct OfInject1;

/// `Self` may be injected from `P` via `V`.
///
/// ```compile_fail
/// use ph_beans::{BeanInfo, FromParent, Get, OfProvide0};
///
/// struct Id;
/// struct Keys;
/// struct App;
///
/// impl BeanInfo for App {
///     type Provides = (Get<Id>, (Get<Keys>, ()));
///     type Injects = ();
///     type Beans = ();
///     type State = ();
/// }
///
/// fn first<C: FromParent<App, OfProvide0>>() {}
/// first::<Get<Id>>();
/// ```
///
/// ```compile_fail
/// use ph_beans::{BeanInfo, FromParent, Get, Mut};
///
/// struct Id;
/// struct Keys;
/// struct App;
///
/// impl BeanInfo for App {
///     type Provides = (Get<Id>, Mut<Keys>);
///     type Injects = ();
///     type Beans = ();
///     type State = ();
/// }
///
/// fn nested<C: FromParent<App, (Get<Id>, Mut<Keys>)>>() {}
/// nested::<(Get<Id>, (Mut<Keys>, ()))>();
/// ```
///
/// ```compile_fail
/// use ph_beans::{BeanInfo, FromParent, Get, Mut, OfProvide};
///
/// struct Logic;
/// struct App;
///
/// impl BeanInfo for Logic {
///     type Provides = ();
///     type Injects = ();
///     type Beans = ();
///     type State = ();
/// }
///
/// impl BeanInfo for App {
///     type Provides = Get<Logic>;
///     type Injects = ();
///     type Beans = Get<Logic>;
///     type State = ();
/// }
///
/// fn mount<C>()
/// where
///     C: FromParent<App, OfProvide>,
/// {
/// }
///
/// mount::<Mut<Logic>>();
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not on `{P}` via `{V}`",
    label = "fix Descriptor::Status",
    note = "a child may inject the parent, or the parent's injects or provides; MUT against an immutable provide is rejected"
)]
pub trait FromParent<P, V> {}

/// `Get` or `Mut`. One bound covers both marker kinds.
pub(super) trait ClaimMark {}

impl<T> ClaimMark for Get<T> {}

impl<T> ClaimMark for Mut<T> {}

/// The claim is the parent bean itself.
trait IsParent<P> {}

impl<P> IsParent<P> for Get<P> {}

impl<P> IsParent<P> for Mut<P> {}

impl<P, C> FromParent<P, OfParent> for C where C: IsParent<P> {}

impl<P> FromParent<P, ()> for () {}

impl<P, C> FromParent<P, OfProvide> for C
where
    P: BeanInfo<Provides = C>,
    C: ClaimMark,
{
}

impl<P, C> FromParent<P, OfInject> for C
where
    P: BeanInfo<Injects = C>,
    C: ClaimMark,
{
}

// Flat pairs. The other element is a claim, so a nested tail does not match.
impl<P, C, R> FromParent<P, OfProvide0> for C
where
    P: BeanInfo<Provides = (C, R)>,
    C: ClaimMark,
    R: ClaimMark,
{
}

impl<P, C, L> FromParent<P, OfProvide1> for C
where
    P: BeanInfo<Provides = (L, C)>,
    C: ClaimMark,
    L: ClaimMark,
{
}

impl<P, C, R> FromParent<P, OfInject0> for C
where
    P: BeanInfo<Injects = (C, R)>,
    C: ClaimMark,
    R: ClaimMark,
{
}

impl<P, C, L> FromParent<P, OfInject1> for C
where
    P: BeanInfo<Injects = (L, C)>,
    C: ClaimMark,
    L: ClaimMark,
{
}

/// Child whose inject list is on `P`'s surface through `V`.
pub trait Module<P, V>: BeanInfo {}

impl<C, P, V> Module<P, V> for C
where
    C: BeanInfo,
    C::Injects: FromParent<P, V>,
{
}
