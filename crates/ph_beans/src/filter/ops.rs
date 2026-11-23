//! `&`, `|`, and `!` build filter nodes.
//!
//! Each local node has its own `BitAnd`, `BitOr`, and `Not`. The right-hand
//! side is any [`FilterOperand`].

use core::ops::BitAnd;
use core::ops::BitOr;
use core::ops::Not as OpsNot;

use super::{And, Not, On, Or};
use crate::descriptor::{Get, Mut};

/// A node that can sit on the right of `&` or `|`, or under `!`.
pub trait FilterOperand: Sized {}

impl<T> FilterOperand for Get<T> {}

impl<T> FilterOperand for Mut<T> {}

impl<A, B> FilterOperand for And<A, B> {}

impl<A, B> FilterOperand for Or<A, B> {}

impl<A> FilterOperand for Not<A> {}

impl<E, I> FilterOperand for On<E, I> {}

impl<T, R> BitAnd<R> for Get<T>
where
    R: FilterOperand,
{
    type Output = And<Self, R>;

    fn bitand(self, _rhs: R) -> Self::Output {
        And::new()
    }
}

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

impl<T> OpsNot for Mut<T> {
    type Output = Not<Self>;

    fn not(self) -> Self::Output {
        Not::new()
    }
}
