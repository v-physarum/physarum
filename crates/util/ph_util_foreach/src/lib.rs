//! Repeats a token tree for each index of a finite range.
//!
//! The crate README documents every range form, `(range).step_by(step)`,
//! `paste`, index bindings, and flat versus fan expansion.

mod body;
mod error;
mod range;

/// Repeats `body` once for each index of a finite `range`.
///
/// ```
/// use ph_util_foreach::impl_foreach;
///
/// impl_foreach!(n in 0..3 {
///     pub struct paste(Ix, n);
/// });
///
/// let _ = (Ix0, Ix1, Ix2);
/// ```
///
/// The caller names the loop variable. The call is `impl_foreach!(n in 0..3 { .. })`.
/// A comma before the block is optional. `paste(Ix, n)` appends the index's
/// decimal digits (`paste(Ix, n)` expands to `Ix0`). `$name = paste(Ix, n);`
/// captures that identifier, `$name = n;` captures the index literal, and a
/// later `$name` expands to the captured token, including as a function argument.
///
/// ```
/// #![allow(non_upper_case_globals)]
/// use ph_util_foreach::impl_foreach;
///
/// fn takes_index(index: i128) -> i128 {
///     index
/// }
///
/// impl_foreach!(n in 0..2 {
///     $id = paste(Ix, n);
///     $k = n;
///     pub const $id: i128 = $k;
///     pub fn paste(passed, n)() -> i128 {
///         takes_index($id)
///     }
/// });
///
/// assert_eq!(passed0(), 0);
/// assert_eq!(passed1(), 1);
/// ```
/// `Name[0..4]` expands to the comma-separated identifiers `Name0, Name1, Name2, Name3`.
/// Sequence pieces concatenate only with `+` (`X + Name[..]`). `X Name[..]` is a
/// compile error.
///
/// An outer range must be finite (`A..B`, `A..=B`, or `(A..B).step_by(S)`).
/// Parentheses around the range are required for `step_by`. Open ranges such as
/// `0..` or `..4` are valid inside `Name[...]`, where the current domain closes
/// them. A syntax or range error expands to `compile_error!`. See the crate
/// README for the full range table.
#[proc_macro]
pub fn impl_foreach(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = proc_macro2::TokenStream::from(input);
    let output = match body::expand(input) {
        Ok(tokens) => tokens,
        Err(error) => error.to_compile_error(),
    };
    proc_macro::TokenStream::from(output)
}
