//! `provide`, `inject`, `module`, and `config` on a [`syn::DeriveInput`].

use syn::ext::IdentExt;
use syn::parse::ParseStream;
use syn::{Attribute, Data, DeriveInput, Fields, Ident, Token, Type};

use crate::claim::{self, Claim, Module};

const MODULE_ATTR: &str = "module";
const CONFIG_ATTR: &str = "config";

/// What `#[derive(Module)]` was applied to.
pub(crate) enum Role {
    /// `State = Self`. Provide and inject lists are empty.
    Data,
    /// A unit struct with claims and an optional state type.
    Module(Box<Module>),
}

pub(crate) fn role(input: &DeriveInput) -> syn::Result<Role> {
    let data = struct_data(input)?;
    reject_generics(input)?;
    let module = module_attrs(&input.attrs)?;
    if matches!(data.fields, Fields::Unit) {
        let module = module.unwrap_or_else(|| Module {
            provide: Vec::new(),
            inject: Vec::new(),
            state: None,
        });
        claim::validate(&module)?;
        return Ok(Role::Module(Box::new(module)));
    }
    if module.is_some() {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "a module is a unit struct",
        ));
    }
    Ok(Role::Data)
}

fn struct_data(input: &DeriveInput) -> syn::Result<&syn::DataStruct> {
    match &input.data {
        Data::Struct(data) => Ok(data),
        Data::Enum(_) | Data::Union(_) => Err(syn::Error::new_spanned(
            &input.ident,
            "Module only supports structs",
        )),
    }
}

fn reject_generics(input: &DeriveInput) -> syn::Result<()> {
    if input.generics.params.is_empty() && input.generics.where_clause.is_none() {
        return Ok(());
    }
    Err(syn::Error::new_spanned(
        &input.generics,
        "Module does not support generic types",
    ))
}

fn module_attrs(attrs: &[Attribute]) -> syn::Result<Option<Module>> {
    let mut provide = None;
    let mut inject = None;
    let mut state = None;
    let mut saw_config = false;
    let mut saw_any = false;
    for attr in attrs {
        if attr.path().is_ident(claim::PROVIDE_ATTR) {
            saw_any = true;
            assign_list(attr, &mut provide, claim::PROVIDE_ATTR)?;
        } else if attr.path().is_ident(claim::INJECT_ATTR) {
            saw_any = true;
            assign_list(attr, &mut inject, claim::INJECT_ATTR)?;
        } else if attr.path().is_ident(MODULE_ATTR) {
            saw_any = true;
            if state.is_some() {
                return Err(syn::Error::new_spanned(
                    attr,
                    format!("duplicate `{MODULE_ATTR}` attribute"),
                ));
            }
            state = Some(parse_state(attr)?);
        } else if attr.path().is_ident(CONFIG_ATTR) {
            saw_any = true;
            if saw_config {
                return Err(syn::Error::new_spanned(
                    attr,
                    format!("duplicate `{CONFIG_ATTR}` attribute"),
                ));
            }
            parse_config(attr)?;
            saw_config = true;
        }
    }
    if !saw_any {
        return Ok(None);
    }
    Ok(Some(Module {
        provide: provide.unwrap_or_default(),
        inject: inject.unwrap_or_default(),
        state,
    }))
}

fn assign_list(attr: &Attribute, slot: &mut Option<Vec<Claim>>, label: &str) -> syn::Result<()> {
    if slot.is_some() {
        return Err(syn::Error::new_spanned(
            attr,
            format!("duplicate `{label}` attribute"),
        ));
    }
    *slot = Some(claim::parse_claims(attr)?);
    Ok(())
}

fn parse_state(attr: &Attribute) -> syn::Result<Type> {
    attr.parse_args_with(|input: ParseStream| {
        let key: Ident = input.parse()?;
        if key != "state" {
            return Err(syn::Error::new_spanned(key, "expected `state = ...`"));
        }
        input.parse::<Token![=]>()?;
        let ty: Type = input.parse()?;
        if !input.is_empty() {
            return Err(input.error("unexpected token in `module`"));
        }
        Ok(ty)
    })
}

fn parse_config(attr: &Attribute) -> syn::Result<()> {
    attr.parse_args_with(|input: ParseStream| {
        // `type` is a keyword, so the key is not an ordinary identifier.
        let key: Ident = input.call(Ident::parse_any)?;
        if key != "type" {
            return Err(syn::Error::new_spanned(key, "expected `type = module`"));
        }
        input.parse::<Token![=]>()?;
        let value: Ident = input.parse()?;
        if value != "module" {
            return Err(syn::Error::new_spanned(value, "expected `type = module`"));
        }
        if !input.is_empty() {
            return Err(input.error("unexpected token in `config`"));
        }
        Ok(())
    })
}
