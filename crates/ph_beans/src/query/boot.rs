//! Makes one instance of a module: its provided states, then the init system once.

use crate::bean::BeanInfo;
use crate::context::Owned;
use crate::query::args::ClaimShape;
use crate::query::claim::Claim;
use crate::query::lend::Beans;
use crate::query::sees::Sees;
use crate::query::store::Stored;

/// An init function for module `B` under claim `A`.
///
/// The checks sit on the handler type. While the handler is still being
/// inferred the compiler cannot reject `.init_state` early (`E0599`), so a
/// failure prints its coded message. Generics: `B` module, `A` claim.
#[doc(hidden)]
pub trait InitSys<B: BeanInfo, A> {
    /// States the module owns for its provided beans.
    type States;
    /// The inject view the caller lends.
    type Injects<'x>;
    /// Creates the provided states, then runs the function once over its
    /// claim: provided states and `injects`.
    fn boot(&self, injects: Self::Injects<'_>) -> Owned<B::State, Self::States>;
}

// Generics: `B` module, `A` claim, `H` init handler.
impl<B, A, H> InitSys<B, A> for H
where
    B: Stored<Injects: ClaimShape> + for<'s, 'x> Beans<'s, 'x>,
    for<'s, 'x> A: Sees<B, <B as Beans<'s, 'x>>::Full>,
    H: for<'a> Fn(<A as Claim<B>>::Arg<'a>) -> B::State,
{
    type States = B::States;
    type Injects<'x> = <B::Injects as ClaimShape>::Lent<'x>;

    fn boot(&self, injects: Self::Injects<'_>) -> Owned<B::State, B::States> {
        let mut provides: B::States = Default::default();
        let state = A::call(&mut B::full(&mut provides, injects), self);
        Owned::new(state, provides)
    }
}
