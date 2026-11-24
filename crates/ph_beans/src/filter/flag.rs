//! Wait markers used by filter shape checks.

/// The filter contains a `Mut` wait marker.
pub struct Yes;

/// The filter does not wait.
pub struct No;

/// Combines two wait flags (`A | B`). Policy on the left flag — not a 2×2 table.
/// Generics: `R` right flag.
#[doc(hidden)]
pub trait OrWith<R> {
    /// `Yes` if either side waited.
    type Out;
}

impl OrWith<No> for No {
    type Out = No;
}

impl OrWith<Yes> for No {
    type Out = Yes;
}

impl<R> OrWith<R> for Yes {
    type Out = Yes;
}
