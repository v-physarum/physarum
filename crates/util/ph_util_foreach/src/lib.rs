//! Repeats a token tree for each index of a finite range.
//!
//! The crate README documents every range form, the `::S` step, placeholders,
//! and flat versus fan expansion.

mod body;
mod range;

#[cfg(test)]
mod tests;

use proc_macro2::TokenStream;

/// Repeats `body` once for each index of a finite `range`.
///
/// ```
/// use ph_util_foreach::impl_foreach;
///
/// impl_foreach!(0..3 {
///     pub struct Ix + #;
/// });
///
/// let _ = (Ix0, Ix1, Ix2);
/// ```
///
/// The range and the `{ ... }` body both sit inside the macro delimiter.
/// `#` is the current index. `Name + #` appends that index's decimal digits
/// (`Ix + #` expands to `Ix0`). An identifier immediately followed by `#`
/// is a compile error.
/// `Name[0..4]` expands to the comma-separated identifiers `Name0, Name1, Name2, Name3`.
///
/// An outer range must be finite (`A..B` or `A..=B`, with an optional `::S` step).
/// Open ranges such as `0..` or `..4` are valid inside `Name[...]`, where the
/// current domain closes them. A syntax or range error expands to `compile_error!`.
/// See the crate README for the full range table.
#[proc_macro]
pub fn impl_foreach(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match body::expand(TokenStream::from(input)) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
