//! [`EmitMask`]: one bit per event, and the fold of an emit tree to a mask
//! ([`Collect`]).

use crate::filter::And;
use crate::filter::ops::FilterOperand;
use crate::filter::shape::{Here, IdleEmit, NodeForm, PairBody, Skip};
use crate::filter::slot::{EmitBit, EmitIx, InReg};

/// Tick emit set: one bit per [`EmitBit`] event.
///
/// Not a raw `u128` and not a type alias (rules 21–22). Bits stay inside the
/// newtype; callers use [`Self::from_bit`], [`Self::or`], [`Self::contains`].
#[must_use]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EmitMask(u128);

impl EmitMask {
    /// Bits available in this mask.
    pub const BITS: u32 = 128;

    /// No events set.
    pub const EMPTY: Self = Self(0);

    /// Single-bit mask for `bit` (`0..BITS`).
    ///
    /// # Panics
    ///
    /// Const-eval panics when `bit >= BITS`.
    #[inline]
    pub const fn from_bit(bit: u32) -> Self {
        assert!(bit < Self::BITS, "Bit<N> must have N < EmitMask::BITS");
        Self(1u128 << bit)
    }

    /// Bits set in `self` or `other`.
    #[inline]
    pub const fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether `self` and `other` are the same bit set.
    #[must_use]
    #[inline]
    pub const fn same(self, other: Self) -> bool {
        self.0 == other.0
    }

    /// Whether every bit of `need` is set in `self`.
    #[must_use]
    #[inline]
    pub const fn contains(self, need: Self) -> bool {
        self.0 & need.0 == need.0
    }

    /// `self` plus the bit of an outer event on the same path.
    ///
    /// # Panics
    ///
    /// Const-eval panics when the inner path already holds `bit`.
    #[inline]
    pub const fn under(self, bit: u32) -> Self {
        let outer = Self::from_bit(bit);
        assert!(self.0 & outer.0 == 0, "two events on one path share a bit");
        self.or(outer)
    }
}

/// Folds an emit AST to a mask (build-time const).
#[doc(hidden)]
pub trait Collect {
    /// Bits set by this emit tree.
    const MASK: EmitMask;
}

/// [`Collect::MASK`] of a [`NodeForm::Body`](crate::filter::shape::NodeForm::Body).
#[doc(hidden)]
pub trait AsMask {
    /// Bits this body contributes.
    const MASK: EmitMask;
}

// Generics: `T` tick / type / tip.
impl<T: IdleEmit> AsMask for T {
    const MASK: EmitMask = EmitMask::EMPTY;
}

// Generics: `A` left child, `B` right child, `K` join kind — one MASK fold for And / Or.
impl<A, B, K> AsMask for PairBody<A, B, K>
where
    A: Collect,
    B: Collect,
{
    const MASK: EmitMask = A::MASK.or(B::MASK);
}

// Generics: `A` left child.
impl<A> AsMask for Skip<A> {
    const MASK: EmitMask = EmitMask::EMPTY;
}

// Generics: `E` event, `I` inner path.
impl<E, I> AsMask for Here<E, I>
where
    E: EmitBit,
    I: Collect + InReg<E::Reg>,
{
    const MASK: EmitMask = I::MASK.under(<E as EmitIx>::BIT);
}

// Generics: `T` operator ([`NodeForm`]).
impl<T> Collect for T
where
    T: NodeForm,
    T::Body: AsMask,
{
    const MASK: EmitMask = <T::Body as AsMask>::MASK;
}

/// Joins a second `.emit` onto the set already stored.
///
/// Empty `()` becomes `N`. An existing [`FilterOperand`] tree becomes `And<Self, N>`.
/// Generics: `N` next emit.
#[doc(hidden)]
pub trait UnionEmit<N> {
    /// Combined emit tree.
    type Out;
}

// Generics: `N` new emit tree. Empty slot is replaced, not wrapped.
impl<N> UnionEmit<N> for () {
    type Out = N;
}

// Generics: `T` stored tree ([`FilterOperand`]), `N` new emit tree.
impl<T, N> UnionEmit<N> for T
where
    T: FilterOperand,
{
    type Out = And<Self, N>;
}
