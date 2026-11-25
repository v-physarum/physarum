//! Match of a filter against the emitted event mask.
//!
//! Build-time: each filter/emit AST folds to [`MatchEmit::NEED`] /
//! [`Collect::MASK`]. Const leaves use [`MatchEmit::Against`]`<M>::MATCHES`.
//! Runtime [`MatchEmit::matches`] still takes a live [`EmitMask`].
//!
//! **Bit registry:** each event implements [`EmitBit`] with one
//! [`Bit`](crate::Bit) slot of a registry. The compiler rejects a slot with
//! two owners. Not on [`EventInfo`](crate::EventInfo) — rule 14.

use core::marker::PhantomData;

use crate::filter::On;
pub use crate::filter::mask::{AsMask, Collect, EmitMask, UnionEmit};
use crate::filter::shape::{Here, IdleEmit, JoinBoth, JoinEither, NodeForm, PairBody, Skip};
use crate::filter::slot::{EmitBit, EmitIx};

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
pub struct OnMatch<E, I, M> {
    _markers: PhantomData<(E, I, M)>,
}

// Generics: `E` event, `I` inner, `M` emit mask const.
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
pub struct AndMatch<A, B, M> {
    _markers: PhantomData<(A, B, M)>,
}

// Generics: `A` left, `B` right, `M` emit mask const.
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
pub struct OrMatch<A, B, M> {
    _markers: PhantomData<(A, B, M)>,
}

// Generics: `A` left, `B` right, `M` emit mask const.
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
pub struct NotMatch<A, M> {
    _markers: PhantomData<(A, M)>,
}

// Generics: `A` left, `M` emit mask const.
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
    const NEED: EmitMask = EmitMask::from_bit(<E as EmitIx>::BIT).or(I::NEED);
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
