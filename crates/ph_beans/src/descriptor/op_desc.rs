//! Local operator status for one claim list.

use core::marker::PhantomData;

use crate::descriptor::Valid;

/// Per-operator lens. Pipeline [`crate::Descriptor`] stays the tip status.
///
/// Implemented for [`Claim`](crate::Claim) in `query/args.rs` (`Status` = claim status).
pub trait OpDesc {
    /// `Valid`, or a named claim error.
    type Status;
}

/// `Status` may pass the next gated step (`.inject_resources`, init, build).
#[diagnostic::on_unimplemented(
    message = "operator invalid: Status=`{Self}`",
    label = "fix OpDesc::Status",
    note = "`ErrDuplicateResource<T>` — same bean claimed twice; \
            `ErrConflictAccess<T>` — `T` and `Mut` on one bean"
)]
pub trait OpOk {}

impl OpOk for Valid {}

/// The same bean was claimed twice. Generics: `T` bean.
pub struct ErrDuplicateResource<T>(PhantomData<T>);

/// Shared and exclusive claims on one bean. Generics: `T` bean.
pub struct ErrConflictAccess<T>(PhantomData<T>);

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
impl<T> JoinStatus<ErrDuplicateResource<T>> for Valid {
    type Out = ErrDuplicateResource<T>;
}

// Generics: `T` bean on the right error.
impl<T> JoinStatus<ErrConflictAccess<T>> for Valid {
    type Out = ErrConflictAccess<T>;
}

// Generics: `T` bean on the left error.
impl<T, R> JoinStatus<R> for ErrDuplicateResource<T> {
    type Out = ErrDuplicateResource<T>;
}

// Generics: `T` bean on the left error.
impl<T, R> JoinStatus<R> for ErrConflictAccess<T> {
    type Out = ErrConflictAccess<T>;
}
