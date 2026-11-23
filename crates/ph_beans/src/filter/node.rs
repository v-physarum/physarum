//! Predicate and event nodes. `&`, `|`, and `!` build them.

use core::marker::PhantomData;
use core::ops::Index;

/// `A & B`.
pub struct And<A, B>(PhantomData<(A, B)>);

impl<A, B> Copy for And<A, B> {}

impl<A, B> Clone for And<A, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, B> Default for And<A, B> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A, B> And<A, B> {
    /// Builds `A & B`.
    #[must_use]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

/// `A | B`.
pub struct Or<A, B>(PhantomData<(A, B)>);

impl<A, B> Copy for Or<A, B> {}

impl<A, B> Clone for Or<A, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, B> Default for Or<A, B> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A, B> Or<A, B> {
    /// Builds `A | B`.
    #[must_use]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

/// `!A`.
pub struct Not<A>(PhantomData<A>);

impl<A> Copy for Not<A> {}

impl<A> Clone for Not<A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A> Default for Not<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A> Not<A> {
    /// Builds `!A`.
    #[must_use]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

/// `E::ON[I]`.
pub struct On<E, I>(PhantomData<(E, I)>);

impl<E, I> Copy for On<E, I> {}

impl<E, I> Clone for On<E, I> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<E, I> Default for On<E, I> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E, I> On<E, I> {
    /// Builds one event node.
    #[must_use]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<E, I> Index<I> for On<E, ()> {
    type Output = On<E, I>;

    fn index(&self, index: I) -> &Self::Output {
        let _ = (self, index);
        const { &On::new() }
    }
}
