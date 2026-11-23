//! `Module` derive for the `ph_beans` container.
//!
//! A struct with fields is a data bean. A unit struct is a module.

#![forbid(unsafe_code)]

mod claim;
mod expand;
mod parse;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

/// Emits `BeanInfo` and `At` for a bean or a module.
///
/// A struct with fields is a data bean: `State` is `Self`, and the provide
/// and inject lists are empty. A unit struct is a module.
/// `#[provide(Id::T, Keys::MUT)]` owns those beans. `#[inject(App::T)]`
/// receives that bean from the parent. The marker is syntax: the lists store
/// the concrete bean. `Beans` is the provides followed by the injects.
/// `#[module(state = u8)]` sets the owned state. `#[config(type = module)]`
/// marks a unit struct as a module. `Beans` accepts at most four beans, and
/// each one gets `At`. Each claim is `Type::T` or `Type::MUT`.
#[proc_macro_derive(Module, attributes(config, module, provide, inject))]
pub fn derive_module(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand::expand(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
