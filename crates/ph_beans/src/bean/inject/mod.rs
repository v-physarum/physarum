//! Manual provide / inject against a parent surface.
//!
//! Membership is [`ClaimMark`] + a flat tuple of at most eight claims.
//! Slot markers (`OfProvide0`, …) name the index. A child inject list is the
//! same flat tuple: each element is a claim, and the tuple impl does not
//! recurse. A nested tail such as `(Get<T>, (Get<U>, ()))` is not a list.
//! The markers are not a Get×Mut matrix.

mod flat;

use crate::bean::BeanInfo;
use crate::descriptor::{Get, Mut};

/// The child is injecting the parent bean itself.
pub struct OfParent;

/// The child is injecting the parent's single provide.
pub struct OfProvide;

/// The child is injecting the parent's single inject.
pub struct OfInject;

/// First claim of a flat provide pair.
pub struct OfProvide0;

/// Second claim of a flat provide pair.
pub struct OfProvide1;

/// Index `I` in a flat provide list of length `N` (3..=8).
///
/// A pair keeps [`OfProvide0`] / [`OfProvide1`]. Longer lists cannot share
/// those markers: two impls that differ only by tuple length overlap (E0119).
pub struct ProvideAt<const N: u8, const I: u8>;

/// First claim of a flat inject list (two claims).
pub struct OfInject0;

/// Second claim of a flat inject list (two claims).
pub struct OfInject1;

/// Index `I` in a flat inject list of length `N` (3..=8).
///
/// A pair keeps [`OfInject0`] / [`OfInject1`].
pub struct InjectAt<const N: u8, const I: u8>;

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
/// use ph_beans::{BeanInfo, FromParent, Get, Mut, OfProvide0, OfProvide1};
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
/// fn nested<C: FromParent<App, (OfProvide0, OfProvide1)>>() {}
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

// A child inject list is one flat tuple of claims. A nested tail does not match.
impl<P, V0, V1, A, B> FromParent<P, (V0, V1)> for (A, B)
where
    A: FromParent<P, V0> + ClaimMark,
    B: FromParent<P, V1> + ClaimMark,
{
}

impl<P, V0, V1, V2, A, B, C> FromParent<P, (V0, V1, V2)> for (A, B, C)
where
    A: FromParent<P, V0> + ClaimMark,
    B: FromParent<P, V1> + ClaimMark,
    C: FromParent<P, V2> + ClaimMark,
{
}

impl<P, V0, V1, V2, V3, A, B, C, D> FromParent<P, (V0, V1, V2, V3)> for (A, B, C, D)
where
    A: FromParent<P, V0> + ClaimMark,
    B: FromParent<P, V1> + ClaimMark,
    C: FromParent<P, V2> + ClaimMark,
    D: FromParent<P, V3> + ClaimMark,
{
}

impl<P, V0, V1, V2, V3, V4, A, B, C, D, E> FromParent<P, (V0, V1, V2, V3, V4)> for (A, B, C, D, E)
where
    A: FromParent<P, V0> + ClaimMark,
    B: FromParent<P, V1> + ClaimMark,
    C: FromParent<P, V2> + ClaimMark,
    D: FromParent<P, V3> + ClaimMark,
    E: FromParent<P, V4> + ClaimMark,
{
}

impl<P, V0, V1, V2, V3, V4, V5, A, B, C, D, E, F> FromParent<P, (V0, V1, V2, V3, V4, V5)>
    for (A, B, C, D, E, F)
where
    A: FromParent<P, V0> + ClaimMark,
    B: FromParent<P, V1> + ClaimMark,
    C: FromParent<P, V2> + ClaimMark,
    D: FromParent<P, V3> + ClaimMark,
    E: FromParent<P, V4> + ClaimMark,
    F: FromParent<P, V5> + ClaimMark,
{
}

impl<P, V0, V1, V2, V3, V4, V5, V6, A, B, C, D, E, F, G> FromParent<P, (V0, V1, V2, V3, V4, V5, V6)>
    for (A, B, C, D, E, F, G)
where
    A: FromParent<P, V0> + ClaimMark,
    B: FromParent<P, V1> + ClaimMark,
    C: FromParent<P, V2> + ClaimMark,
    D: FromParent<P, V3> + ClaimMark,
    E: FromParent<P, V4> + ClaimMark,
    F: FromParent<P, V5> + ClaimMark,
    G: FromParent<P, V6> + ClaimMark,
{
}

impl<P, V0, V1, V2, V3, V4, V5, V6, V7, A, B, C, D, E, F, G, H>
    FromParent<P, (V0, V1, V2, V3, V4, V5, V6, V7)> for (A, B, C, D, E, F, G, H)
where
    A: FromParent<P, V0> + ClaimMark,
    B: FromParent<P, V1> + ClaimMark,
    C: FromParent<P, V2> + ClaimMark,
    D: FromParent<P, V3> + ClaimMark,
    E: FromParent<P, V4> + ClaimMark,
    F: FromParent<P, V5> + ClaimMark,
    G: FromParent<P, V6> + ClaimMark,
    H: FromParent<P, V7> + ClaimMark,
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
