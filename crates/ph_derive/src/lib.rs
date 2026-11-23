//! `Module` derive for the `ph_beans` container.
//!
//! A struct with fields is a data bean. A unit struct is a module.

#![forbid(unsafe_code)]

mod claim;
mod config_attr;
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
/// `#[inject(Draw::T as Painter)]` still stores `Draw`. The trait object is
/// the view, and only a trait marked with [`config`] may appear after `as`.
#[proc_macro_derive(Module, attributes(module, provide, inject))]
pub fn derive_module(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand::expand(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Marks a trait as a dyn inject view, or passes a module item through.
///
/// `#[config(type = r#dyn)]` on a trait emits `DynView` for `dyn Trait`.
/// Rustc rejects the keyword `dyn` in this attribute, so the raw identifier
/// is the form that reaches the macro.
/// `#[config(type = module)]` leaves the item unchanged so [`derive_module`]
/// can still see a unit struct.
#[proc_macro_attribute]
pub fn config(attr: TokenStream, item: TokenStream) -> TokenStream {
    match config_attr::expand(attr.into(), item.into()) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
