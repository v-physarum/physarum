//! The init system of a module: run once over the borrows of a new instance.

use crate::bean::{BeanInfo, Blank, NoState};
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
    /// Runs the function once over its claim, taken from the borrows of the
    /// new instance: its provided states and the injects.
    ///
    /// Generics: `R` those borrows.
    fn start<R>(&self, full: &mut R) -> B::State
    where
        A: Sees<B, R>;
}

/// The init system of a module whose state is [`NoState`]: it makes
/// [`NoState`]. `.build()` stores it when no `.init_state` ran.
pub struct NoInit;

// Generics: `B` module without state, `A` claim.
impl<B: BeanInfo<State = NoState>, A> InitSys<B, A> for NoInit {
    fn start<R>(&self, _full: &mut R) -> NoState
    where
        A: Sees<B, R>,
    {
        NoState::blank()
    }
}

// Generics: `B` module, `A` claim, `H` init handler.
impl<B, A, H> InitSys<B, A> for H
where
    B: Stored<Injects: ClaimShape> + for<'s, 'x> Beans<'s, 'x>,
    for<'s, 'x> A: Sees<B, <B as Beans<'s, 'x>>::Full>,
    H: for<'a> Fn(<A as Claim<B>>::Arg<'a>) -> B::State,
{
    fn start<R>(&self, full: &mut R) -> B::State
    where
        A: Sees<B, R>,
    {
        A::call(full, self)
    }
}
