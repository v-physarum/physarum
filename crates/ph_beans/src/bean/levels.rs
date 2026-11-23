//! Outer proxies as one tuple on [`BeanInfo::Chain`](crate::BeanInfo::Chain).
//!
//! `Stop` is no further proxy. `(Mid, Outer)` is the tail, nearest first.
//! The stored claim stays nested: `Via<Outer, Via<Mid, Via<Inner, C>>>`.

use super::proxy::{Enfold, EnterAll, ForwardInit, Fwd, InitPath, Stop, Via, WrapFwd};
use crate::bean::BeanInfo;

impl<P, C, O0> Enfold<C, (O0,)> for P
where
    O0: Enfold<Via<P, C>, Stop>,
{
    type Wrapped = <O0 as Enfold<Via<P, C>, Stop>>::Wrapped;
}

impl<P, C, O0, O1> Enfold<C, (O0, O1)> for P
where
    O0: Enfold<Via<P, C>, Stop>,
    O1: Enfold<<O0 as Enfold<Via<P, C>, Stop>>::Wrapped, Stop>,
{
    type Wrapped = <O1 as Enfold<<O0 as Enfold<Via<P, C>, Stop>>::Wrapped, Stop>>::Wrapped;
}

impl<P, C, O0, O1, O2> Enfold<C, (O0, O1, O2)> for P
where
    O0: Enfold<Via<P, C>, Stop>,
    O1: Enfold<<O0 as Enfold<Via<P, C>, Stop>>::Wrapped, Stop>,
    O2: Enfold<<O1 as Enfold<<O0 as Enfold<Via<P, C>, Stop>>::Wrapped, Stop>>::Wrapped, Stop>,
{
    type Wrapped = <O2 as Enfold<
        <O1 as Enfold<<O0 as Enfold<Via<P, C>, Stop>>::Wrapped, Stop>>::Wrapped,
        Stop,
    >>::Wrapped;
}

impl<P, B, H, O0> ForwardInit<B, H, (O0,)> for P
where
    B: BeanInfo,
    P: InitPath<B>,
    O0: WrapFwd<Fwd<P, H>, Stop>,
{
    type Out = <O0 as WrapFwd<Fwd<P, H>, Stop>>::Out;

    fn forward(handler: H) -> Self::Out {
        O0::wrap_fwd(<P as InitPath<B>>::install(handler))
    }
}

impl<P, B, H, O0, O1> ForwardInit<B, H, (O0, O1)> for P
where
    B: BeanInfo,
    P: InitPath<B>,
    O0: WrapFwd<Fwd<P, H>, Stop>,
    O1: WrapFwd<<O0 as WrapFwd<Fwd<P, H>, Stop>>::Out, Stop>,
{
    type Out = <O1 as WrapFwd<<O0 as WrapFwd<Fwd<P, H>, Stop>>::Out, Stop>>::Out;

    fn forward(handler: H) -> Self::Out {
        let inner = <P as InitPath<B>>::install(handler);
        O1::wrap_fwd(O0::wrap_fwd(inner))
    }
}

impl<P, B, H, O0, O1, O2> ForwardInit<B, H, (O0, O1, O2)> for P
where
    B: BeanInfo,
    P: InitPath<B>,
    O0: WrapFwd<Fwd<P, H>, Stop>,
    O1: WrapFwd<<O0 as WrapFwd<Fwd<P, H>, Stop>>::Out, Stop>,
    O2: WrapFwd<<O1 as WrapFwd<<O0 as WrapFwd<Fwd<P, H>, Stop>>::Out, Stop>>::Out, Stop>,
{
    type Out = <O2 as WrapFwd<
        <O1 as WrapFwd<<O0 as WrapFwd<Fwd<P, H>, Stop>>::Out, Stop>>::Out,
        Stop,
    >>::Out;

    fn forward(handler: H) -> Self::Out {
        let inner = <P as InitPath<B>>::install(handler);
        O2::wrap_fwd(O1::wrap_fwd(O0::wrap_fwd(inner)))
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
