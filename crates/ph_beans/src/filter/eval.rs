//! Runtime match of a filter against the emitted event set.

use core::any::TypeId;

use crate::bean::{BeanInfo, Marked, ParentList, Proxied, Stop};
use crate::descriptor::{Get, Mut};
use crate::filter::{And, Not, On, Or};

/// Parent ids a proxy event contributes to the target's schedule.
pub trait EventParent {
    /// `true` when `emitted` names this parent.
    fn listed(emitted: &[TypeId]) -> bool;
}

impl EventParent for () {
    #[inline]
    fn listed(_emitted: &[TypeId]) -> bool {
        false
    }
}

impl<E> EventParent for E
where
    E: crate::EventInfo + 'static,
{
    #[inline]
    fn listed(emitted: &[TypeId]) -> bool {
        emitted.contains(&TypeId::of::<E>())
    }
}

impl<A, B> EventParent for (A, B)
where
    A: crate::EventInfo + 'static,
    B: crate::EventInfo + 'static,
{
    #[inline]
    fn listed(emitted: &[TypeId]) -> bool {
        emitted.contains(&TypeId::of::<A>()) || emitted.contains(&TypeId::of::<B>())
    }
}

impl<A, B, C> EventParent for (A, B, C)
where
    A: crate::EventInfo + 'static,
    B: crate::EventInfo + 'static,
    C: crate::EventInfo + 'static,
{
    #[inline]
    fn listed(emitted: &[TypeId]) -> bool {
        emitted.contains(&TypeId::of::<A>())
            || emitted.contains(&TypeId::of::<B>())
            || emitted.contains(&TypeId::of::<C>())
    }
}

impl<A, B, C, D> EventParent for (A, B, C, D)
where
    A: crate::EventInfo + 'static,
    B: crate::EventInfo + 'static,
    C: crate::EventInfo + 'static,
    D: crate::EventInfo + 'static,
{
    #[inline]
    fn listed(emitted: &[TypeId]) -> bool {
        emitted.contains(&TypeId::of::<A>())
            || emitted.contains(&TypeId::of::<B>())
            || emitted.contains(&TypeId::of::<C>())
            || emitted.contains(&TypeId::of::<D>())
    }
}

impl<P> ParentList for P
where
    P: crate::EventInfo,
    P::Parents: EventParent,
{
    #[inline]
    fn listed(emitted: &[TypeId]) -> bool {
        P::Parents::listed(emitted)
    }
}

#[doc(hidden)]
pub trait Collect {
    /// Writes event ids into `buf` starting at `n`.
    fn write(buf: &mut [TypeId], n: usize) -> usize;
}

impl Collect for () {
    fn write(_buf: &mut [TypeId], n: usize) -> usize {
        n
    }
}

impl<T> Collect for Get<T> {
    fn write(_buf: &mut [TypeId], n: usize) -> usize {
        n
    }
}

impl<T> Collect for Mut<T> {
    fn write(_buf: &mut [TypeId], n: usize) -> usize {
        n
    }
}

impl<E, I> Collect for On<E, I>
where
    E: 'static,
    I: Collect,
{
    fn write(buf: &mut [TypeId], n: usize) -> usize {
        if n < buf.len() {
            buf[n] = TypeId::of::<E>();
            I::write(buf, n + 1)
        } else {
            n
        }
    }
}

impl<A, B> Collect for And<A, B>
where
    A: Collect,
    B: Collect,
{
    fn write(buf: &mut [TypeId], n: usize) -> usize {
        let n = A::write(buf, n);
        B::write(buf, n)
    }
}

impl<A, B> Collect for Or<A, B>
where
    A: Collect,
    B: Collect,
{
    fn write(buf: &mut [TypeId], n: usize) -> usize {
        let n = A::write(buf, n);
        B::write(buf, n)
    }
}

impl<A> Collect for Not<A> {
    fn write(_buf: &mut [TypeId], n: usize) -> usize {
        let _ = core::marker::PhantomData::<A>;
        n
    }
}

#[doc(hidden)]
pub trait MatchEmit {
    /// The filter passes for this emitted set.
    fn matches(emitted: &[TypeId]) -> bool;
}

impl MatchEmit for () {
    fn matches(_emitted: &[TypeId]) -> bool {
        true
    }
}

impl<T> MatchEmit for Get<T> {
    fn matches(_emitted: &[TypeId]) -> bool {
        true
    }
}

impl<T> MatchEmit for Mut<T> {
    fn matches(_emitted: &[TypeId]) -> bool {
        true
    }
}

#[inline]
fn listed(emitted: &[TypeId], id: TypeId) -> bool {
    emitted.contains(&id)
}

/// Events of every proxy from `Self` out to the end of the chain.
#[doc(hidden)]
pub trait ChainEmit<Link> {
    /// `true` when `emitted` names this proxy, one of its parents, or an outer proxy.
    fn hit(emitted: &[TypeId]) -> bool;
}

impl<P> ChainEmit<Stop> for P
where
    P: ParentList + 'static,
{
    #[inline]
    fn hit(emitted: &[TypeId]) -> bool {
        listed(emitted, TypeId::of::<P>()) || P::listed(emitted)
    }
}

impl<P, O0> ChainEmit<(O0,)> for P
where
    P: ParentList + 'static,
    O0: ParentList + 'static,
{
    #[inline]
    fn hit(emitted: &[TypeId]) -> bool {
        listed(emitted, TypeId::of::<P>())
            || P::listed(emitted)
            || listed(emitted, TypeId::of::<O0>())
            || O0::listed(emitted)
    }
}

impl<P, O0, O1> ChainEmit<(O0, O1)> for P
where
    P: ParentList + 'static,
    O0: ParentList + 'static,
    O1: ParentList + 'static,
{
    #[inline]
    fn hit(emitted: &[TypeId]) -> bool {
        listed(emitted, TypeId::of::<P>())
            || P::listed(emitted)
            || listed(emitted, TypeId::of::<O0>())
            || O0::listed(emitted)
            || listed(emitted, TypeId::of::<O1>())
            || O1::listed(emitted)
    }
}

impl<P, O0, O1, O2> ChainEmit<(O0, O1, O2)> for P
where
    P: ParentList + 'static,
    O0: ParentList + 'static,
    O1: ParentList + 'static,
    O2: ParentList + 'static,
{
    #[inline]
    fn hit(emitted: &[TypeId]) -> bool {
        listed(emitted, TypeId::of::<P>())
            || P::listed(emitted)
            || listed(emitted, TypeId::of::<O0>())
            || O0::listed(emitted)
            || listed(emitted, TypeId::of::<O1>())
            || O1::listed(emitted)
            || listed(emitted, TypeId::of::<O2>())
            || O2::listed(emitted)
    }
}

impl<E, I, P> MatchEmit for On<E, I>
where
    E: 'static + Proxied<Link: Marked<Target = P>>,
    P: BeanInfo + ChainEmit<P::Chain>,
    I: MatchEmit,
{
    #[inline]
    fn matches(emitted: &[TypeId]) -> bool {
        (listed(emitted, TypeId::of::<E>()) || <P as ChainEmit<P::Chain>>::hit(emitted))
            && I::matches(emitted)
    }
}

impl<A, B> MatchEmit for And<A, B>
where
    A: MatchEmit,
    B: MatchEmit,
{
    fn matches(emitted: &[TypeId]) -> bool {
        A::matches(emitted) && B::matches(emitted)
    }
}

impl<A, B> MatchEmit for Or<A, B>
where
    A: MatchEmit,
    B: MatchEmit,
{
    fn matches(emitted: &[TypeId]) -> bool {
        A::matches(emitted) || B::matches(emitted)
    }
}

impl<A> MatchEmit for Not<A>
where
    A: MatchEmit,
{
    fn matches(emitted: &[TypeId]) -> bool {
        !A::matches(emitted)
    }
}

/// Joins a second `.emit` onto the set already stored.
#[doc(hidden)]
pub trait UnionEmit<N> {
    /// Combined emit tree.
    type Out;
}

impl<N> UnionEmit<N> for () {
    type Out = N;
}

impl<E, I, N> UnionEmit<N> for On<E, I> {
    type Out = And<Self, N>;
}

impl<A, B, N> UnionEmit<N> for And<A, B> {
    type Out = And<Self, N>;
}

impl<A, B, N> UnionEmit<N> for Or<A, B> {
    type Out = And<Self, N>;
}

impl<A, N> UnionEmit<N> for Not<A> {
    type Out = And<Self, N>;
}
