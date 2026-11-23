//! Flat provide and inject lists of length 2..=8.
//!
//! Each impl is `for (A0, A1, …)`. The elements are generic claims.
//! A nested tail is not an element: every `A*` is a [`ClaimMark`](super::ClaimMark).

use crate::bean::BeanInfo;

use super::{ClaimMark, FromParent, OfInject};

impl<P, A0, A1> FromParent<P, (A0, A1)> for (A0, A1)
where
    P: BeanInfo<Provides = (A0, A1)>,
    A0: ClaimMark,
    A1: ClaimMark,
{
}

impl<P, A0, A1, A2> FromParent<P, (A0, A1, A2)> for (A0, A1, A2)
where
    P: BeanInfo<Provides = (A0, A1, A2)>,
    A0: ClaimMark,
    A1: ClaimMark,
    A2: ClaimMark,
{
}

impl<P, A0, A1, A2, A3> FromParent<P, (A0, A1, A2, A3)> for (A0, A1, A2, A3)
where
    P: BeanInfo<Provides = (A0, A1, A2, A3)>,
    A0: ClaimMark,
    A1: ClaimMark,
    A2: ClaimMark,
    A3: ClaimMark,
{
}

impl<P, A0, A1, A2, A3, A4> FromParent<P, (A0, A1, A2, A3, A4)> for (A0, A1, A2, A3, A4)
where
    P: BeanInfo<Provides = (A0, A1, A2, A3, A4)>,
    A0: ClaimMark,
    A1: ClaimMark,
    A2: ClaimMark,
    A3: ClaimMark,
    A4: ClaimMark,
{
}

impl<P, A0, A1, A2, A3, A4, A5> FromParent<P, (A0, A1, A2, A3, A4, A5)> for (A0, A1, A2, A3, A4, A5)
where
    P: BeanInfo<Provides = (A0, A1, A2, A3, A4, A5)>,
    A0: ClaimMark,
    A1: ClaimMark,
    A2: ClaimMark,
    A3: ClaimMark,
    A4: ClaimMark,
    A5: ClaimMark,
{
}

impl<P, A0, A1, A2, A3, A4, A5, A6> FromParent<P, (A0, A1, A2, A3, A4, A5, A6)>
    for (A0, A1, A2, A3, A4, A5, A6)
where
    P: BeanInfo<Provides = (A0, A1, A2, A3, A4, A5, A6)>,
    A0: ClaimMark,
    A1: ClaimMark,
    A2: ClaimMark,
    A3: ClaimMark,
    A4: ClaimMark,
    A5: ClaimMark,
    A6: ClaimMark,
{
}

impl<P, A0, A1, A2, A3, A4, A5, A6, A7> FromParent<P, (A0, A1, A2, A3, A4, A5, A6, A7)>
    for (A0, A1, A2, A3, A4, A5, A6, A7)
where
    P: BeanInfo<Provides = (A0, A1, A2, A3, A4, A5, A6, A7)>,
    A0: ClaimMark,
    A1: ClaimMark,
    A2: ClaimMark,
    A3: ClaimMark,
    A4: ClaimMark,
    A5: ClaimMark,
    A6: ClaimMark,
    A7: ClaimMark,
{
}

impl<P, A0, A1> FromParent<P, OfInject<(A0, A1)>> for (A0, A1)
where
    P: BeanInfo<Injects = (A0, A1)>,
    A0: ClaimMark,
    A1: ClaimMark,
{
}

impl<P, A0, A1, A2> FromParent<P, OfInject<(A0, A1, A2)>> for (A0, A1, A2)
where
    P: BeanInfo<Injects = (A0, A1, A2)>,
    A0: ClaimMark,
    A1: ClaimMark,
    A2: ClaimMark,
{
}

impl<P, A0, A1, A2, A3> FromParent<P, OfInject<(A0, A1, A2, A3)>> for (A0, A1, A2, A3)
where
    P: BeanInfo<Injects = (A0, A1, A2, A3)>,
    A0: ClaimMark,
    A1: ClaimMark,
    A2: ClaimMark,
    A3: ClaimMark,
{
}

impl<P, A0, A1, A2, A3, A4> FromParent<P, OfInject<(A0, A1, A2, A3, A4)>> for (A0, A1, A2, A3, A4)
where
    P: BeanInfo<Injects = (A0, A1, A2, A3, A4)>,
    A0: ClaimMark,
    A1: ClaimMark,
    A2: ClaimMark,
    A3: ClaimMark,
    A4: ClaimMark,
{
}

impl<P, A0, A1, A2, A3, A4, A5> FromParent<P, OfInject<(A0, A1, A2, A3, A4, A5)>>
    for (A0, A1, A2, A3, A4, A5)
where
    P: BeanInfo<Injects = (A0, A1, A2, A3, A4, A5)>,
    A0: ClaimMark,
    A1: ClaimMark,
    A2: ClaimMark,
    A3: ClaimMark,
    A4: ClaimMark,
    A5: ClaimMark,
{
}

impl<P, A0, A1, A2, A3, A4, A5, A6> FromParent<P, OfInject<(A0, A1, A2, A3, A4, A5, A6)>>
    for (A0, A1, A2, A3, A4, A5, A6)
where
    P: BeanInfo<Injects = (A0, A1, A2, A3, A4, A5, A6)>,
    A0: ClaimMark,
    A1: ClaimMark,
    A2: ClaimMark,
    A3: ClaimMark,
    A4: ClaimMark,
    A5: ClaimMark,
    A6: ClaimMark,
{
}

impl<P, A0, A1, A2, A3, A4, A5, A6, A7> FromParent<P, OfInject<(A0, A1, A2, A3, A4, A5, A6, A7)>>
    for (A0, A1, A2, A3, A4, A5, A6, A7)
where
    P: BeanInfo<Injects = (A0, A1, A2, A3, A4, A5, A6, A7)>,
    A0: ClaimMark,
    A1: ClaimMark,
    A2: ClaimMark,
    A3: ClaimMark,
    A4: ClaimMark,
    A5: ClaimMark,
    A6: ClaimMark,
    A7: ClaimMark,
{
}
