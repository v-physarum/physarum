use std::ops::BitAnd;

use crate::container::{
    accessors::{Get, Mut, Res},
    injection::{Bean, BeanInfo},
};

impl<B0, B1> BitAnd<B1> for Get<B0>
where
    B0: BeanInfo,
    B1: Bean,
{
    type Output = Res<(Self, B1)>;

    fn bitand(self, _: B1) -> Self::Output {
        Res::new()
    }
}

impl<B0, B1> BitAnd<B1> for Mut<B0>
where
    B0: BeanInfo,
    B1: Bean,
{
    type Output = Res<(Self, B1)>;

    fn bitand(self, _: B1) -> Self::Output {
        Res::new()
    }
}
