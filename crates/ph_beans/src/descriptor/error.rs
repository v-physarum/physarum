//! Named error ZSTs with numeric codes.
//!
//! Code table: [`docs/errors.md`](../../docs/errors.md).

use core::marker::PhantomData;

/// Filter / emit / wait path failure.
///
/// Generics: `V` code — see [`docs/errors.md`](../../docs/errors.md).
pub struct FilterError<const V: usize>;

/// Pipeline step failure (init / map cap).
///
/// Generics: `V` code — see [`docs/errors.md`](../../docs/errors.md).
pub struct PipelineError<const V: usize>;

/// Claim-list failure for bean `T`.
///
/// Generics: `T` bean, `V` code — see [`docs/errors.md`](../../docs/errors.md).
pub struct ClaimError<T, const V: usize>(PhantomData<T>);
