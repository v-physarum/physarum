//! Repeats a token tree for each index of a finite range.
//!
//! The crate README documents every range form, the `::S` step, placeholders,
//! index bindings, and flat versus fan expansion.

mod body;
mod error;
mod range;

/// Repeats `body` once for each index of a finite `range`.
///
/// ```
/// use ph_util_foreach::impl_foreach;
///
/// impl_foreach!(range = 0..3, body = {
///     pub struct Ix + #;
/// });
///
/// let _ = (Ix0, Ix1, Ix2);
/// ```
///
/// The call is `impl_foreach!(0..3 { .. })`. The trailing block is the body and
/// needs no name. `impl_foreach!(range = 0..3, body = { .. })` still names both
/// arguments, in either order.
/// `#` is the current index. `Name + #` appends that index's decimal digits
/// (`Ix + #` expands to `Ix0`). An identifier immediately followed by `#`
/// is a compile error. `$name = Ident + #;` captures that pasted identifier,
/// `$name = #;` captures the index literal, and later `$name` expands to the
/// captured token, including as a function argument.
///
/// ```
/// #![allow(non_upper_case_globals)]
/// use ph_util_foreach::impl_foreach;
///
/// fn takes_index(index: i128) -> i128 {
///     index
/// }
///
/// impl_foreach!(range = 0..2, body = {
///     $id = Ix + #;
///     $n = #;
///     pub const $id: i128 = $n;
///     pub fn passed + #() -> i128 {
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
/// An outer range must be finite (`A..B` or `A..=B`, with an optional `::S` step).
/// Open ranges such as `0..` or `..4` are valid inside `Name[...]`, where the
/// current domain closes them. A syntax or range error expands to `compile_error!`.
/// See the crate README for the full range table.
#[proc_macro]
pub fn impl_foreach(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = proc_macro2::TokenStream::from(input);
    let output = match body::expand(input) {
        Ok(tokens) => tokens,
        Err(error) => error.to_compile_error(),
    };
    proc_macro::TokenStream::from(output)
}
