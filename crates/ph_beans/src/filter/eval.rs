//! Runtime match of a filter against the emitted event set.

use core::any::TypeId;

use crate::descriptor::{Get, Mut};
use crate::filter::{And, Not, On, Or};

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

fn listed(emitted: &[TypeId], id: TypeId) -> bool {
    emitted.contains(&id)
}

impl<E, I> MatchEmit for On<E, I>
where
    E: 'static,
    I: MatchEmit,
{
    fn matches(emitted: &[TypeId]) -> bool {
        listed(emitted, TypeId::of::<E>()) && I::matches(emitted)
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
