//! Systems a list walk can run.

use core::marker::PhantomData;

use crate::context::ContextTick;
use crate::query::sees::Sees;

/// A system the walk can run with the borrows `R` of one call.
///
/// Generics: `R` borrows lent by the running call.
pub trait MapSys<R> {
    /// Runs the system once.
    fn run(&self, res: &mut R);
}

// A system with no claim runs with no borrows. Path gate trees use this.
// Generics: `H` handler.
impl<H: Fn(())> MapSys<()> for H {
    fn run(&self, _res: &mut ()) {
        self(());
    }
}

/// A system of module `B` and the claim it reads: a `.map` handler, or the
/// init handler that [`Tick::init`](crate::Tick::init) runs.
///
/// Generics: `B` module, `A` claim, `H` handler.
pub struct Sys<B, A, H> {
    pub(crate) handler: H,
    _markers: PhantomData<(B, A)>,
}

// The init slot of a tick holds the init system.
// Generics: `B` module, `A` claim, `H` handler.
impl<B, A, H> ContextTick for Sys<B, A, H> {}

// Generics: `B` module, `A` claim, `H` handler.
impl<B, A, H> Sys<B, A, H> {
    pub(crate) const fn new(handler: H) -> Self {
        Self { handler, _markers: PhantomData }
    }
}

// Generics: `R` borrows, `B` module, `A` claim, `H` handler.
impl<R, B, A, H> MapSys<R> for Sys<B, A, H>
where
    A: Sees<B, R>,
    H: for<'a> Fn(A::Arg<'a>),
{
    fn run(&self, res: &mut R) {
        A::call(res, &self.handler);
    }
}
