//! `Component` derive for the `ph_beans` container.
//!
//! A struct with `provider` or `inject` is a module. A struct with neither is a
//! data bean.

#![forbid(unsafe_code)]

mod expand;
mod parse;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

/// Stores a data bean as itself, or a module as the beans it provides.
///
/// `#[provider(A, B)]` owns `A` and `B`. `#[inject(A)]` receives `A` from the
/// parent module. Provider accepts at most four types. Inject accepts at most
/// three. A module is a unit struct.
#[proc_macro_derive(Component, attributes(provider, inject))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand::expand(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
