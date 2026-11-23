//! Flat provide and inject lists of length 3..=8.
//!
//! Each claim sits beside the others in one tuple. [`ProvideAt`] / [`InjectAt`]
//! carry the length and the index, so two arities do not overlap. A nested
//! tail is not a list: every element is a [`ClaimMark`](super::ClaimMark).
//!
//! [`InjectAt`]: super::InjectAt
//! [`ProvideAt`]: super::ProvideAt

use crate::bean::BeanInfo;

use super::{ClaimMark, FromParent, InjectAt, ProvideAt};

impl<P, C, E1, E2> FromParent<P, ProvideAt<3, 0>> for C
where
    P: BeanInfo<Provides = (C, E1, E2)>,
    C: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
{
}

impl<P, C, E0, E2> FromParent<P, ProvideAt<3, 1>> for C
where
    P: BeanInfo<Provides = (E0, C, E2)>,
    C: ClaimMark,
    E0: ClaimMark,
    E2: ClaimMark,
{
}

impl<P, C, E0, E1> FromParent<P, ProvideAt<3, 2>> for C
where
    P: BeanInfo<Provides = (E0, E1, C)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
{
}

impl<P, C, E1, E2, E3> FromParent<P, ProvideAt<4, 0>> for C
where
    P: BeanInfo<Provides = (C, E1, E2, E3)>,
    C: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
{
}

impl<P, C, E0, E2, E3> FromParent<P, ProvideAt<4, 1>> for C
where
    P: BeanInfo<Provides = (E0, C, E2, E3)>,
    C: ClaimMark,
    E0: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
{
}

impl<P, C, E0, E1, E3> FromParent<P, ProvideAt<4, 2>> for C
where
    P: BeanInfo<Provides = (E0, E1, C, E3)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E3: ClaimMark,
{
}

impl<P, C, E0, E1, E2> FromParent<P, ProvideAt<4, 3>> for C
where
    P: BeanInfo<Provides = (E0, E1, E2, C)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
{
}

impl<P, C, E1, E2, E3, E4> FromParent<P, ProvideAt<5, 0>> for C
where
    P: BeanInfo<Provides = (C, E1, E2, E3, E4)>,
    C: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
{
}

impl<P, C, E0, E2, E3, E4> FromParent<P, ProvideAt<5, 1>> for C
where
    P: BeanInfo<Provides = (E0, C, E2, E3, E4)>,
    C: ClaimMark,
    E0: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
{
}

impl<P, C, E0, E1, E3, E4> FromParent<P, ProvideAt<5, 2>> for C
where
    P: BeanInfo<Provides = (E0, E1, C, E3, E4)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E4> FromParent<P, ProvideAt<5, 3>> for C
where
    P: BeanInfo<Provides = (E0, E1, E2, C, E4)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E4: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E3> FromParent<P, ProvideAt<5, 4>> for C
where
    P: BeanInfo<Provides = (E0, E1, E2, E3, C)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
{
}

impl<P, C, E1, E2, E3, E4, E5> FromParent<P, ProvideAt<6, 0>> for C
where
    P: BeanInfo<Provides = (C, E1, E2, E3, E4, E5)>,
    C: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
{
}

impl<P, C, E0, E2, E3, E4, E5> FromParent<P, ProvideAt<6, 1>> for C
where
    P: BeanInfo<Provides = (E0, C, E2, E3, E4, E5)>,
    C: ClaimMark,
    E0: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
{
}

impl<P, C, E0, E1, E3, E4, E5> FromParent<P, ProvideAt<6, 2>> for C
where
    P: BeanInfo<Provides = (E0, E1, C, E3, E4, E5)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E4, E5> FromParent<P, ProvideAt<6, 3>> for C
where
    P: BeanInfo<Provides = (E0, E1, E2, C, E4, E5)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E3, E5> FromParent<P, ProvideAt<6, 4>> for C
where
    P: BeanInfo<Provides = (E0, E1, E2, E3, C, E5)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E5: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E3, E4> FromParent<P, ProvideAt<6, 5>> for C
where
    P: BeanInfo<Provides = (E0, E1, E2, E3, E4, C)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
{
}

impl<P, C, E1, E2, E3, E4, E5, E6> FromParent<P, ProvideAt<7, 0>> for C
where
    P: BeanInfo<Provides = (C, E1, E2, E3, E4, E5, E6)>,
    C: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
    E6: ClaimMark,
{
}

impl<P, C, E0, E2, E3, E4, E5, E6> FromParent<P, ProvideAt<7, 1>> for C
where
    P: BeanInfo<Provides = (E0, C, E2, E3, E4, E5, E6)>,
    C: ClaimMark,
    E0: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
    E6: ClaimMark,
{
}

impl<P, C, E0, E1, E3, E4, E5, E6> FromParent<P, ProvideAt<7, 2>> for C
where
    P: BeanInfo<Provides = (E0, E1, C, E3, E4, E5, E6)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
    E6: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E4, E5, E6> FromParent<P, ProvideAt<7, 3>> for C
where
    P: BeanInfo<Provides = (E0, E1, E2, C, E4, E5, E6)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
    E6: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E3, E5, E6> FromParent<P, ProvideAt<7, 4>> for C
where
    P: BeanInfo<Provides = (E0, E1, E2, E3, C, E5, E6)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E5: ClaimMark,
    E6: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E3, E4, E6> FromParent<P, ProvideAt<7, 5>> for C
where
    P: BeanInfo<Provides = (E0, E1, E2, E3, E4, C, E6)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E6: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E3, E4, E5> FromParent<P, ProvideAt<7, 6>> for C
where
    P: BeanInfo<Provides = (E0, E1, E2, E3, E4, E5, C)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
{
}

impl<P, C, E1, E2, E3, E4, E5, E6, E7> FromParent<P, ProvideAt<8, 0>> for C
where
    P: BeanInfo<Provides = (C, E1, E2, E3, E4, E5, E6, E7)>,
    C: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
    E6: ClaimMark,
    E7: ClaimMark,
{
}

impl<P, C, E0, E2, E3, E4, E5, E6, E7> FromParent<P, ProvideAt<8, 1>> for C
where
    P: BeanInfo<Provides = (E0, C, E2, E3, E4, E5, E6, E7)>,
    C: ClaimMark,
    E0: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
    E6: ClaimMark,
    E7: ClaimMark,
{
}

impl<P, C, E0, E1, E3, E4, E5, E6, E7> FromParent<P, ProvideAt<8, 2>> for C
where
    P: BeanInfo<Provides = (E0, E1, C, E3, E4, E5, E6, E7)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
    E6: ClaimMark,
    E7: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E4, E5, E6, E7> FromParent<P, ProvideAt<8, 3>> for C
where
    P: BeanInfo<Provides = (E0, E1, E2, C, E4, E5, E6, E7)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
    E6: ClaimMark,
    E7: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E3, E5, E6, E7> FromParent<P, ProvideAt<8, 4>> for C
where
    P: BeanInfo<Provides = (E0, E1, E2, E3, C, E5, E6, E7)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E5: ClaimMark,
    E6: ClaimMark,
    E7: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E3, E4, E6, E7> FromParent<P, ProvideAt<8, 5>> for C
where
    P: BeanInfo<Provides = (E0, E1, E2, E3, E4, C, E6, E7)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E6: ClaimMark,
    E7: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E3, E4, E5, E7> FromParent<P, ProvideAt<8, 6>> for C
where
    P: BeanInfo<Provides = (E0, E1, E2, E3, E4, E5, C, E7)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
    E7: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E3, E4, E5, E6> FromParent<P, ProvideAt<8, 7>> for C
where
    P: BeanInfo<Provides = (E0, E1, E2, E3, E4, E5, E6, C)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
    E6: ClaimMark,
{
}

impl<P, C, E1, E2> FromParent<P, InjectAt<3, 0>> for C
where
    P: BeanInfo<Injects = (C, E1, E2)>,
    C: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
{
}

impl<P, C, E0, E2> FromParent<P, InjectAt<3, 1>> for C
where
    P: BeanInfo<Injects = (E0, C, E2)>,
    C: ClaimMark,
    E0: ClaimMark,
    E2: ClaimMark,
{
}

impl<P, C, E0, E1> FromParent<P, InjectAt<3, 2>> for C
where
    P: BeanInfo<Injects = (E0, E1, C)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
{
}

impl<P, C, E1, E2, E3> FromParent<P, InjectAt<4, 0>> for C
where
    P: BeanInfo<Injects = (C, E1, E2, E3)>,
    C: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
{
}

impl<P, C, E0, E2, E3> FromParent<P, InjectAt<4, 1>> for C
where
    P: BeanInfo<Injects = (E0, C, E2, E3)>,
    C: ClaimMark,
    E0: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
{
}

impl<P, C, E0, E1, E3> FromParent<P, InjectAt<4, 2>> for C
where
    P: BeanInfo<Injects = (E0, E1, C, E3)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E3: ClaimMark,
{
}

impl<P, C, E0, E1, E2> FromParent<P, InjectAt<4, 3>> for C
where
    P: BeanInfo<Injects = (E0, E1, E2, C)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
{
}

impl<P, C, E1, E2, E3, E4> FromParent<P, InjectAt<5, 0>> for C
where
    P: BeanInfo<Injects = (C, E1, E2, E3, E4)>,
    C: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
{
}

impl<P, C, E0, E2, E3, E4> FromParent<P, InjectAt<5, 1>> for C
where
    P: BeanInfo<Injects = (E0, C, E2, E3, E4)>,
    C: ClaimMark,
    E0: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
{
}

impl<P, C, E0, E1, E3, E4> FromParent<P, InjectAt<5, 2>> for C
where
    P: BeanInfo<Injects = (E0, E1, C, E3, E4)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E4> FromParent<P, InjectAt<5, 3>> for C
where
    P: BeanInfo<Injects = (E0, E1, E2, C, E4)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E4: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E3> FromParent<P, InjectAt<5, 4>> for C
where
    P: BeanInfo<Injects = (E0, E1, E2, E3, C)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
{
}

impl<P, C, E1, E2, E3, E4, E5> FromParent<P, InjectAt<6, 0>> for C
where
    P: BeanInfo<Injects = (C, E1, E2, E3, E4, E5)>,
    C: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
{
}

impl<P, C, E0, E2, E3, E4, E5> FromParent<P, InjectAt<6, 1>> for C
where
    P: BeanInfo<Injects = (E0, C, E2, E3, E4, E5)>,
    C: ClaimMark,
    E0: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
{
}

impl<P, C, E0, E1, E3, E4, E5> FromParent<P, InjectAt<6, 2>> for C
where
    P: BeanInfo<Injects = (E0, E1, C, E3, E4, E5)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E4, E5> FromParent<P, InjectAt<6, 3>> for C
where
    P: BeanInfo<Injects = (E0, E1, E2, C, E4, E5)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E3, E5> FromParent<P, InjectAt<6, 4>> for C
where
    P: BeanInfo<Injects = (E0, E1, E2, E3, C, E5)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E5: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E3, E4> FromParent<P, InjectAt<6, 5>> for C
where
    P: BeanInfo<Injects = (E0, E1, E2, E3, E4, C)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
{
}

impl<P, C, E1, E2, E3, E4, E5, E6> FromParent<P, InjectAt<7, 0>> for C
where
    P: BeanInfo<Injects = (C, E1, E2, E3, E4, E5, E6)>,
    C: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
    E6: ClaimMark,
{
}

impl<P, C, E0, E2, E3, E4, E5, E6> FromParent<P, InjectAt<7, 1>> for C
where
    P: BeanInfo<Injects = (E0, C, E2, E3, E4, E5, E6)>,
    C: ClaimMark,
    E0: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
    E6: ClaimMark,
{
}

impl<P, C, E0, E1, E3, E4, E5, E6> FromParent<P, InjectAt<7, 2>> for C
where
    P: BeanInfo<Injects = (E0, E1, C, E3, E4, E5, E6)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
    E6: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E4, E5, E6> FromParent<P, InjectAt<7, 3>> for C
where
    P: BeanInfo<Injects = (E0, E1, E2, C, E4, E5, E6)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
    E6: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E3, E5, E6> FromParent<P, InjectAt<7, 4>> for C
where
    P: BeanInfo<Injects = (E0, E1, E2, E3, C, E5, E6)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E5: ClaimMark,
    E6: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E3, E4, E6> FromParent<P, InjectAt<7, 5>> for C
where
    P: BeanInfo<Injects = (E0, E1, E2, E3, E4, C, E6)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E6: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E3, E4, E5> FromParent<P, InjectAt<7, 6>> for C
where
    P: BeanInfo<Injects = (E0, E1, E2, E3, E4, E5, C)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
{
}

impl<P, C, E1, E2, E3, E4, E5, E6, E7> FromParent<P, InjectAt<8, 0>> for C
where
    P: BeanInfo<Injects = (C, E1, E2, E3, E4, E5, E6, E7)>,
    C: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
    E6: ClaimMark,
    E7: ClaimMark,
{
}

impl<P, C, E0, E2, E3, E4, E5, E6, E7> FromParent<P, InjectAt<8, 1>> for C
where
    P: BeanInfo<Injects = (E0, C, E2, E3, E4, E5, E6, E7)>,
    C: ClaimMark,
    E0: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
    E6: ClaimMark,
    E7: ClaimMark,
{
}

impl<P, C, E0, E1, E3, E4, E5, E6, E7> FromParent<P, InjectAt<8, 2>> for C
where
    P: BeanInfo<Injects = (E0, E1, C, E3, E4, E5, E6, E7)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
    E6: ClaimMark,
    E7: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E4, E5, E6, E7> FromParent<P, InjectAt<8, 3>> for C
where
    P: BeanInfo<Injects = (E0, E1, E2, C, E4, E5, E6, E7)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
    E6: ClaimMark,
    E7: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E3, E5, E6, E7> FromParent<P, InjectAt<8, 4>> for C
where
    P: BeanInfo<Injects = (E0, E1, E2, E3, C, E5, E6, E7)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E5: ClaimMark,
    E6: ClaimMark,
    E7: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E3, E4, E6, E7> FromParent<P, InjectAt<8, 5>> for C
where
    P: BeanInfo<Injects = (E0, E1, E2, E3, E4, C, E6, E7)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E6: ClaimMark,
    E7: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E3, E4, E5, E7> FromParent<P, InjectAt<8, 6>> for C
where
    P: BeanInfo<Injects = (E0, E1, E2, E3, E4, E5, C, E7)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
    E7: ClaimMark,
{
}

impl<P, C, E0, E1, E2, E3, E4, E5, E6> FromParent<P, InjectAt<8, 7>> for C
where
    P: BeanInfo<Injects = (E0, E1, E2, E3, E4, E5, E6, C)>,
    C: ClaimMark,
    E0: ClaimMark,
    E1: ClaimMark,
    E2: ClaimMark,
    E3: ClaimMark,
    E4: ClaimMark,
    E5: ClaimMark,
    E6: ClaimMark,
{
}
