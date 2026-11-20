//! `provider` and `inject` lists on a [`syn::DeriveInput`].

use syn::punctuated::Punctuated;
use syn::{Attribute, Data, DeriveInput, Fields, Ident, Meta, Token, Type};

/// Most beans a module may provide. The fourth parent slot is the child.
pub(crate) const PROVIDE_LIMIT: usize = 4;
/// Most beans a module may inject. The remaining parent slot is the module.
pub(crate) const INJECT_LIMIT: usize = 3;

const PROVIDE_ATTR: &str = "provider";
const INJECT_ATTR: &str = "inject";

/// What `#[derive(Component)]` was applied to.
pub(crate) enum Role {
    /// `State = Self` and `create` calls [`Default`](std::default::Default).
    Data,
    /// A unit struct that owns `provider` and receives `inject` from its parent.
    Module(Lists),
}

/// Bean types written on a module.
pub(crate) struct Lists {
    /// Beans this module owns, in slot order.
    pub(crate) provider: Vec<Type>,
    /// Beans taken from the parent, in attribute order.
    pub(crate) inject: Vec<Type>,
}

pub(crate) fn role(input: &DeriveInput) -> syn::Result<Role> {
    let data = struct_data(input)?;
    reject_generics(input)?;
    let lists = lists(&input.attrs)?;
    let Some(lists) = lists else {
        return Ok(Role::Data);
    };
    if !matches!(data.fields, Fields::Unit) {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "a module is a unit struct",
        ));
    }
    validate(&input.ident, &lists)?;
    Ok(Role::Module(lists))
}

fn struct_data(input: &DeriveInput) -> syn::Result<&syn::DataStruct> {
    match &input.data {
        Data::Struct(data) => Ok(data),
        Data::Enum(_) | Data::Union(_) => Err(syn::Error::new_spanned(
            &input.ident,
            "Component only supports structs",
        )),
    }
}

fn reject_generics(input: &DeriveInput) -> syn::Result<()> {
    if input.generics.params.is_empty() && input.generics.where_clause.is_none() {
        return Ok(());
    }
    Err(syn::Error::new_spanned(
        &input.generics,
        "Component does not support generic types",
    ))
}

fn lists(attrs: &[Attribute]) -> syn::Result<Option<Lists>> {
    let mut provider = None;
    let mut inject = None;
    for attr in attrs {
        let Some(name) = list_name(attr) else {
            continue;
        };
        let slot = if name == PROVIDE_ATTR {
            &mut provider
        } else {
            &mut inject
        };
        if slot.is_some() {
            return Err(syn::Error::new_spanned(
                attr,
                format!("duplicate `{name}` attribute"),
            ));
        }
        *slot = Some(parse_list(attr)?);
    }
    match (provider, inject) {
        (None, None) => Ok(None),
        (provider, inject) => Ok(Some(Lists {
            provider: provider.unwrap_or_default(),
            inject: inject.unwrap_or_default(),
        })),
    }
}

fn list_name(attr: &Attribute) -> Option<&'static str> {
    if attr.path().is_ident(PROVIDE_ATTR) {
        Some(PROVIDE_ATTR)
    } else if attr.path().is_ident(INJECT_ATTR) {
        Some(INJECT_ATTR)
    } else {
        None
    }
}

fn parse_list(attr: &Attribute) -> syn::Result<Vec<Type>> {
    let Meta::List(list) = &attr.meta else {
        return Err(syn::Error::new_spanned(
            attr,
            "expected a parenthesized bean list",
        ));
    };
    let parsed = list.parse_args_with(Punctuated::<Type, Token![,]>::parse_terminated)?;
    parsed.into_iter().map(path_type).collect()
}

fn path_type(ty: Type) -> syn::Result<Type> {
    let Type::Path(path) = &ty else {
        return Err(syn::Error::new_spanned(ty, "a bean type must be a path"));
    };
    if path.qself.is_some() || path.path.segments.is_empty() {
        return Err(syn::Error::new_spanned(ty, "a bean type must be a path"));
    }
    Ok(ty)
}

fn validate(name: &Ident, lists: &Lists) -> syn::Result<()> {
    check_len(&lists.provider, PROVIDE_LIMIT, PROVIDE_ATTR)?;
    check_len(&lists.inject, INJECT_LIMIT, INJECT_ATTR)?;
    reject_duplicates(&lists.provider, PROVIDE_ATTR)?;
    reject_duplicates(&lists.inject, INJECT_ATTR)?;
    reject_self(name, &lists.provider, "provide")?;
    reject_self(name, &lists.inject, "inject")?;
    reject_both(lists)
}

fn check_len(list: &[Type], limit: usize, label: &str) -> syn::Result<()> {
    if list.len() <= limit {
        return Ok(());
    }
    Err(syn::Error::new_spanned(
        &list[limit],
        format!("`{label}` accepts at most {limit} beans"),
    ))
}

fn reject_duplicates(list: &[Type], label: &str) -> syn::Result<()> {
    for (index, ty) in list.iter().enumerate() {
        let key = type_key(ty);
        let seen = list[..index].iter().any(|earlier| type_key(earlier) == key);
        if seen {
            return Err(syn::Error::new_spanned(
                ty,
                format!("duplicate bean in `{label}`"),
            ));
        }
    }
    Ok(())
}

fn reject_self(name: &Ident, list: &[Type], verb: &str) -> syn::Result<()> {
    for ty in list {
        if is_self_type(ty, name) {
            return Err(syn::Error::new_spanned(
                ty,
                format!("a module cannot {verb} itself"),
            ));
        }
    }
    Ok(())
}

fn reject_both(lists: &Lists) -> syn::Result<()> {
    for ty in &lists.inject {
        let key = type_key(ty);
        let provided = lists.provider.iter().any(|other| type_key(other) == key);
        if provided {
            return Err(syn::Error::new_spanned(
                ty,
                "bean is in both `provider` and `inject`",
            ));
        }
    }
    Ok(())
}

fn is_self_type(ty: &Type, name: &Ident) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    let Some(segment) = path.path.segments.last() else {
        return false;
    };
    path.path.segments.len() == 1 && segment.ident == *name && segment.arguments.is_none()
}

fn type_key(ty: &Type) -> String {
    use quote::ToTokens;
    ty.to_token_stream().to_string()
}
