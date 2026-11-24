//! Manual provide / inject against a parent surface.
//!
//! Membership is [`ClaimMark`] + list match. [`OfProvide`]`<N>` / [`OfInject`]`<N>`
//! are Via slots (E0119 residual for head vs tail) — not a Get×Mut matrix.
//! Prefer collapsing further if a non-overlapping membership encoding appears.
//!
//! Length-1 lists use the default (`OfProvide` / `OfInject`). Multi-item slots
//! use `OfProvide<0>`, `OfProvide<1>`, … (same for inject).

use crate::bean::BeanInfo;
use crate::descriptor::{Get, Mut};

/// Via code for a length-1 provide/inject list (default on [`OfProvide`] / [`OfInject`]).
const VIA_LEN1: usize = usize::MAX;

/// The child is injecting the parent bean itself.
pub struct OfParent;

/// Via into the parent's provide list.
///
/// Default = length-1 exact match. `V` = 0-based slot in a longer list.
/// Generics: `V` slot (default = length-1).
pub struct OfProvide<const V: usize = { VIA_LEN1 }>;

/// Via into the parent's inject list.
///
/// Default = length-1 exact match. `V` = 0-based slot in a longer list.
/// Generics: `V` slot (default = length-1).
pub struct OfInject<const V: usize = { VIA_LEN1 }>;

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

// Length-1 provide. Generics: `P` parent, `C` claim ([`ClaimMark`]).
impl<P, C> FromParent<P, OfProvide> for C
where
    P: BeanInfo<Provides = C>,
    C: ClaimMark,
{
}

// Length-1 inject. Generics: `P` parent, `C` claim ([`ClaimMark`]).
impl<P, C> FromParent<P, OfInject> for C
where
    P: BeanInfo<Injects = C>,
    C: ClaimMark,
{
}

// Two-item lists: slot Via (E0119 if head/tail shared one Via).
// Generics: `P` parent, `C` claim, `R` rest.
impl<P, C, R> FromParent<P, OfProvide<0>> for C
where
    P: BeanInfo<Provides = (C, R)>,
    C: ClaimMark,
{
}

// Generics: `P` parent, `C` claim, `L` left.
impl<P, C, L> FromParent<P, OfProvide<1>> for C
where
    P: BeanInfo<Provides = (L, C)>,
    C: ClaimMark,
{
}

// Generics: `P` parent, `C` claim, `R` rest.
impl<P, C, R> FromParent<P, OfInject<0>> for C
where
    P: BeanInfo<Injects = (C, R)>,
    C: ClaimMark,
{
}

// Generics: `P` parent, `C` claim, `L` left.
impl<P, C, L> FromParent<P, OfInject<1>> for C
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
