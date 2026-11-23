//! `#[config(type = dyn)]` marks a trait as an inject view.

use proc_macro2::TokenStream;
use quote::quote;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Item, ItemTrait, Token};

pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let kind = syn::parse2::<ConfigKind>(attr)?;
    let item: Item = syn::parse2(item)?;
    match kind {
        ConfigKind::Module => Ok(quote! { #item }),
        ConfigKind::Dyn => expand_dyn(item),
    }
}

enum ConfigKind {
    Module,
    Dyn,
}

impl Parse for ConfigKind {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let key: Ident = input.call(Ident::parse_any)?;
        if key != "type" {
            return Err(syn::Error::new_spanned(key, "expected `type = ...`"));
        }
        input.parse::<Token![=]>()?;
        let value: Ident = input.parse()?;
        if !input.is_empty() {
            return Err(input.error("unexpected token in `config`"));
        }
        // `dyn` is a keyword, so rustc rejects `type = dyn` before this macro
        // runs. `r#dyn` is the spelling that reaches the macro.
        let name = value.to_string();
        let name = name.trim_start_matches("r#");
        if name == "dyn" {
            Ok(Self::Dyn)
        } else if name == "module" {
            Ok(Self::Module)
        } else {
            Err(syn::Error::new_spanned(
                value,
                "expected `type = r#dyn` or `type = module`",
            ))
        }
    }
}

fn expand_dyn(item: Item) -> syn::Result<TokenStream> {
    let Item::Trait(trait_item) = item else {
        return Err(syn::Error::new_spanned(item, "expected a trait"));
    };
    reject_generic(&trait_item)?;
    if trait_item.unsafety.is_some() {
        return Err(syn::Error::new_spanned(
            &trait_item.ident,
            "dyn inject does not support unsafe traits",
        ));
    }
    let name = &trait_item.ident;
    Ok(quote! {
        #trait_item

        impl<T: #name> ::ph_beans::DynView<T> for dyn #name {}
    })
}

fn reject_generic(trait_item: &ItemTrait) -> syn::Result<()> {
    if trait_item.generics.params.is_empty() && trait_item.generics.where_clause.is_none() {
        return Ok(());
    }
    Err(syn::Error::new_spanned(
        &trait_item.generics,
        "dyn inject does not support generic traits",
    ))
}
