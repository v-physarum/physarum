//! `&`, `|`, and `!` build filter nodes.
//!
//! RHS is always [`FilterOperand`] (one bound — not a RHS matrix). Each local
//! AST node needs its own `BitAnd` / `BitOr` / `Not`: a blanket
//! `impl<L: FilterOperand> BitAnd<R> for L` hits the orphan rule (E0210).
//! Prefer that residual over any Get×Mut×node product table. Rule 6: no
//! `macro_rules!` here to “save” the arms.

use core::ops::BitAnd;
use core::ops::BitOr;
use core::ops::Not as OpsNot;

use super::shape::{Here, NodeForm, PairBody, Read, Skip, Yes};
use super::{And, Not, On, Or};
use crate::descriptor::{Get, Mut};

/// A node that can sit on the right of `&` or `|`, or under `!`.
pub trait FilterOperand: Sized {}

/// [`NodeForm::Body`](crate::filter::shape::NodeForm::Body) that may sit on the right of `&` / `|`.
trait AsOperand {}

impl AsOperand for Read {}

impl AsOperand for Yes {}

// Generics: `A` left child, `B` right child, `K` join kind.
impl<A, B, K> AsOperand for PairBody<A, B, K> {}

// Generics: `A` child.
impl<A> AsOperand for Skip<A> {}

// Generics: `E` event, `I` inner path.
impl<E, I> AsOperand for Here<E, I> {}

// Generics: `T` operator ([`NodeForm`](crate::filter::shape::NodeForm)).
impl<T> FilterOperand for T
where
    T: NodeForm,
    T::Body: AsOperand,
{
}

impl<T, R> BitAnd<R> for Get<T>
where
    R: FilterOperand,
{
    type Output = And<Self, R>;

    fn bitand(self, _rhs: R) -> Self::Output {
        And::new()
    }
}

// Generics: `T` claimed type, `R` RHS operand.
impl<T, R> BitOr<R> for Get<T>
where
    R: FilterOperand,
{
    type Output = Or<Self, R>;

    fn bitor(self, _rhs: R) -> Self::Output {
        Or::new()
    }
}

impl<T> OpsNot for Get<T> {
    type Output = Not<Self>;

    fn not(self) -> Self::Output {
        Not::new()
    }
}

// Generics: `T` claimed type, `R` RHS operand.
impl<T, R> BitAnd<R> for Mut<T>
where
    R: FilterOperand,
{
    type Output = And<Self, R>;

    fn bitand(self, _rhs: R) -> Self::Output {
        And::new()
    }
}

impl<T, R> BitOr<R> for Mut<T>
where
    R: FilterOperand,
{
    type Output = Or<Self, R>;

    fn bitor(self, _rhs: R) -> Self::Output {
        Or::new()
    }
}

// Generics: `T` claimed type.
impl<T> OpsNot for Mut<T> {
    type Output = Not<Self>;

    fn not(self) -> Self::Output {
        Not::new()
    }
}

impl<A, B, R> BitAnd<R> for And<A, B>
where
    R: FilterOperand,
{
    type Output = And<Self, R>;

    fn bitand(self, _rhs: R) -> Self::Output {
        And::new()
    }
}

// Generics: `A` left child, `B` right child, `R` RHS operand.
impl<A, B, R> BitOr<R> for And<A, B>
where
    R: FilterOperand,
{
    type Output = Or<Self, R>;

    fn bitor(self, _rhs: R) -> Self::Output {
        Or::new()
    }
}

impl<A, B> OpsNot for And<A, B> {
    type Output = Not<Self>;

    fn not(self) -> Self::Output {
        Not::new()
    }
}

// Generics: `A` left child, `B` right child, `R` RHS operand.
impl<A, B, R> BitAnd<R> for Or<A, B>
where
    R: FilterOperand,
{
    type Output = And<Self, R>;

    fn bitand(self, _rhs: R) -> Self::Output {
        And::new()
    }
}

impl<A, B, R> BitOr<R> for Or<A, B>
where
    R: FilterOperand,
{
    type Output = Or<Self, R>;

    fn bitor(self, _rhs: R) -> Self::Output {
        Or::new()
    }
}

// Generics: `A` left child, `B` right child.
impl<A, B> OpsNot for Or<A, B> {
    type Output = Not<Self>;

    fn not(self) -> Self::Output {
        Not::new()
    }
}

impl<A, R> BitAnd<R> for Not<A>
where
    R: FilterOperand,
{
    type Output = And<Self, R>;

    fn bitand(self, _rhs: R) -> Self::Output {
        And::new()
    }
}

// Generics: `A` left child, `R` RHS operand.
impl<A, R> BitOr<R> for Not<A>
where
    R: FilterOperand,
{
    type Output = Or<Self, R>;

    fn bitor(self, _rhs: R) -> Self::Output {
        Or::new()
    }
}

impl<A> OpsNot for Not<A> {
    type Output = Not<Self>;

    fn not(self) -> Self::Output {
        Not::new()
    }
}

// Generics: `E` event, `I` inner, `R` RHS operand.
impl<E, I, R> BitAnd<R> for On<E, I>
where
    R: FilterOperand,
{
    type Output = And<Self, R>;

    fn bitand(self, _rhs: R) -> Self::Output {
        And::new()
    }
}

impl<E, I, R> BitOr<R> for On<E, I>
where
    R: FilterOperand,
{
    type Output = Or<Self, R>;

    fn bitor(self, _rhs: R) -> Self::Output {
        Or::new()
    }
}

// Generics: `E` event, `I` inner.
impl<E, I> OpsNot for On<E, I> {
    type Output = Not<Self>;

    fn not(self) -> Self::Output {
        Not::new()
    }
}
