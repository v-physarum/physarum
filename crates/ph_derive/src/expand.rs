//! Token generation for [`Module`](crate::derive_module).

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

use crate::claim::{Claim, Module};
use crate::parse::{self, Role};

pub(crate) fn expand(input: &syn::DeriveInput) -> syn::Result<TokenStream> {
    match parse::role(input)? {
        Role::Data => Ok(expand_data(&input.ident)),
        Role::Module(module) => Ok(expand_module(&input.ident, &module)),
    }
}

fn expand_data(name: &Ident) -> TokenStream {
    quote! {
        impl ::ph_beans::BeanInfo for #name {
            type Provides = ();
            type Injects = ();
            type Beans = ();
            type State = Self;
        }
    }
}

fn expand_module(name: &Ident, module: &Module) -> TokenStream {
    let provide = concrete(&module.provide);
    let inject = concrete(&module.inject);
    let mut beans = provide.clone();
    beans.extend(inject.iter().cloned());
    let provides = list_ty(&provide);
    let injects = list_ty(&inject);
    let beans = list_ty(&beans);
    let state = if let Some(ty) = &module.state {
        quote! { #ty }
    } else {
        quote! { () }
    };
    let at = at_impls(name, &module.provide, &module.inject);
    let views = view_checks(&module.inject);
    quote! {
        impl ::ph_beans::BeanInfo for #name {
            type Provides = #provides;
            type Injects = #injects;
            type Beans = #beans;
            type State = #state;
        }

        #at
        #views
    }
}

fn view_checks(inject: &[Claim]) -> TokenStream {
    let checks = inject.iter().filter_map(|claim| {
        let view = claim.view.as_ref()?;
        let bean = &claim.bean;
        Some(quote! {
            const _: () = {
                const fn assert_dyn<T, V>()
                where
                    V: ::ph_beans::DynView<T> + ?Sized,
                {
                }
                assert_dyn::<#bean, dyn #view>();
            };
        })
    });
    quote! { #(#checks)* }
}

fn concrete(claims: &[Claim]) -> Vec<TokenStream> {
    claims
        .iter()
        .map(|claim| {
            let bean = &claim.bean;
            quote! { #bean }
        })
        .collect()
}

fn list_ty(items: &[TokenStream]) -> TokenStream {
    if items.is_empty() {
        quote! { () }
    } else {
        quote! { (#(#items,)*) }
    }
}

fn at_impls(name: &Ident, provide: &[Claim], inject: &[Claim]) -> TokenStream {
    let impls = provide
        .iter()
        .chain(inject)
        .enumerate()
        .map(|(index, claim)| {
            let bean = &claim.bean;
            let ix = index_ident(index);
            quote! {
                impl ::ph_beans::At<#bean> for #name {
                    type Ix = ::ph_beans::#ix;
                }
            }
        });
    quote! { #(#impls)* }
}

fn index_ident(index: usize) -> Ident {
    const NAMES: [&str; 4] = ["Ix0", "Ix1", "Ix2", "Ix3"];
    format_ident!("{}", NAMES[index])
}
