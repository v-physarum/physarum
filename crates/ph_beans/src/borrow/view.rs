//! Views a claim takes out of a tuple of borrows.
//!
//! An index tree is `()`, one [`Ix`], or a pair of trees. It is
//! [`Claim::Indexes`](crate::Claim). [`TakeView`] walks that tree with
//! [`Take`], so every borrow leaves the tuple at most once.

use super::{Ix, Take};

/// A borrow that can be handed out as `V`: itself, or `&mut` read as `&`.
///
/// Generics: `V` the view asked for.
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `PipelineError<4>` (borrow `{Self}`)",
    label = "fix Descriptor::Status",
    note = "this bean cannot be borrowed as `{V}`; PipelineError<4> — docs/errors.md"
)]
pub trait Lend<V> {
    /// Hands the borrow out.
    fn lend(self) -> V;
}

// Generics: `V` the borrow as it is.
impl<V> Lend<V> for V {
    fn lend(self) -> V {
        self
    }
}

// Generics: `S` borrowed state.
impl<'a, S: ?Sized> Lend<&'a S> for &'a mut S {
    fn lend(self) -> &'a S {
        self
    }
}

/// Takes the borrows an index tree names out of `R`, shaped as view `V`.
///
/// Generics: `R` tuple of borrows, `V` view.
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `PipelineError<4>` (indexes `{Self}`)",
    label = "fix Descriptor::Status",
    note = "a system claims a bean that `{R}` cannot lend as `{V}`; PipelineError<4> — docs/errors.md"
)]
pub trait TakeView<R, V> {
    /// `R` with a hole in every taken slot.
    type Rest;
    /// Splits `refs` into the view and the rest.
    fn take_view(refs: R) -> (V, Self::Rest);
}

// Generics: `R` tuple of borrows.
impl<R> TakeView<R, ()> for () {
    type Rest = R;

    fn take_view(refs: R) -> ((), Self::Rest) {
        ((), refs)
    }
}

// Generics: `R` tuple of borrows, `V` view, `N` slot index.
impl<R, V, const N: usize> TakeView<R, V> for Ix<N>
where
    R: Take<Ix<N>, Out: Lend<V>>,
{
    type Rest = R::Rest;

    fn take_view(refs: R) -> (V, Self::Rest) {
        let (slot, rest) = refs.take();
        (slot.lend(), rest)
    }
}

// Generics: `R` tuple of borrows, `I0`…`I1` index trees, `V0`…`V1` their views.
impl<R, I0, I1, V0, V1> TakeView<R, (V0, V1)> for (I0, I1)
where
    I0: TakeView<R, V0>,
    I1: TakeView<I0::Rest, V1>,
{
    type Rest = I1::Rest;

    fn take_view(refs: R) -> ((V0, V1), Self::Rest) {
        let (left, rest) = I0::take_view(refs);
        let (right, rest) = I1::take_view(rest);
        ((left, right), rest)
    }
}
