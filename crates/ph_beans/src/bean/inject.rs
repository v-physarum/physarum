//! Manual provide / inject against a parent surface.
//!
//! Membership is [`Provided`] (bean and access of an entry) + list match. [`OfProvide`]`<N>` / [`OfInject`]`<N>`
//! are Via slots (E0119 residual for head vs tail) — not a Get×Mut matrix.
//! Prefer collapsing further if a non-overlapping membership encoding appears.
//!
//! `OfProvide<N>` is slot `N` of the provide list, for a list of any length;
//! the default is slot 0. `Provides` is always a list, so one provide is
//! `(C,)`. One inject is the bare claim `C` (`OfInject`); a pair of injects
//! uses `OfInject<0>` / `OfInject<1>`.

use crate::bean::BeanInfo;
use crate::borrow::{Ix, Take};
use crate::descriptor::{Allows, Named, Provided};

/// The child is injecting the parent bean itself.
pub struct OfParent;

/// Via into the parent's provide list.
///
/// `V` is the 0-based slot in the list. The default is slot 0: the one item
/// of a length-1 list `(C,)`. Generics: `V` slot.
pub struct OfProvide<const V: usize = 0>;

/// Via into the parent's inject list.
///
/// Default = the bare claim of a length-1 list ([`BARE`](OfInject::BARE)).
/// `V` = 0-based slot of a pair. Generics: `V` slot.
pub struct OfInject<const V: usize = { OfInject::<0>::BARE }>;

impl OfInject<0> {
    /// Via code of a length-1 inject list: no slot of a pair has it.
    pub const BARE: usize = usize::MAX;
}

/// `Self` may be injected from `P` via `V`. Generics: `P` parent, `V` via.
/// ```compile_fail
/// use ph_beans::{BeanInfo, FromParent, Get, Mut, OfProvide};
/// struct Logic; struct App;
/// impl BeanInfo for Logic { type Provides=(); type Injects=(); type Beans=(); type State=ph_beans::NoState; }
/// impl BeanInfo for App { type Provides=(Get<Logic>,); type Injects=(); type Beans=(Get<Logic>,); type State=ph_beans::NoState; }
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

// The claim is the parent bean itself, under a marker.
// Generics: `P` parent, `C` claim ([`Provided`]).
impl<P, C> FromParent<P, OfParent> for C where C: Provided<Bean = P, Access: Named> {}

// Generics: `P` parent.
impl<P> FromParent<P, ()> for () {}

/// Claim `Self` can be taken from entry `E` of a parent list: the same bean,
/// and the entry's access is enough ([`Allows`]).
///
/// A shared claim is taken from any entry of that bean. An exclusive claim
/// needs an exclusive entry. Generics: `E` entry ([`Provided`]).
trait TakenFrom<E> {}

// Generics: `C` child claim, `E` parent entry.
impl<C, E> TakenFrom<E> for C
where
    C: Provided<Access: Named>,
    E: Provided<Bean = C::Bean, Access: Allows<C::Access>>,
{
}

// Any slot of the provide list. `Provides` is always a list, so one provide
// is slot 0 of `(E,)`. Generics: `P` parent, `C` claim, `E` entry, `N` slot.
impl<P, C, E, const N: usize> FromParent<P, OfProvide<N>> for C
where
    P: BeanInfo<Provides: Take<Ix<N>, Out = E>>,
    C: TakenFrom<E>,
{
}

// Length-1 inject: the bare entry. Generics: `P` parent, `C` claim, `E` entry.
impl<P, C, E> FromParent<P, OfInject> for C
where
    P: BeanInfo<Injects = E>,
    C: TakenFrom<E>,
{
}

// Two-item inject lists: slot Via (E0119 if head/tail shared one Via).
// Generics: `P` parent, `C` claim, `E` entry, `R` rest.
impl<P, C, E, R> FromParent<P, OfInject<0>> for C
where
    P: BeanInfo<Injects = (E, R)>,
    C: TakenFrom<E>,
{
}

// Generics: `P` parent, `C` claim, `E` entry, `L` left.
impl<P, C, E, L> FromParent<P, OfInject<1>> for C
where
    P: BeanInfo<Injects = (L, E)>,
    C: TakenFrom<E>,
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
