//! Wait markers used by filter shape checks.

/// The filter contains a `Mut` wait marker.
pub struct Yes;

/// The filter does not wait.
pub struct No;

#[doc(hidden)]
pub trait OrFlag {
    /// `Yes` if either side waited.
    type Out;
}

impl OrFlag for (No, No) {
    type Out = No;
}

impl OrFlag for (No, Yes) {
    type Out = Yes;
}

impl OrFlag for (Yes, No) {
    type Out = Yes;
}

impl OrFlag for (Yes, Yes) {
    type Out = Yes;
}
