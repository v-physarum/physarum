//! One `Type::T` or `Type::MUT` claim, and the checks on a claim list.

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Attribute, Token, Type, TypePath};

/// Most concrete beans in `Beans` (`Provides` then `Injects`). `At` stops at `Ix3`.
pub(crate) const BEAN_LIMIT: usize = 4;

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
///
/// The marker is required syntax. `BeanInfo` stores the concrete bean.
pub(crate) struct Claim {
    /// Bean the marker names.
    pub(crate) bean: Type,
    /// `as Trait` on an inject. Absent for a concrete claim.
    pub(crate) view: Option<Type>,
}

pub(crate) fn parse_claims(attr: &Attribute) -> syn::Result<Vec<Claim>> {
    let syn::Meta::List(list) = &attr.meta else {
        return Err(syn::Error::new_spanned(
            attr,
            "expected a parenthesized claim list",
        ));
    };
    let parsed = list.parse_args_with(Punctuated::<RawClaim, Token![,]>::parse_terminated)?;
    parsed.iter().map(split_claim).collect()
}

struct RawClaim {
    ty: Type,
    view: Option<Type>,
}

impl Parse for RawClaim {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let ty = input.parse()?;
        let view = if input.peek(Token![as]) {
            input.parse::<Token![as]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        Ok(Self { ty, view })
    }
}

pub(crate) fn validate(module: &Module) -> syn::Result<()> {
    check_total(module)?;
    reject_provide_cast(module)?;
    reject_duplicates(&module.provide, PROVIDE_ATTR)?;
    reject_duplicates(&module.inject, INJECT_ATTR)?;
    reject_both(module)
}

fn split_claim(raw: &RawClaim) -> syn::Result<Claim> {
    let claim = split_marker(&raw.ty)?;
    Ok(Claim { view: raw.view.clone(), ..claim })
}

fn split_marker(ty: &Type) -> syn::Result<Claim> {
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
    if last.ident != "T" && last.ident != "MUT" {
        return Err(syn::Error::new_spanned(
            &last.ident,
            "expected Type::T or Type::MUT",
        ));
    }
    let bean = Type::Path(TypePath { qself: None, path });
    Ok(Claim { bean, view: None })
}

fn reject_provide_cast(module: &Module) -> syn::Result<()> {
    for claim in &module.provide {
        if let Some(view) = &claim.view {
            return Err(syn::Error::new_spanned(
                view,
                "cast is not allowed in `provide`",
            ));
        }
    }
    Ok(())
}

fn check_total(module: &Module) -> syn::Result<()> {
    let total = module.provide.len() + module.inject.len();
    if total <= BEAN_LIMIT {
        return Ok(());
    }
    let claim = if module.provide.len() > BEAN_LIMIT {
        &module.provide[BEAN_LIMIT]
    } else {
        &module.inject[BEAN_LIMIT - module.provide.len()]
    };
    Err(syn::Error::new_spanned(
        &claim.bean,
        format!("`Beans` accepts at most {BEAN_LIMIT} beans"),
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
