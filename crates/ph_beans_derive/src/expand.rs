//! Token generation for [`Module`](crate::derive_module).

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

use crate::claim::{Access, Claim, Module};
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
    let provide = markers(&module.provide);
    let inject = markers(&module.inject);
    let mut beans = inject.clone();
    beans.extend(provide.iter().cloned());
    let provides = list_ty(&provide);
    let injects = list_ty(&inject);
    let beans = list_ty(&beans);
    let state = if let Some(ty) = &module.state {
        quote! { #ty }
    } else {
        quote! { () }
    };
    let at = at_impls(name, &module.provide);
    quote! {
        impl ::ph_beans::BeanInfo for #name {
            type Provides = #provides;
            type Injects = #injects;
            type Beans = #beans;
            type State = #state;
        }

        #at
    }
}

fn markers(claims: &[Claim]) -> Vec<TokenStream> {
    claims.iter().map(marker).collect()
}

fn marker(claim: &Claim) -> TokenStream {
    let bean = &claim.bean;
    match claim.access {
        Access::Shared => quote! { ::ph_beans::Get<#bean> },
        Access::Exclusive => quote! { ::ph_beans::Mut<#bean> },
    }
}

fn list_ty(items: &[TokenStream]) -> TokenStream {
    match items {
        [] => quote! { () },
        [one] => one.clone(),
        many => quote! { (#(#many,)*) },
    }
}

fn at_impls(name: &Ident, provide: &[Claim]) -> TokenStream {
    let impls = provide.iter().enumerate().map(|(index, claim)| {
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
    let name = match index {
        0 => "Ix0",
        1 => "Ix1",
        2 => "Ix2",
        _ => "Ix3",
    };
    format_ident!("{name}")
}
