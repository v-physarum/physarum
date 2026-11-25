//! A claim viewed from the borrows of one call.

use crate::borrow::TakeView;
use crate::core::each::Each;
use crate::query::claim::Claim;
use crate::query::lend::Reborrow;

/// Claim `Self` of module `B` can be viewed from the borrows `R`.
///
/// Generics: `B` module, `R` tuple of borrows over `B::Beans`.
pub trait Sees<B, R>: Claim<B> {
    /// Lends `beans` to `system` for one call.
    ///
    /// Generics: `H` system, `O` its result.
    fn call<H, O>(beans: &mut R, system: &H) -> O
    where
        H: for<'a> Fn(Self::Arg<'a>) -> O;
}

// Generics: `B` module, `R` borrows, `A` claim, `I` its index tree.
impl<B, R, A, I> Sees<B, R> for A
where
    A: Claim<B, Indexes = I>,
    for<'a> &'a mut R: Each<Reborrow>,
    for<'a> I: TakeView<<&'a mut R as Each<Reborrow>>::Out, A::Seen<'a>>,
{
    fn call<H, O>(beans: &mut R, system: &H) -> O
    where
        H: for<'a> Fn(Self::Arg<'a>) -> O,
    {
        let (view, _rest) = I::take_view(beans.each());
        system(A::arg(view))
    }
}
