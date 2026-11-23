//! View of a claimed bean as `&dyn Trait`.

use core::marker::PhantomData;

use crate::descriptor::{Get, Mut};

/// Marker: `V` may view bean `T`.
///
/// `dyn Trait` implements this only when the trait is marked for dyn inject.
/// The claimed bean stays `T`. [`Get`] and [`Mut`] stay one type parameter.
/// The init borrow is `&V` or `&mut V`.
pub trait DynView<T: ?Sized> {}

/// Claim `C` viewed as `V`.
///
/// `C` is [`Get<T>`](Get) or [`Mut<T>`](Mut). `V` is `dyn Trait`.
/// The bean key stays `T`.
pub struct Dyn<C, V: ?Sized> {
    #[allow(clippy::type_complexity)]
    _markers: PhantomData<(C, fn() -> *const V)>,
}

impl<C, V: ?Sized> Dyn<C, V> {
    /// Marker for claim `C` viewed as `V`.
    #[must_use]
    pub const fn new() -> Self {
        Self { _markers: PhantomData }
    }
}

impl<C, V: ?Sized> Default for Dyn<C, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Get<T> {
    /// This claim, viewed as `V`.
    ///
    /// `V` is `dyn Trait` when that trait object implements [`DynView`].
    /// Provides stay concrete: the cast is an inject view, not a new bean.
    ///
    /// ```compile_fail
    /// use ph_beans::Get;
    ///
    /// trait Other {
    ///     fn go(&self);
    /// }
    /// struct Draw;
    /// impl Other for Draw {
    ///     fn go(&self) {}
    /// }
    ///
    /// let _claim = Get::<Draw>::new().as_dyn::<dyn Other>();
    /// ```
    #[must_use]
    pub const fn as_dyn<V>(self) -> Dyn<Self, V>
    where
        V: DynView<T> + ?Sized,
    {
        let _ = self;
        Dyn::new()
    }
}

impl<T> Mut<T> {
    /// This claim, viewed as `V`.
    ///
    /// `V` is `dyn Trait` when that trait object implements [`DynView`].
    /// The exclusive view is `&mut V`.
    #[must_use]
    pub const fn as_dyn<V>(self) -> Dyn<Self, V>
    where
        V: DynView<T> + ?Sized,
    {
        let _ = self;
        Dyn::new()
    }
}
