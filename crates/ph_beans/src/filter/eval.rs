//! Match of a filter against the emitted event mask.
//!
//! Build-time: each filter/emit AST folds to [`MatchEmit::NEED`] /
//! [`Collect::MASK`]. Const leaves use [`MatchEmit::Against`]`<M>::MATCHES`.
//! Runtime [`MatchEmit::matches`] still takes a live [`EmitMask`].
//!
//! **Bit registry:** each event implements [`EmitBit`] with a unique `BIT` in
//! `0..EmitMask::BITS`. Manual today; derive may emit it later (not on
//! [`EventInfo`](crate::EventInfo) — rule 14).

use core::marker::PhantomData;

use crate::filter::ops::FilterOperand;
use crate::filter::shape::{Here, IdleEmit, JoinBoth, JoinEither, NodeForm, PairBody, Skip};
use crate::filter::{And, On};

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
    pub const fn from_bit(bit: u32) -> Self {
        assert!(bit < Self::BITS, "EmitBit::BIT must be < EmitMask::BITS");
        Self(1u128 << bit)
    }

    /// Bits set in `self` or `other`.
    pub const fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether `self` and `other` are the same bit set.
    #[must_use]
    pub const fn same(self, other: Self) -> bool {
        self.0 == other.0
    }

    /// Whether every bit of `need` is set in `self`.
    #[must_use]
    pub const fn contains(self, need: Self) -> bool {
        self.0 & need.0 == need.0
    }
}

/// Event bit for the emit mask. Separate from [`EventInfo`](crate::EventInfo).
///
/// Assign unique `BIT` values across the app (registry by convention).
pub trait EmitBit {
    /// Bit index in [`EmitMask`] (`0..EmitMask::BITS`).
    const BIT: u32;
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

impl<A> AsMask for Skip<A> {
    const MASK: EmitMask = EmitMask::EMPTY;
}

// Generics: `E` event, `I` inner path.
impl<E, I> AsMask for Here<E, I>
where
    E: EmitBit,
    I: Collect,
{
    const MASK: EmitMask = EmitMask::from_bit(E::BIT).or(I::MASK);
}

// Generics: `T` operator ([`NodeForm`]).
impl<T> Collect for T
where
    T: NodeForm,
    T::Body: AsMask,
{
    const MASK: EmitMask = <T::Body as AsMask>::MASK;
}

/// Const bool for [`MatchEmit::Against`].
#[doc(hidden)]
pub trait ConstMatch {
    /// Whether the filter passes for that emit tree.
    const MATCHES: bool;
}

/// Always matches (no emit bits required).
#[doc(hidden)]
pub struct AlwaysMatch;

impl ConstMatch for AlwaysMatch {
    const MATCHES: bool = true;
}

/// Const match of `On<E, I>` against emit tree `M`.
/// Generics: `E` event, `I` inner, `M` emit mask const.
#[doc(hidden)]
pub struct OnMatch<E, I, M>(PhantomData<(E, I, M)>);

impl<E, I, M> ConstMatch for OnMatch<E, I, M>
where
    E: EmitBit,
    I: MatchEmit,
    M: Collect,
{
    const MATCHES: bool =
        M::MASK.contains(<On<E, I> as MatchEmit>::NEED) && I::Against::<M>::MATCHES;
}

/// Const match of `And<A, B>` against emit tree `M`.
/// Generics: `A` left, `B` right, `M` emit mask const.
#[doc(hidden)]
pub struct AndMatch<A, B, M>(PhantomData<(A, B, M)>);

impl<A, B, M> ConstMatch for AndMatch<A, B, M>
where
    A: MatchEmit,
    B: MatchEmit,
    M: Collect,
{
    const MATCHES: bool = A::Against::<M>::MATCHES && B::Against::<M>::MATCHES;
}

/// Const match of `Or<A, B>` against emit tree `M`.
/// Generics: `A` left, `B` right, `M` emit mask const.
#[doc(hidden)]
pub struct OrMatch<A, B, M>(PhantomData<(A, B, M)>);

impl<A, B, M> ConstMatch for OrMatch<A, B, M>
where
    A: MatchEmit,
    B: MatchEmit,
    M: Collect,
{
    const MATCHES: bool = A::Against::<M>::MATCHES || B::Against::<M>::MATCHES;
}

/// Const match of `Not<A>` against emit tree `M`.
/// Generics: `A` left, `M` emit mask const.
#[doc(hidden)]
pub struct NotMatch<A, M>(PhantomData<(A, M)>);

impl<A, M> ConstMatch for NotMatch<A, M>
where
    A: MatchEmit,
    M: Collect,
{
    const MATCHES: bool = !A::Against::<M>::MATCHES;
}

/// Filter vs emit mask. `NEED` is the And-of-`On` bit set when that applies.
///
/// [`Against`](MatchEmit::Against) is the GAT that carries a const
/// [`MATCHES`](ConstMatch::MATCHES) for emit tree `M` (stable: no `const` trait
/// methods). Drive / path leaves use `const { F::Against::<E>::MATCHES }`.
#[doc(hidden)]
pub trait MatchEmit {
    /// Bits that must all be set for a pure And-of-`On` path ([`EmitMask::EMPTY`] if unused).
    const NEED: EmitMask;
    /// Const match result against emit tree `M`.
    type Against<M: Collect>: ConstMatch;
    /// Whether this filter passes for `mask` (runtime).
    fn matches(mask: EmitMask) -> bool;
}

/// [`MatchEmit`] of a [`NodeForm::Body`](crate::filter::shape::NodeForm::Body).
#[doc(hidden)]
pub trait AsEmit {
    /// Bits that must all be set.
    const NEED: EmitMask;
    /// Const match against emit tree `M`.
    type Against<M: Collect>: ConstMatch;
    /// Whether this body passes for `mask`.
    fn matches(mask: EmitMask) -> bool;
}

// Generics: `T` tick / type / tip.
impl<T: IdleEmit> AsEmit for T {
    const NEED: EmitMask = EmitMask::EMPTY;
    type Against<M: Collect> = AlwaysMatch;

    fn matches(_mask: EmitMask) -> bool {
        true
    }
}

// Generics: `A` left child, `B` right child — And body (`JoinBoth`).
impl<A, B> AsEmit for PairBody<A, B, JoinBoth>
where
    A: MatchEmit,
    B: MatchEmit,
{
    const NEED: EmitMask = A::NEED.or(B::NEED);
    type Against<M: Collect> = AndMatch<A, B, M>;

    fn matches(mask: EmitMask) -> bool {
        A::matches(mask) && B::matches(mask)
    }
}

// Generics: `A` left child, `B` right child — Or body (`JoinEither`).
impl<A, B> AsEmit for PairBody<A, B, JoinEither>
where
    A: MatchEmit,
    B: MatchEmit,
{
    const NEED: EmitMask = EmitMask::EMPTY;
    type Against<M: Collect> = OrMatch<A, B, M>;

    fn matches(mask: EmitMask) -> bool {
        A::matches(mask) || B::matches(mask)
    }
}

// Generics: `A` child.
impl<A: MatchEmit> AsEmit for Skip<A> {
    const NEED: EmitMask = EmitMask::EMPTY;
    type Against<M: Collect> = NotMatch<A, M>;

    fn matches(mask: EmitMask) -> bool {
        !A::matches(mask)
    }
}

// Generics: `E` event, `I` inner path.
impl<E, I> AsEmit for Here<E, I>
where
    E: EmitBit,
    I: MatchEmit,
{
    const NEED: EmitMask = EmitMask::from_bit(E::BIT).or(I::NEED);
    type Against<M: Collect> = OnMatch<E, I, M>;

    fn matches(mask: EmitMask) -> bool {
        mask.contains(Self::NEED) && I::matches(mask)
    }
}

// Generics: `T` operator ([`NodeForm`]).
impl<T> MatchEmit for T
where
    T: NodeForm,
    T::Body: AsEmit,
{
    const NEED: EmitMask = <T::Body as AsEmit>::NEED;
    type Against<M: Collect> = <T::Body as AsEmit>::Against<M>;

    fn matches(mask: EmitMask) -> bool {
        <T::Body as AsEmit>::matches(mask)
    }
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
