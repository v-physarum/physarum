//! Token generation for [`Component`](crate::derive_component).

use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};
use syn::{DeriveInput, Lifetime, Type, Visibility};

use crate::parse::{self, Lists, PROVIDE_LIMIT, Role};

const SLOTS: [&str; PROVIDE_LIMIT] = ["Ix0", "Ix1", "Ix2", "Ix3"];

struct Provided {
    ty: Type,
    field: Ident,
    slot: Ident,
}

struct Module {
    vis: Visibility,
    name: Ident,
    state: Ident,
    beans: Ident,
    drive: Ident,
    provider: Vec<Provided>,
    inject: Vec<Type>,
}

pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    match parse::role(input)? {
        Role::Data => Ok(expand_data(&input.vis, &input.ident)),
        Role::Module(lists) => expand_module(&input.vis, &input.ident, &lists),
    }
}

fn expand_data(vis: &Visibility, name: &Ident) -> TokenStream {
    let drive_doc = "Schedules this data bean. A data bean has no children.";
    quote! {
        impl ::ph_beans::Bean for #name {
            type State = Self;

            #[must_use]
            fn create() -> Self::State {
                Self::default()
            }
        }

        impl #name {
            #[doc = #drive_doc]
            #[allow(clippy::inline_always)]
            #[inline(always)]
            #vis fn drive<M, R>(_refs: R) {}
        }
    }
}

fn expand_module(vis: &Visibility, name: &Ident, lists: &Lists) -> syn::Result<TokenStream> {
    let module = Module {
        vis: vis.clone(),
        name: name.clone(),
        state: format_ident!("{}State", name),
        beans: format_ident!("{}Beans", name),
        drive: format_ident!("{}Drive", name),
        provider: provided(lists)?,
        inject: lists.inject.clone(),
    };
    let state = state_struct(&module);
    let beans = beans_alias(&module);
    let bean = bean_impl(&module);
    let at = at_impls(&module);
    let bridge = drive_trait(&module);
    let drive = drive_fn(&module);
    let tick = state_tick(&module);
    Ok(quote! {
        #state
        #beans
        #bean
        #at
        #bridge
        #drive
        #tick
    })
}

fn provided(lists: &Lists) -> syn::Result<Vec<Provided>> {
    let mut slots = Vec::with_capacity(lists.provider.len());
    for (index, ty) in lists.provider.iter().enumerate() {
        let field = field_name(ty)?;
        if slots
            .iter()
            .any(|earlier: &Provided| earlier.field == field)
        {
            return Err(syn::Error::new(
                field.span(),
                "provider types share a field name",
            ));
        }
        slots.push(Provided { ty: ty.clone(), field, slot: slot_name(index, ty)? });
    }
    Ok(slots)
}

fn field_name(ty: &Type) -> syn::Result<Ident> {
    let ident = path_ident(ty)?;
    let snake = snake_case(&ident.to_string());
    Ok(field_ident(&snake, ident.span()))
}

fn path_ident(ty: &Type) -> syn::Result<&Ident> {
    let syn::Type::Path(path) = ty else {
        return Err(syn::Error::new_spanned(ty, "a bean type must be a path"));
    };
    path.path
        .segments
        .last()
        .map(|segment| &segment.ident)
        .ok_or_else(|| syn::Error::new_spanned(ty, "a bean type must be a path"))
}

fn snake_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len().saturating_mul(2));
    let mut chars = name.chars().peekable();
    let mut index = 0usize;
    let mut prev_lower = false;
    while let Some(ch) = chars.next() {
        if ch.is_uppercase() {
            let next_lower = chars.peek().is_some_and(|next| next.is_lowercase());
            if index > 0 && (prev_lower || next_lower) {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
            prev_lower = false;
        } else {
            out.push(ch);
            prev_lower = ch.is_lowercase();
        }
        index += 1;
    }
    out
}

fn field_ident(name: &str, span: proc_macro2::Span) -> Ident {
    if syn::parse_str::<Ident>(name).is_ok() {
        Ident::new(name, span)
    } else {
        Ident::new_raw(name, span)
    }
}

fn slot_name(index: usize, ty: &Type) -> syn::Result<Ident> {
    let name = SLOTS.get(index).copied().ok_or_else(|| {
        syn::Error::new_spanned(
            ty,
            format!("`provider` accepts at most {PROVIDE_LIMIT} beans"),
        )
    })?;
    Ok(format_ident!("{name}"))
}

fn state_struct(module: &Module) -> TokenStream {
    let vis = &module.vis;
    let state = &module.state;
    let doc = format!("Stored beans of `{}`.", module.name);
    if module.provider.is_empty() {
        return quote! {
            #[doc = #doc]
            #vis struct #state;
        };
    }
    let fields = module.provider.iter().map(|slot| {
        let field = &slot.field;
        let ty = &slot.ty;
        quote! { pub #field: <#ty as ::ph_beans::Bean>::State }
    });
    quote! {
        #[doc = #doc]
        #vis struct #state {
            #(#fields,)*
        }
    }
}

fn beans_alias(module: &Module) -> TokenStream {
    let vis = &module.vis;
    let beans = &module.beans;
    let doc = format!(
        "Injected and provided beans of `{}`, as mutable references.",
        module.name
    );
    let refs = ref_types(module);
    let body = tuple_of(&refs);
    let allow = if refs.is_empty() {
        quote! { #[allow(unused_lifetimes, clippy::extra_unused_lifetimes)] }
    } else {
        quote! {}
    };
    quote! {
        #[doc = #doc]
        #allow
        #vis type #beans<'a> = #body;
    }
}

fn ref_types(module: &Module) -> Vec<TokenStream> {
    module
        .inject
        .iter()
        .chain(module.provider.iter().map(|slot| &slot.ty))
        .map(|ty| quote! { &'a mut <#ty as ::ph_beans::Bean>::State })
        .collect()
}

fn tuple_of(elems: &[TokenStream]) -> TokenStream {
    if elems.is_empty() {
        quote! { () }
    } else {
        quote! { (#(#elems,)*) }
    }
}

fn bean_impl(module: &Module) -> TokenStream {
    let name = &module.name;
    let state = &module.state;
    let value = if module.provider.is_empty() {
        quote! { #state }
    } else {
        let fields = module.provider.iter().map(|slot| {
            let field = &slot.field;
            let ty = &slot.ty;
            quote! { #field: <#ty as ::ph_beans::Bean>::create() }
        });
        quote! { #state { #(#fields,)* } }
    };
    quote! {
        impl ::ph_beans::Bean for #name {
            type State = #state;

            #[must_use]
            fn create() -> Self::State {
                #value
            }
        }
    }
}

fn at_impls(module: &Module) -> TokenStream {
    let name = &module.name;
    let impls = module.provider.iter().map(|slot| {
        let ty = &slot.ty;
        let index = &slot.slot;
        quote! {
            impl ::ph_beans::At<#ty> for #name {
                type Ix = ::ph_beans::#index;
            }
        }
    });
    quote! { #(#impls)* }
}

fn drive_trait(module: &Module) -> TokenStream {
    let vis = &module.vis;
    let state = &module.state;
    let drive = &module.drive;
    let doc = format!("Bridge that runs `{state}` from an opaque parent slot.");
    let sig = inject_sig(&module.inject);
    let lifetimes = lifetimes(module.inject.len());
    let (arg, body) = drive_slot_body(&module.inject, state);
    quote! {
        #[doc = #doc]
        #[doc(hidden)]
        #vis trait #drive<Injects> {
            /// Runs the module held in this slot.
            fn drive_slot(self, injects: Injects);
        }

        #[allow(clippy::inline_always)]
        impl<'a #(, #lifetimes)*> #drive<#sig> for &'a mut #state {
            #[inline(always)]
            fn drive_slot(self, #arg: #sig) {
                #body
            }
        }
    }
}

fn drive_slot_body(inject: &[Type], state: &Ident) -> (TokenStream, TokenStream) {
    if inject.is_empty() {
        (quote! { _injects }, quote! { #state::tick(self); })
    } else {
        (quote! { injects }, quote! { #state::tick(self, injects); })
    }
}

fn inject_sig(inject: &[Type]) -> TokenStream {
    if inject.is_empty() {
        return quote! { () };
    }
    let lifetimes = lifetimes(inject.len());
    let types = inject.iter().zip(&lifetimes).map(|(ty, lifetime)| {
        quote! { &#lifetime mut <#ty as ::ph_beans::Bean>::State }
    });
    quote! { (#(#types,)*) }
}

fn lifetimes(count: usize) -> Vec<Lifetime> {
    (0..count)
        .map(|index| Lifetime::new(&format!("'i{index}"), Span::call_site()))
        .collect()
}

fn drive_fn(module: &Module) -> TokenStream {
    let vis = &module.vis;
    let name = &module.name;
    let drive = &module.drive;
    let doc =
        format!("Takes `{name}` and its injected beans out of a parent tuple, then ticks it.");
    let Schedule { bounds, pulls, injects } = schedule(module);
    quote! {
        impl #name {
            #[doc = #doc]
            #[allow(
                clippy::inline_always,
                clippy::similar_names,
                clippy::type_complexity
            )]
            #[inline(always)]
            #vis fn drive<M, R>(refs: R)
            where
                #(#bounds,)*
            {
                #pulls
                #drive::drive_slot(slot, #injects);
            }
        }
    }
}

struct Schedule {
    bounds: Vec<TokenStream>,
    pulls: TokenStream,
    injects: TokenStream,
}

fn schedule(module: &Module) -> Schedule {
    let name = &module.name;
    let child_ix = quote! { <M as ::ph_beans::At<#name>>::Ix };
    let mut ats = vec![quote! { ::ph_beans::At<#name> }];
    for ty in &module.inject {
        ats.push(quote! { ::ph_beans::At<#ty> });
    }
    let mut bounds = vec![
        quote! { M: #(#ats)+* },
        quote! { R: ::ph_beans::Take<#child_ix> },
    ];
    let pulls = pull_slots(&module.inject, &child_ix, &mut bounds);
    let injects = value_tuple(module.inject.len());
    let out = quote! { <R as ::ph_beans::Take<#child_ix>>::Out };
    let drive = &module.drive;
    let args = if module.inject.is_empty() {
        quote! { () }
    } else {
        owned_outs(&module.inject, &child_ix)
    };
    bounds.push(quote! { #out: #drive<#args> });
    Schedule { bounds, pulls, injects }
}

fn pull_slots(
    inject: &[Type],
    child_ix: &TokenStream,
    bounds: &mut Vec<TokenStream>,
) -> TokenStream {
    if inject.is_empty() {
        return quote! {
            let (slot, _rest) = ::ph_beans::Take::<#child_ix>::take(refs);
        };
    }
    let mut pulls = quote! {
        let (slot, rest) = ::ph_beans::Take::<#child_ix>::take(refs);
    };
    let mut rest_ty = quote! { <R as ::ph_beans::Take<#child_ix>>::Rest };
    let last = inject.len() - 1;
    for (index, ty) in inject.iter().enumerate() {
        let binding = format_ident!("inject_{index}");
        let ix = quote! { <M as ::ph_beans::At<#ty>>::Ix };
        bounds.push(quote! { #rest_ty: ::ph_beans::Take<#ix> });
        let next = if index == last {
            format_ident!("_rest")
        } else {
            format_ident!("rest")
        };
        pulls.extend(quote! {
            let (#binding, #next) = ::ph_beans::Take::<#ix>::take(rest);
        });
        rest_ty = quote! { <#rest_ty as ::ph_beans::Take<#ix>>::Rest };
    }
    pulls
}

fn owned_outs(inject: &[Type], child_ix: &TokenStream) -> TokenStream {
    let mut rest_ty = quote! { <R as ::ph_beans::Take<#child_ix>>::Rest };
    let mut outs = Vec::with_capacity(inject.len());
    for ty in inject {
        let ix = quote! { <M as ::ph_beans::At<#ty>>::Ix };
        outs.push(quote! { <#rest_ty as ::ph_beans::Take<#ix>>::Out });
        rest_ty = quote! { <#rest_ty as ::ph_beans::Take<#ix>>::Rest };
    }
    quote! { (#(#outs,)*) }
}

fn value_tuple(count: usize) -> TokenStream {
    if count == 0 {
        quote! { () }
    } else {
        let names = (0..count).map(|index| format_ident!("inject_{index}"));
        quote! { (#(#names,)*) }
    }
}

fn state_tick(module: &Module) -> TokenStream {
    let vis = &module.vis;
    let name = &module.name;
    let state = &module.state;
    let doc = format!("Runs the modules `{name}` provides, then `{name}` itself.");
    let header = tick_header(vis, &module.inject);
    let open = tick_open(&module.inject);
    let drives = module.provider.iter().map(|slot| {
        let ty = &slot.ty;
        let tuple = provide_tuple(&module.provider);
        quote! { <#ty>::drive::<#name, _>(#tuple); }
    });
    let beans = tick_beans(&module.provider, module.inject.len());
    quote! {
        impl #state {
            #[doc = #doc]
            #[allow(clippy::inline_always, clippy::similar_names)]
            #[inline(always)]
            #header {
                #open
                #(#drives)*
                <#name as ::ph_beans::Tick>::tick(#beans);
            }
        }
    }
}

fn tick_header(vis: &Visibility, inject: &[Type]) -> TokenStream {
    if inject.is_empty() {
        quote! { #vis fn tick(&mut self) }
    } else {
        let sig = elided_inject(inject);
        quote! { #vis fn tick(&mut self, injects: #sig) }
    }
}

fn elided_inject(inject: &[Type]) -> TokenStream {
    let types = inject.iter().map(|ty| {
        quote! { &mut <#ty as ::ph_beans::Bean>::State }
    });
    quote! { (#(#types,)*) }
}

fn tick_open(inject: &[Type]) -> TokenStream {
    if inject.is_empty() {
        quote! {}
    } else {
        let value = value_tuple(inject.len());
        quote! { let #value = injects; }
    }
}

fn provide_tuple(provider: &[Provided]) -> TokenStream {
    let fields = provider.iter().map(|slot| {
        let field = &slot.field;
        quote! { &mut self.#field }
    });
    quote! { (#(#fields,)*) }
}

fn tick_beans(provider: &[Provided], injects: usize) -> TokenStream {
    let names = (0..injects).map(|index| {
        let name = format_ident!("inject_{index}");
        quote! { #name }
    });
    let fields = provider.iter().map(|slot| {
        let field = &slot.field;
        quote! { &mut self.#field }
    });
    let elems: Vec<TokenStream> = names.chain(fields).collect();
    tuple_of(&elems)
}
