//! Status of one claim list: no bean is named twice.
//!
//! A claim's [`Claim::Indexes`](crate::Claim::Indexes) names a slot of the
//! module, an access and a bean per leaf. [`Marks`] walks that tree over a
//! tuple of slot marks and reports the first clash as a
//! [`ClaimError`]. `.inject_resources` reads the status through
//! [`OpDesc`] and [`OpOk`](crate::OpOk), the one channel for operator status.

use crate::borrow::{Ix, Take};
use crate::descriptor::error::ClaimError;
use crate::descriptor::{Exclusive, JoinStatus, OpDesc, Shared, Valid};
use crate::query::claim::Claim;
use crate::query::slot::Pick;

/// No claim names this slot yet.
pub struct Free;

/// A claim names this slot with access `A`. Generics: `A` access.
pub struct Taken<A> {
    _markers: core::marker::PhantomData<A>,
}

/// The marks of a module before a claim list is read.
pub struct Fresh;

/// The tuple of marks [`Fresh`] stands for.
#[doc(hidden)]
pub trait Start {
    /// Every slot [`Free`]: one per bean a module can hold (the widest
    /// tuple [`Take`] reads).
    type Marks;
}

impl Start for Fresh {
    type Marks = (Free, Free, Free, Free);
}

/// One slot mark meets a claim with access `A` on bean `P`.
///
/// Generics: `A` access asked for, `P` bean.
#[doc(hidden)]
pub trait Meets<A, P> {
    /// The mark after the claim.
    type Mark;
    /// `Valid`, or the clash.
    type Status;
}

// Generics: `A` access, `P` bean.
impl<A, P> Meets<A, P> for Free {
    type Mark = Taken<A>;
    type Status = Valid;
}

// The same access twice is a duplicate. Generics: `A` access, `P` bean.
impl<A, P> Meets<A, P> for Taken<A> {
    type Mark = Self;
    type Status = ClaimError<P, 1>;
}

// Shared after exclusive is a conflict. Generics: `P` bean.
impl<P> Meets<Shared, P> for Taken<Exclusive> {
    type Mark = Self;
    type Status = ClaimError<P, 2>;
}

// Exclusive after shared is a conflict. Generics: `P` bean.
impl<P> Meets<Exclusive, P> for Taken<Shared> {
    type Mark = Self;
    type Status = ClaimError<P, 2>;
}

/// An index tree read over the slot marks `S`.
///
/// Generics: `S` tuple of slot marks.
#[doc(hidden)]
pub trait Marks<S> {
    /// The marks after this tree.
    type Next;
    /// `Valid`, or the first clash.
    type Status;
}

// Generics: `S` marks.
impl<S> Marks<S> for () {
    type Next = S;
    type Status = Valid;
}

// A claim on a `Dyn` entry names a bare slot and is not checked here.
// Generics: `S` marks, `N` slot.
impl<S, const N: usize> Marks<S> for Ix<N> {
    type Next = S;
    type Status = Valid;
}

// Generics: `S` marks, `K` part, `N` slot, `A` access, `P` bean, `M` the
// mark in that slot.
impl<S, K, A, P, M, const N: usize> Marks<S> for Pick<K, Ix<N>, A, P>
where
    S: Take<Ix<N>, Out = M>,
    M: Meets<A, P>,
{
    type Next = S::With<M::Mark>;
    type Status = M::Status;
}

// Generics: `S` marks, `X` left tree, `Y` right tree.
impl<S, X, Y> Marks<S> for (X, Y)
where
    X: Marks<S>,
    Y: Marks<X::Next>,
    X::Status: JoinStatus<Y::Status>,
{
    type Next = Y::Next;
    type Status = <X::Status as JoinStatus<Y::Status>>::Out;
}

// The status of a claim list, read over the module's slots.
// Generics: `B` module, `R` claim, `I` its index tree.
impl<B, R, I> OpDesc<B> for R
where
    R: Claim<B, Indexes = I>,
    I: Marks<<Fresh as Start>::Marks>,
{
    type Status = I::Status;
}
