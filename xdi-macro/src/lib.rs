use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{ToTokens, quote};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{Expr, ExprArray, Lit, PatLit};
use syn::{ItemFn, parse_macro_input};

#[proc_macro_attribute]
pub fn register_constructor(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Парсим аргументы как key = value через запятую
    let args =
        parse_macro_input!(attr with Punctuated::<syn::MetaNameValue, Comma>::parse_terminated);

    let args = args.into_iter().collect::<Vec<syn::MetaNameValue>>();

    let inject_scope = args
        .iter()
        .find(|x| x.path.get_ident().is_some_and(|x| x.to_string() == "scope"))
        .cloned();

    let maps = args
        .iter()
        .find(|x| x.path.get_ident().is_some_and(|x| x.to_string() == "map"))
        .cloned();

    let maps = maps
        .as_ref()
        .and_then(|x| {
            if let Expr::Array(ExprArray { elems, .. }) = &x.value {
                Some(elems.iter().collect::<Vec<_>>())
            } else {
                None
            }
        })
        .unwrap_or_default();

    let maps_quote = maps
        .iter()
        .map(|map| quote! { builder.map_as_trait::<dyn #map>(); })
        .collect::<Vec<_>>();

    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;

    let crate_name = proc_macro_crate::crate_name("xdi").expect("Failed to get crate name for xdi");

    let crate_name = match crate_name {
        proc_macro_crate::FoundCrate::Name(name) => Ident::new(&name, Span::call_site()),
        proc_macro_crate::FoundCrate::Itself => Ident::new("crate", Span::call_site()),
    };

    let scope = inject_scope.as_ref().and_then(|x| {
        if let Expr::Lit(PatLit {
            lit: Lit::Str(val), ..
        }) = &x.value
        {
            Some(val.value())
        } else {
            None
        }
    });

    if let Some(inject_scope) = inject_scope
        && scope.is_none()
    {
        panic!(
            r#"Invalid scope value in register_constructor: {:?}, expected: "singleton", "transient", "task_local", "thread_local""#,
            inject_scope.value.to_token_stream()
        );
    }

    if scope.as_ref().is_some_and(|x| x == "singleton") {
        let expanded = quote! {
            #input_fn

            inventory::submit! {
                #crate_name::Registration {
                    constructor: &|builder| {
                        let builder = builder.singletone(#fn_name);

                        #(#maps_quote)*
                    }
                }
            }
        };

        return expanded.into();
    }

    if scope.as_ref().is_some_and(|x| x == "thread_local") {
        let expanded = quote! {
            #input_fn

            inventory::submit! {
                #crate_name::Registration {
                    constructor: &|builder| {
                        let builder = builder.thread_local(#fn_name);

                        #(#maps_quote)*
                    }
                }
            }
        };

        return expanded.into();
    }

    if scope.as_ref().is_some_and(|x| x == "task_local") {
        let expanded = quote! {
            #input_fn

            inventory::submit! {
                #crate_name::Registration {
                    constructor: &|builder| {
                        let builder = builder.task_local(#fn_name);

                        #(#maps_quote)*
                    }
                }
            }
        };

        return expanded.into();
    }

    if scope.as_ref().is_none_or(|x| x == "transient") {
        let expanded = quote! {
            #input_fn

            inventory::submit! {
                #crate_name::Registration {
                    constructor: &|builder| {
                        let builder = builder.transient(#fn_name);

                        #(#maps_quote)*
                    }
                }
            }
        };

        return expanded.into();
    }

    panic!("Unsupported inject scope: {:?}", scope);
}
