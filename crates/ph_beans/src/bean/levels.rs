//! Outer proxies as the third element of [`BeanInfo::Gate`](crate::BeanInfo::Gate).
//!
//! `Stop` is no further proxy. `(Mid, Outer)` is the tail, nearest first.
//! The stored claim is one leaf: `List<Leaf<(Inner, Mid, Outer, Get<Core>)>>`.

use super::proxy::{Enfold, EnterAll, ForwardInit, Fwd, InitPath};
use crate::bean::BeanInfo;
use crate::context::{Leaf, List};

impl<P, C, O0> Enfold<C, (O0,)> for P {
    type Wrapped = List<Leaf<(P, O0, C)>>;
}

impl<P, C, O0, O1> Enfold<C, (O0, O1)> for P {
    type Wrapped = List<Leaf<(P, O0, O1, C)>>;
}

impl<P, C, O0, O1, O2> Enfold<C, (O0, O1, O2)> for P {
    type Wrapped = List<Leaf<(P, O0, O1, O2, C)>>;
}

impl<P, B, H, O0> ForwardInit<B, H, (O0,)> for P
where
    B: BeanInfo,
    P: InitPath<B>,
{
    type Out = Fwd<List<Leaf<(P, O0)>>, H>;

    fn forward(handler: H) -> Self::Out {
        Fwd::new(<P as InitPath<B>>::install(handler).into_handler())
    }
}

impl<P, B, H, O0, O1> ForwardInit<B, H, (O0, O1)> for P
where
    B: BeanInfo,
    P: InitPath<B>,
{
    type Out = Fwd<List<Leaf<(P, O0, O1)>>, H>;

    fn forward(handler: H) -> Self::Out {
        Fwd::new(<P as InitPath<B>>::install(handler).into_handler())
    }
}

impl<P, B, H, O0, O1, O2> ForwardInit<B, H, (O0, O1, O2)> for P
where
    B: BeanInfo,
    P: InitPath<B>,
{
    type Out = Fwd<List<Leaf<(P, O0, O1, O2)>>, H>;

    fn forward(handler: H) -> Self::Out {
        Fwd::new(<P as InitPath<B>>::install(handler).into_handler())
    }
}

impl<P, B, O0> EnterAll<B, (O0,)> for P
where
    B: BeanInfo,
    P: InitPath<B>,
    O0: InitPath<P>,
{
    fn enter_all() {
        <O0 as InitPath<P>>::enter();
        <P as InitPath<B>>::enter();
    }
}

impl<P, B, O0, O1> EnterAll<B, (O0, O1)> for P
where
    B: BeanInfo,
    P: InitPath<B>,
    O0: InitPath<P>,
    O1: InitPath<O0>,
{
    fn enter_all() {
        <O1 as InitPath<O0>>::enter();
        <O0 as InitPath<P>>::enter();
        <P as InitPath<B>>::enter();
    }
}

impl<P, B, O0, O1, O2> EnterAll<B, (O0, O1, O2)> for P
where
    B: BeanInfo,
    P: InitPath<B>,
    O0: InitPath<P>,
    O1: InitPath<O0>,
    O2: InitPath<O1>,
{
    fn enter_all() {
        <O2 as InitPath<O1>>::enter();
        <O1 as InitPath<O0>>::enter();
        <O0 as InitPath<P>>::enter();
        <P as InitPath<B>>::enter();
    }
}
