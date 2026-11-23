//! `&`, `|`, and `!` for compound filter nodes.

use core::ops::BitAnd;
use core::ops::BitOr;
use core::ops::Not as OpsNot;

use super::ops::FilterOperand;
use super::{And, Not, On, Or};

impl<A, B, R> BitAnd<R> for And<A, B>
where
    R: FilterOperand,
{
    type Output = And<Self, R>;

    fn bitand(self, _rhs: R) -> Self::Output {
        And::new()
    }
}

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

impl<E, I> OpsNot for On<E, I> {
    type Output = Not<Self>;

    fn not(self) -> Self::Output {
        Not::new()
    }
}
