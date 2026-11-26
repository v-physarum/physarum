//! What `.forward` takes: one child marker, or several joined with `&`.
//!
//! The marker names the access the module provides the child under:
//! `P::T` a child under `Get`, `P::MUT` a child under `Mut` ([`Gives`]).

use crate::context::Context;
use crate::descriptor::{Exclusive, Get, Mut, Provided, Shared};
use crate::filter::And;
use crate::query::dyn_claim::Holds;
use crate::query::forward::SealForward;
use crate::query::store::ReadOnly;

/// Module `Self` provides bean `P` under access `A`.
///
/// Generics: `P` child bean, `A` the access the `.forward` marker names.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `PipelineError<8>` (child `{P}`)",
    label = "fix Descriptor::Status",
    note = "the marker of `.forward` is not the marker `{Self}` provides the child under; PipelineError<8> — docs/errors.md"
)]
pub trait Gives<P, A> {}

// Generics: `B` module, `P` child, `E` the entry of `P` in `B`, `A` access.
#[diagnostic::do_not_recommend]
impl<B, P, E, A> Gives<P, A> for B
where
    B: Holds<P, Entry = E>,
    E: Provided<Access = A>,
{
}

/// Children markers `Self` pushed onto context `C`.
///
/// The impls are on the marker itself, so an argument that is not yet
/// inferred is ambiguous at once and the solver does not recurse.
/// Generics: `C` context.
#[doc(hidden)]
pub trait Forwards<C> {
    /// Context after the children.
    type Out: Context;
    /// Stores the children, left to right.
    fn push_all(ctx: C) -> Self::Out;
}

// `P::MUT`: a child provided under `Mut`. One impl per marker: a blanket
// over both would overlap the `And` impl.
// Generics: `C` context, `P` child bean, `B` module.
impl<C, P, B> Forwards<C> for Mut<P>
where
    C: Context<Bean = B> + SealForward<P>,
    B: Gives<P, Exclusive>,
{
    type Out = C::Out;

    fn push_all(ctx: C) -> Self::Out {
        ctx.seal_forward()
    }
}

// `P::T`: a child provided under `Get`. It changes nothing of its own
// ([`ReadOnly`]).
// Generics: `C` context, `P` child bean, `B` module.
impl<C, P, B> Forwards<C> for Get<P>
where
    C: Context<Bean = B> + SealForward<P>,
    B: Gives<P, Shared>,
    P: ReadOnly,
{
    type Out = C::Out;

    fn push_all(ctx: C) -> Self::Out {
        ctx.seal_forward()
    }
}

// `X & Y`: the children of `X`, then the children of `Y`.
// Generics: `C` context, `X` left children, `Y` right children, `M` the
// context after `X`.
impl<C, X, Y, M> Forwards<C> for And<X, Y>
where
    X: Forwards<C, Out = M>,
    Y: Forwards<M>,
{
    type Out = Y::Out;

    fn push_all(ctx: C) -> Self::Out {
        Y::push_all(X::push_all(ctx))
    }
}
