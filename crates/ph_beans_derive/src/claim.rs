//! One `Type::T` or `Type::MUT` claim, and the checks on a claim list.

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::{Attribute, Token, Type, TypePath};

/// Most claims a module may provide. `At` stops at `Ix3`.
pub(crate) const PROVIDE_LIMIT: usize = 4;
/// Most claims a module may inject.
pub(crate) const INJECT_LIMIT: usize = 3;

pub(crate) const PROVIDE_ATTR: &str = "provide";
pub(crate) const INJECT_ATTR: &str = "inject";

/// Claims and state written on a module.
pub(crate) struct Module {
    /// Claims this module owns, in slot order.
    pub(crate) provide: Vec<Claim>,
    /// Claims taken from the parent, in attribute order.
    pub(crate) inject: Vec<Claim>,
    /// `#[module(state = ...)]`. Absent means `()`.
    pub(crate) state: Option<Type>,
}

/// One `Type::T` or `Type::MUT` claim.
pub(crate) struct Claim {
    /// Bean the marker names.
    pub(crate) bean: Type,
    /// Shared (`T`) or exclusive (`MUT`).
    pub(crate) access: Access,
}

/// Which marker a claim wrote.
pub(crate) enum Access {
    /// `Type::T`, emitted as `Get`.
    Shared,
    /// `Type::MUT`, emitted as `Mut`.
    Exclusive,
}

pub(crate) fn parse_claims(attr: &Attribute) -> syn::Result<Vec<Claim>> {
    let syn::Meta::List(list) = &attr.meta else {
        return Err(syn::Error::new_spanned(
            attr,
            "expected a parenthesized claim list",
        ));
    };
    let parsed = list.parse_args_with(Punctuated::<Type, Token![,]>::parse_terminated)?;
    parsed.iter().map(split_claim).collect()
}

pub(crate) fn validate(module: &Module) -> syn::Result<()> {
    check_len(&module.provide, PROVIDE_LIMIT, PROVIDE_ATTR)?;
    check_len(&module.inject, INJECT_LIMIT, INJECT_ATTR)?;
    reject_duplicates(&module.provide, PROVIDE_ATTR)?;
    reject_duplicates(&module.inject, INJECT_ATTR)?;
    reject_both(module)
}

fn split_claim(ty: &Type) -> syn::Result<Claim> {
    let Type::Path(type_path) = ty else {
        return Err(syn::Error::new_spanned(ty, "expected Type::T or Type::MUT"));
    };
    if type_path.qself.is_some() {
        return Err(syn::Error::new_spanned(ty, "expected Type::T or Type::MUT"));
    }
    let mut path = type_path.path.clone();
    let Some(last) = path.segments.pop() else {
        return Err(syn::Error::new_spanned(ty, "expected Type::T or Type::MUT"));
    };
    path.segments.pop_punct();
    let last = last.into_value();
    if path.segments.is_empty() || !last.arguments.is_empty() {
        return Err(syn::Error::new_spanned(
            &last,
            "expected Type::T or Type::MUT",
        ));
    }
    let access = if last.ident == "T" {
        Access::Shared
    } else if last.ident == "MUT" {
        Access::Exclusive
    } else {
        return Err(syn::Error::new_spanned(
            &last.ident,
            "expected Type::T or Type::MUT",
        ));
    };
    let bean = Type::Path(TypePath { qself: None, path });
    Ok(Claim { bean, access })
}

fn check_len(list: &[Claim], limit: usize, label: &str) -> syn::Result<()> {
    if list.len() <= limit {
        return Ok(());
    }
    Err(syn::Error::new_spanned(
        &list[limit].bean,
        format!("`{label}` accepts at most {limit} beans"),
    ))
}

fn reject_duplicates(list: &[Claim], label: &str) -> syn::Result<()> {
    for (index, claim) in list.iter().enumerate() {
        let key = type_key(&claim.bean);
        let seen = list[..index]
            .iter()
            .any(|earlier| type_key(&earlier.bean) == key);
        if seen {
            return Err(syn::Error::new_spanned(
                &claim.bean,
                format!("duplicate bean in `{label}`"),
            ));
        }
    }
    Ok(())
}

fn reject_both(module: &Module) -> syn::Result<()> {
    for claim in &module.inject {
        let key = type_key(&claim.bean);
        let provided = module
            .provide
            .iter()
            .any(|other| type_key(&other.bean) == key);
        if provided {
            return Err(syn::Error::new_spanned(
                &claim.bean,
                "bean is in both `provide` and `inject`",
            ));
        }
    }
    Ok(())
}

fn type_key(ty: &Type) -> String {
    let mut tokens = TokenStream::new();
    ty.to_tokens(&mut tokens);
    tokens.to_string()
}
