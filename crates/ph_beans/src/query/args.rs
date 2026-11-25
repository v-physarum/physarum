//! Claim shape: what a claim lends when it is a module's `Injects`
//! ([`ClaimShape::Lent`]), and what a claim leaf names ([`ClaimLeaf`]).
//!
//! What a system takes for a claim is module-aware: see
//! [`Claim`](crate::Claim) in `query/claim.rs`. The status of a claim list is
//! [`OpDesc`](crate::OpDesc) in `query/marks.rs`.

use crate::bean::BeanInfo;
use crate::descriptor::live::ClaimDesc;
use crate::descriptor::{Get, Mut, State};
use crate::filter::And;
use crate::query::slot::{BeanPart, Part, Slot, StatePart};

/// Structural claim node.
pub trait ClaimShape {
    /// What a caller lends when this claim is a module's `Injects`: the
    /// slots of the injected beans (`&Slot<P>` / `&mut Slot<P>`).
    type Lent<'a>
    where
        Self: 'a;
}

impl ClaimShape for () {
    type Lent<'a>
        = ()
    where
        Self: 'a;
}

/// What a claim leaf names: a bean and the part of its [`Slot`] it reads.
///
/// `P` reads the bean, [`State`]`<P>` (`*P::T`) reads its state.
#[doc(hidden)]
pub trait ClaimLeaf {
    /// The bean whose slot is read.
    type Bean: BeanInfo;
    /// The part of that slot.
    type Part: Part;
}

// Generics: `P` bean. `Id::T` reads the bean itself.
impl<P: BeanInfo> ClaimLeaf for P {
    type Bean = P;
    type Part = BeanPart;
}

// Generics: `P` bean. `*Id::T` reads the bean's state.
impl<P: BeanInfo> ClaimLeaf for State<P> {
    type Bean = P;
    type Part = StatePart;
}

// Generics: `P` claim leaf ([`ClaimLeaf`]).
impl<P: ClaimLeaf> ClaimShape for Get<P> {
    type Lent<'a>
        = &'a Slot<P::Bean>
    where
        Self: 'a;
}

// The module's entry for the bean decides the view ([`Entry`]).
// Generics: `B` module ([`Holds`]), `P` claim leaf.
// Generics: `P` claim leaf ([`ClaimLeaf`]).
impl<P: ClaimLeaf> ClaimShape for Mut<P> {
    type Lent<'a>
        = &'a mut Slot<P::Bean>
    where
        Self: 'a;
}

// Generics: `B` module ([`Holds`]), `P` claim leaf.
// Generics: `A` left claim, `C` right claim.
impl<A, C> ClaimShape for And<A, C>
where
    A: ClaimShape,
    C: ClaimShape,
{
    type Lent<'a>
        = (A::Lent<'a>, C::Lent<'a>)
    where
        Self: 'a;
}

// Generics: `B` surface, `A` left claim, `C` right claim.
// Generics: `A` left claim, `C` right claim.
impl<A, C> ClaimShape for (A, C)
where
    A: ClaimShape,
    C: ClaimShape,
{
    type Lent<'a>
        = (A::Lent<'a>, C::Lent<'a>)
    where
        Self: 'a;
}

// Generics: `B` surface, `A` left claim, `C` right claim.
impl<T> ClaimShape for T
where
    T: ClaimDesc,
    T::Inner: ClaimShape,
{
    type Lent<'a>
        = <T::Inner as ClaimShape>::Lent<'a>
    where
        Self: 'a;
}

// Generics: `B` right / bean, `T` tick / type / tip.
