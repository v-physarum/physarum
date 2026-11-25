//! Predicate and event nodes. `&`, `|`, and `!` build them.

use core::marker::PhantomData;
use core::ops::Index;

/// `A & B`.
/// Generics: `A` left child, `B` right child.
pub struct And<A, B> {
    _markers: PhantomData<(A, B)>,
}

// Generics: `A` left / child, `B` right child.
impl<A, B> Copy for And<A, B> {}

// Generics: `A` left / child, `B` right child.
impl<A, B> Clone for And<A, B> {
    fn clone(&self) -> Self {
        *self
    }
}

// Generics: `A` left / child, `B` right child.
impl<A, B> Default for And<A, B> {
    fn default() -> Self {
        Self::new()
    }
}

// Generics: `A` left / child, `B` right.
impl<A, B> And<A, B> {
    /// Builds `A & B`.
    #[must_use]
    pub const fn new() -> Self {
        Self { _markers: PhantomData }
    }
}

/// `A | B`.
/// Generics: `A` left child, `B` right child.
pub struct Or<A, B> {
    _markers: PhantomData<(A, B)>,
}

// Generics: `A` left / child, `B` right child.
impl<A, B> Copy for Or<A, B> {}

// Generics: `A` left / child, `B` right child.
impl<A, B> Clone for Or<A, B> {
    fn clone(&self) -> Self {
        *self
    }
}

// Generics: `A` left / child, `B` right child.
impl<A, B> Default for Or<A, B> {
    fn default() -> Self {
        Self::new()
    }
}

// Generics: `A` left / child, `B` right.
impl<A, B> Or<A, B> {
    /// Builds `A | B`.
    #[must_use]
    pub const fn new() -> Self {
        Self { _markers: PhantomData }
    }
}

/// `!A`.
/// Generics: `A` child.
pub struct Not<A> {
    _markers: PhantomData<A>,
}

// Generics: `A` left / child.
impl<A> Copy for Not<A> {}

// Generics: `A` left / child.
impl<A> Clone for Not<A> {
    fn clone(&self) -> Self {
        *self
    }
}

// Generics: `A` left / child.
impl<A> Default for Not<A> {
    fn default() -> Self {
        Self::new()
    }
}

// Generics: `A` left / child.
impl<A> Not<A> {
    /// Builds `!A`.
    #[must_use]
    pub const fn new() -> Self {
        Self { _markers: PhantomData }
    }
}

/// `E::ON[I]`.
/// Generics: `E` event, `I` inner path.
pub struct On<E, I> {
    _markers: PhantomData<(E, I)>,
}

// Generics: `E` event, `I` inner path.
impl<E, I> Copy for On<E, I> {}

// Generics: `E` event, `I` inner path.
impl<E, I> Clone for On<E, I> {
    fn clone(&self) -> Self {
        *self
    }
}

// Generics: `E` event, `I` inner path.
impl<E, I> Default for On<E, I> {
    fn default() -> Self {
        Self::new()
    }
}

// Generics: `E` event, `I` inner.
impl<E, I> On<E, I> {
    /// Builds one event node.
    #[must_use]
    pub const fn new() -> Self {
        Self { _markers: PhantomData }
    }
}

// Generics: `E` event, `I` inner path.
impl<E, I> Index<I> for On<E, ()> {
    type Output = On<E, I>;

    fn index(&self, index: I) -> &Self::Output {
        let _ = (self, index);
        const { &On::new() }
    }
}
