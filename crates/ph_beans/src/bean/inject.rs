//! Manual provide / inject against a parent surface.
//!
//! Membership is [`ClaimMark`] + list match. Slot markers (`OfProvide0`, …)
//! exist only because two `Provides = (C, _)` / `Provides = (_, C)` blankets
//! overlap (E0119) — not a Get×Mut matrix. Prefer collapsing further if a
//! non-overlapping membership encoding appears; do not add Get/Mut arms.

use crate::bean::BeanInfo;
use crate::descriptor::{Get, Mut};

/// The child is injecting the parent bean itself.
pub struct OfParent;

/// The child is injecting the parent's single provide.
pub struct OfProvide;

/// The child is injecting the parent's single inject.
pub struct OfInject;

/// First item of a two-item provide list.
///
/// Residual Via for E0119 — not an accessor×slot product.
pub struct OfProvide0;

/// Second item of a two-item provide list.
pub struct OfProvide1;

/// First item of a two-item inject list.
pub struct OfInject0;

/// Second item of a two-item inject list.
pub struct OfInject1;

/// `Self` may be injected from `P` via `V`. Generics: `P` parent, `V` via.
/// ```compile_fail
/// use ph_beans::{BeanInfo, FromParent, Get, Mut, OfProvide};
/// struct Logic; struct App;
/// impl BeanInfo for Logic { type Provides=(); type Injects=(); type Beans=(); type State=(); }
/// impl BeanInfo for App { type Provides=Get<Logic>; type Injects=(); type Beans=Get<Logic>; type State=(); }
/// fn mount<C: FromParent<App, OfProvide>>() {}
/// mount::<Mut<Logic>>();
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not on `{P}` via `{V}`",
    label = "fix Descriptor::Status",
    note = "a child may inject the parent, or the parent's injects or provides; MUT against an immutable provide is rejected"
)]
// Generics: `P` parent, `V` via.
pub trait FromParent<P, V> {}

/// `Get` or `Mut`. One bound covers both marker kinds.
trait ClaimMark {}

// Generics: `T` claimed bean.
impl<T> ClaimMark for Get<T> {}

// Generics: `T` claimed bean.
impl<T> ClaimMark for Mut<T> {}

/// The claim is the parent bean itself.
trait IsParent<P> {}

// Generics: `P` parent bean.
impl<P> IsParent<P> for Get<P> {}

// Generics: `P` parent bean.
impl<P> IsParent<P> for Mut<P> {}

// Generics: `P` parent, `C` claim.
impl<P, C> FromParent<P, OfParent> for C where C: IsParent<P> {}

// Generics: `P` parent.
impl<P> FromParent<P, ()> for () {}

// Generics: `P` parent, `C` claim ([`ClaimMark`]).
impl<P, C> FromParent<P, OfProvide> for C
where
    P: BeanInfo<Provides = C>,
    C: ClaimMark,
{
}

// Generics: `P` parent, `C` claim ([`ClaimMark`]).
impl<P, C> FromParent<P, OfInject> for C
where
    P: BeanInfo<Injects = C>,
    C: ClaimMark,
{
}

// Two-item lists: Via distinguishes head vs tail (E0119 if both use OfProvide).
// Generics: `P` parent, `C` claim, `R` rest.
impl<P, C, R> FromParent<P, OfProvide0> for C
where
    P: BeanInfo<Provides = (C, R)>,
    C: ClaimMark,
{
}

// Generics: `P` parent, `C` claim, `L` left.
impl<P, C, L> FromParent<P, OfProvide1> for C
where
    P: BeanInfo<Provides = (L, C)>,
    C: ClaimMark,
{
}

// Generics: `P` parent, `C` claim, `R` rest.
impl<P, C, R> FromParent<P, OfInject0> for C
where
    P: BeanInfo<Injects = (C, R)>,
    C: ClaimMark,
{
}

// Generics: `P` parent, `C` claim, `L` left.
impl<P, C, L> FromParent<P, OfInject1> for C
where
    P: BeanInfo<Injects = (L, C)>,
    C: ClaimMark,
{
}

// Generics: `P` parent, `V0`/`V1` via, `A`/`B` claims.
impl<P, V0, V1, A, B> FromParent<P, (V0, V1)> for (A, B)
where
    A: FromParent<P, V0>,
    B: FromParent<P, V1>,
{
}

/// Child whose inject list is on `P`'s surface through `V`.
/// Generics: `P` parent, `V` via.
pub trait Module<P, V>: BeanInfo {}

// Generics: `C` child, `P` parent, `V` via.
impl<C, P, V> Module<P, V> for C
where
    C: BeanInfo,
    C::Injects: FromParent<P, V>,
{
}
