//! View of a claimed bean as `&dyn Trait` on the **system argument** only.
//!
//! Call sites inject the concrete bean (`Draw::T`), like any other claim.
//! Under the hood this is ordinary unsizing: concrete value in, `&dyn Trait`
//! only on the callee parameter (`ArgsView`). The cast is declared by
//! `#[inject(Draw::T as Painter)]` / `Injects` encoding — not a method on
//! `Get` / `Mut` (design + rule 19).

use core::marker::PhantomData;

/// Marker: `V` may view bean `T`.
///
/// `dyn Trait` implements this only when the trait is marked for dyn inject
/// (`#[config(type = dyn)]`). The claimed bean stays `T`.
/// Generics: `T` concrete bean.
pub trait DynView<T: ?Sized> {}

/// Type-level claim `C` viewed as `V` (derive / `Injects` encoding).
///
/// Users do **not** construct this at `.inject_resources` — they pass
/// `Get` / `Mut` for the concrete bean. `C` is `Get<T>` or `Mut<T>`;
/// `V` is `dyn Trait`.
pub struct Dyn<C, V: ?Sized> {
    // One field (rule 15). The fn pointer keeps `V` covariant.
    #[allow(clippy::type_complexity)]
    _markers: PhantomData<(C, fn() -> *const V)>,
}

impl<C, V: ?Sized> Dyn<C, V> {
    /// Type-level marker (not an inject call-site API).
    #[must_use]
    pub const fn new() -> Self {
        Self { _markers: PhantomData }
    }
}

// Generics: `C` context / claim / child, `V` view / via.
impl<C, V: ?Sized> Default for Dyn<C, V> {
    fn default() -> Self {
        Self::new()
    }
}
