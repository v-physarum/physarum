//! Local operator status for one claim list.

use crate::descriptor::Valid;
use crate::descriptor::error::ClaimError;

/// Per-operator lens: the status of one claim list read under module `B`.
///
/// Pipeline [`crate::Descriptor`] stays the tip status. The one impl is in
/// `query/marks.rs`: the status is computed over `B`'s slots.
/// Generics: `B` module.
pub trait OpDesc<B> {
    /// `Valid`, or the first clash as a [`ClaimError`].
    type Status;
}

/// `Status` may pass the next gated step (`.inject_resources`, init, build).
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `{Self}`",
    label = "fix Descriptor::Status",
    note = "`ClaimError<T, 1>` — same bean claimed twice; \
            `ClaimError<T, 2>` — `T` and `Mut` on one bean; \
            codes: docs/errors.md"
)]
pub trait OpOk {}

impl OpOk for Valid {}

/// Left claim status combined with right. Generics: `R` right status.
#[doc(hidden)]
pub trait JoinStatus<R> {
    /// Combined status (first error wins).
    type Out;
}

impl JoinStatus<Valid> for Valid {
    type Out = Valid;
}

// Generics: `T` bean on the right error.
impl<T> JoinStatus<ClaimError<T, 1>> for Valid {
    type Out = ClaimError<T, 1>;
}

// Generics: `T` bean on the right error.
impl<T> JoinStatus<ClaimError<T, 2>> for Valid {
    type Out = ClaimError<T, 2>;
}

// Generics: `T` bean on the left error.
impl<T, R> JoinStatus<R> for ClaimError<T, 1> {
    type Out = ClaimError<T, 1>;
}

// Generics: `T` bean on the left error.
impl<T, R> JoinStatus<R> for ClaimError<T, 2> {
    type Out = ClaimError<T, 2>;
}
