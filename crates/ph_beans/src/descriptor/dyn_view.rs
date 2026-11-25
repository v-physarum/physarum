//! View of a claimed bean as `&dyn Trait` on the **system argument** only.
//!
//! Call sites inject the concrete bean (`Draw::T`), like any other claim.
//! Under the hood this is ordinary unsizing: concrete value in, `&dyn Trait`
//! only on the callee parameter (`ArgsView`). The cast is declared by
//! `#[inject(Draw::T as Painter)]` / `Injects` encoding — not a method on
//! `Get` / `Mut` (design + rule 19).

use core::marker::PhantomData;

/// A trait object type that can be injected (`#[config(type = dynamic)]` on
/// the trait), and the same trait object for the borrow `'a`.
///
/// A system takes `&mut dyn Trait`, whose object lives as long as the
/// borrow. A generic type cannot name that on stable Rust, so the trait
/// object states it once (the derive of a `dynamic` trait writes it):
///
/// ```
/// use ph_beans::DynObject;
/// trait Painter {}
/// impl<'a> DynObject<'a> for dyn Painter {
///     type Obj = dyn Painter + 'a;
///     fn shared(&'a self) -> &'a Self::Obj { self }
///     fn exclusive(&'a mut self) -> &'a mut Self::Obj { self }
/// }
/// ```
/// Generics: `'a` the borrow.
pub trait DynObject<'a>: 'static {
    /// `dyn Trait + 'a`.
    type Obj: ?Sized + 'a;
    /// The object for the borrow `'a`.
    fn shared(&'a self) -> &'a Self::Obj;
    /// The object for the exclusive borrow `'a`.
    fn exclusive(&'a mut self) -> &'a mut Self::Obj;
}

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

// Generics: `C` context / claim / child, `V` view / via.
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
