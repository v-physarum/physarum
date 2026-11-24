//! Claim shape: args [`ClaimShape::View`], op [`ClaimShape::Status`], and
//! [`Claim`] indexes into bean `B`.
//!
//! One structural walk for view + status (replaces parallel `ArgsView` /
//! `OpLeaf` tables). [`Claim`] only adds `At` indexes.

use crate::bean::BeanInfo;
use crate::borrow::At;
use crate::context::EventBuf;
use crate::descriptor::Valid;
use crate::descriptor::live::ClaimDesc;
use crate::descriptor::{Dyn, DynView, Get, JoinStatus, Mut, OpDesc, State};
use crate::filter::And;

/// Structural claim node: init/inject view and local op status.
pub trait ClaimShape {
    /// Borrowed arguments for one init / inject call.
    type View<'a>
    where
        Self: 'a;
    /// Local operator status (`Valid`, or a named claim error later).
    type Status;
}

/// Resource claim `B` can project.
///
/// Leaves carry `B: At<P>`. Lists only require each part to be a [`Claim`].
/// Generics: `B` bean surface.
pub trait Claim<B>: ClaimShape {
    /// Absolute indexes of the claimed beans.
    type Indexes;
}

impl ClaimShape for () {
    type View<'a>
        = ()
    where
        Self: 'a;
    type Status = Valid;
}

impl<B> Claim<B> for () {
    type Indexes = ();
}

// Beans and the event buffer. `Get<State<_>>` is a [`ClaimDesc`] forward, not this impl.
trait ClaimLeaf {}

// Generics: `P` bean ([`BeanInfo`]).
impl<P: BeanInfo> ClaimLeaf for P {}

// Generics: `S` event state value.
impl<S> ClaimLeaf for EventBuf<S> {}

// Generics: `P` claimed value ([`ClaimLeaf`]).
impl<P: ClaimLeaf> ClaimShape for Get<P> {
    type View<'a>
        = &'a P
    where
        Self: 'a;
    type Status = Valid;
}

impl<B, P> Claim<B> for Get<P>
where
    P: ClaimLeaf,
    B: At<P>,
{
    type Indexes = B::Ix;
}

// Generics: `P` claimed value ([`ClaimLeaf`]).
impl<P: ClaimLeaf> ClaimShape for Mut<P> {
    type View<'a>
        = &'a mut P
    where
        Self: 'a;
    type Status = Valid;
}

impl<B, P> Claim<B> for Mut<P>
where
    P: ClaimLeaf,
    B: At<P>,
{
    type Indexes = B::Ix;
}

// Event state is not a provide slot. `*Ev::T` / `*Ev::MUT` reuse `Get` / `Mut`.
// Generics: `B` bean, `S` state.
impl<B, S> At<EventBuf<S>> for B {
    type Ix = ();
}

// Generics: `P` event bean. `Get<State<P>>` reuses [`Get`] of [`EventBuf`].
impl<P> ClaimDesc for Get<State<P>>
where
    P: BeanInfo,
{
    type Inner = Get<EventBuf<P::State>>;
}

// Generics: `P` event bean. `Mut<State<P>>` reuses [`Mut`] of [`EventBuf`].
impl<P> ClaimDesc for Mut<State<P>>
where
    P: BeanInfo,
{
    type Inner = Mut<EventBuf<P::State>>;
}

// Generics: `P` bean, `V` view ([`DynView`]).
impl<P, V: ?Sized> ClaimShape for Dyn<Get<P>, V>
where
    V: DynView<P>,
{
    type View<'a>
        = &'a V
    where
        Self: 'a;
    type Status = Valid;
}

// Generics: `B` surface, `P` bean, `V` view.
impl<B, P, V: ?Sized> Claim<B> for Dyn<Get<P>, V>
where
    B: At<P>,
    V: DynView<P>,
{
    type Indexes = B::Ix;
}

// Generics: `P` bean, `V` view ([`DynView`]).
impl<P, V: ?Sized> ClaimShape for Dyn<Mut<P>, V>
where
    V: DynView<P>,
{
    type View<'a>
        = &'a mut V
    where
        Self: 'a;
    type Status = Valid;
}

// Generics: `B` surface, `P` bean, `V` view.
impl<B, P, V: ?Sized> Claim<B> for Dyn<Mut<P>, V>
where
    B: At<P>,
    V: DynView<P>,
{
    type Indexes = B::Ix;
}

// Generics: `A` left claim, `C` right claim.
// Status joins children. Same-bean leaf conflicts need a non-overlapping
// encoding (derive / BeanNe) — stable coherence forbids Err arms beside this.
impl<A, C> ClaimShape for And<A, C>
where
    A: ClaimShape,
    C: ClaimShape,
    A::Status: JoinStatus<C::Status>,
{
    type View<'a>
        = (A::View<'a>, C::View<'a>)
    where
        Self: 'a;
    type Status = <A::Status as JoinStatus<C::Status>>::Out;
}

// Generics: `B` surface, `A` left claim, `C` right claim.
impl<B, A, C> Claim<B> for And<A, C>
where
    A: Claim<B>,
    C: Claim<B>,
    And<A, C>: ClaimShape,
{
    type Indexes = (A::Indexes, C::Indexes);
}

// Generics: `A` left claim, `C` right claim.
impl<A, C> ClaimShape for (A, C)
where
    A: ClaimShape,
    C: ClaimShape,
    A::Status: JoinStatus<C::Status>,
{
    type View<'a>
        = (A::View<'a>, C::View<'a>)
    where
        Self: 'a;
    type Status = <A::Status as JoinStatus<C::Status>>::Out;
}

// Generics: `B` surface, `A` left claim, `C` right claim.
impl<B, A, C> Claim<B> for (A, C)
where
    A: Claim<B>,
    C: Claim<B>,
    (A, C): ClaimShape,
{
    type Indexes = (A::Indexes, C::Indexes);
}

impl<T> ClaimShape for T
where
    T: ClaimDesc,
    T::Inner: ClaimShape,
{
    type View<'a>
        = <T::Inner as ClaimShape>::View<'a>
    where
        Self: 'a;
    type Status = <T::Inner as ClaimShape>::Status;
}

// Generics: `B` right / bean, `T` tick / type / tip.
impl<B, T> Claim<B> for T
where
    T: ClaimDesc,
    T::Inner: Claim<B>,
{
    type Indexes = <T::Inner as Claim<B>>::Indexes;
}

impl<T> OpDesc for T
where
    T: ClaimShape,
{
    type Status = T::Status;
}
