//! Manual provide / inject against a parent surface.
//!
//! A child names where the claim comes from: the parent bean, the parent's
//! provide, or the parent's inject. `MUT` against an immutable provide does
//! not implement [`FromParent`].

use crate::bean::BeanInfo;
use crate::descriptor::{Get, Mut};

/// The child is injecting the parent bean itself.
pub struct OfParent;

/// The child is injecting the parent's single provide.
pub struct OfProvide;

/// The child is injecting the parent's single inject.
pub struct OfInject;

/// First item of a two-item provide list.
pub struct OfProvide0;

/// Second item of a two-item provide list.
pub struct OfProvide1;

/// First item of a two-item inject list.
pub struct OfInject0;

/// Second item of a two-item inject list.
pub struct OfInject1;

/// `Self` may be injected from `P` via `V`.
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
trait ClaimMark {}

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

// A list search with one `V` does not fit stable Rust: `Provides = (C, R)`
// and `Provides = (H, C)` are the same `FromParent<P, OfProvide> for C` and
// overlap (`E0119`). Slot markers keep those impls apart. Each slot is one
// impl for every [`ClaimMark`], not a separate `Get` and `Mut` line.
impl<P, C, R> FromParent<P, OfProvide0> for C
where
    P: BeanInfo<Provides = (C, R)>,
    C: ClaimMark,
{
}

impl<P, C, L> FromParent<P, OfProvide1> for C
where
    P: BeanInfo<Provides = (L, C)>,
    C: ClaimMark,
{
}

impl<P, C, R> FromParent<P, OfInject0> for C
where
    P: BeanInfo<Injects = (C, R)>,
    C: ClaimMark,
{
}

impl<P, C, L> FromParent<P, OfInject1> for C
where
    P: BeanInfo<Injects = (L, C)>,
    C: ClaimMark,
{
}

impl<P, V0, V1, A, B> FromParent<P, (V0, V1)> for (A, B)
where
    A: FromParent<P, V0>,
    B: FromParent<P, V1>,
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
