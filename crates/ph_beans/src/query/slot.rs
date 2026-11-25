//! One bean and its state, stored side by side.
//!
//! An instance keeps one [`Slot`] per provided bean, and an inject is the slot
//! of the parent. A claim picks a part of the slot: `Id::T` the bean,
//! `*Id::T` its state. The part is named by the claim ([`Part`]), not by the
//! view type, so a state of the bean's own type still has one meaning.

use core::marker::PhantomData;

use crate::bean::BeanInfo;
use crate::borrow::{Ix, Lend, Take, TakeView};

/// A bean and its state.
///
/// Generics: `P` bean ([`BeanInfo`]).
pub struct Slot<P: BeanInfo> {
    /// The bean value. Beans are zero-sized types.
    pub bean: P,
    /// The bean's [`BeanInfo::State`].
    pub state: P::State,
}

// Generics: `P` bean.
impl<P: BeanInfo> Slot<P> {
    /// Slot holding `bean` and `state`.
    pub const fn new(bean: P, state: P::State) -> Self {
        Self { bean, state }
    }
}

// Generics: `P` bean.
impl<P> Default for Slot<P>
where
    P: BeanInfo + Default,
    P::State: Default,
{
    fn default() -> Self {
        Self { bean: P::default(), state: P::State::default() }
    }
}

/// Which part of a [`Slot`] a claim reads.
#[doc(hidden)]
pub trait Part {
    /// That part for bean `P`.
    type Of<P: BeanInfo>;
    /// Borrows that part.
    fn get<P: BeanInfo>(slot: &Slot<P>) -> &Self::Of<P>;
    /// Borrows that part exclusively.
    fn get_mut<P: BeanInfo>(slot: &mut Slot<P>) -> &mut Self::Of<P>;
}

/// `Id::T` / `Id::MUT`: the bean.
#[doc(hidden)]
pub struct BeanPart;

impl Part for BeanPart {
    type Of<P: BeanInfo> = P;

    fn get<P: BeanInfo>(slot: &Slot<P>) -> &P {
        &slot.bean
    }

    fn get_mut<P: BeanInfo>(slot: &mut Slot<P>) -> &mut P {
        &mut slot.bean
    }
}

/// `*Id::T` / `*Id::MUT`: the bean's state.
#[doc(hidden)]
pub struct StatePart;

impl Part for StatePart {
    type Of<P: BeanInfo> = P::State;

    fn get<P: BeanInfo>(slot: &Slot<P>) -> &P::State {
        &slot.state
    }

    fn get_mut<P: BeanInfo>(slot: &mut Slot<P>) -> &mut P::State {
        &mut slot.state
    }
}

/// A borrow of a slot narrowed to part `K`. It keeps the borrow's strength.
///
/// Generics: `K` part ([`Part`]).
#[doc(hidden)]
pub trait Project<K> {
    /// The narrowed borrow.
    type Out;
    /// Narrows the borrow.
    fn project(self) -> Self::Out;
}

// Generics: `P` bean, `K` part.
impl<'a, P: BeanInfo, K: Part> Project<K> for &'a Slot<P>
where
    K::Of<P>: 'a,
{
    type Out = &'a K::Of<P>;

    fn project(self) -> Self::Out {
        K::get(self)
    }
}

// Generics: `P` bean, `K` part.
impl<'a, P: BeanInfo, K: Part> Project<K> for &'a mut Slot<P>
where
    K::Of<P>: 'a,
{
    type Out = &'a mut K::Of<P>;

    fn project(self) -> Self::Out {
        K::get_mut(self)
    }
}

/// Index node: part `K` of the slot index `I` names, claimed with access
/// `A` on bean `P`.
///
/// Generics: `K` part ([`Part`]), `I` slot index, `A` access, `P` bean.
#[doc(hidden)]
pub struct Pick<K, I, A, P> {
    _markers: PhantomData<(K, I, A, P)>,
}

// Generics: `R` tuple of borrows, `V` view, `K` part, `N` slot index,
// `A` access, `P` bean.
impl<R, V, K, A, P, const N: usize> TakeView<R, V> for Pick<K, Ix<N>, A, P>
where
    R: Take<Ix<N>, Out: Project<K, Out: Lend<V>>>,
{
    type Rest = R::Rest;

    fn take_view(refs: R) -> (V, Self::Rest) {
        let (slot, rest) = refs.take();
        (slot.project().lend(), rest)
    }
}
