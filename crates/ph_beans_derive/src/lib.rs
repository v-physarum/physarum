//! `Module` derive for the `ph_beans` container.
//!
//! A struct with fields is a data bean. A unit struct is a module.

#![forbid(unsafe_code)]

mod claim;
mod expand;
mod parse;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

/// Emits `BeanInfo` for a bean or a module.
///
/// A struct with fields is a data bean: `State` is `Self`, and the provide
/// and inject lists are empty. A unit struct is a module.
/// `#[provide(Id::T, Keys::MUT)]` owns those claims. `#[inject(App::T)]`
/// receives that claim from the parent. `#[module(state = u8)]` sets the
/// owned state. `#[config(type = module)]` marks a unit struct as a module.
/// Provide accepts at most four claims. Inject accepts at most three.
/// Each claim is `Type::T` (shared) or `Type::MUT` (exclusive).
#[proc_macro_derive(Module, attributes(config, module, provide, inject))]
pub fn derive_module(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand::expand(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
