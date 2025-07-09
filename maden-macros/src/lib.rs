use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, ItemStruct, Ident, LitStr, parse_quote, ImplItem, ItemFn, Attribute, ImplItemFn, Visibility, ReturnType, Type, PathArguments, parse, ItemImpl, Path};

#[proc_macro_attribute]
pub fn handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input_impl = parse_macro_input!(item as ItemImpl);

    let struct_name = if let Type::Path(type_path) = &*input_impl.self_ty {
        type_path.path.segments.last().expect("Expected a struct name").ident.clone()
    } else {
        panic!("#[handler] can only be applied to impl blocks for structs");
    };

    let mut routes_creation = Vec::new();
    let mut original_methods = Vec::new();

    for impl_item in &mut input_impl.items {
        if let ImplItem::Fn(ref mut method) = impl_item {
            let mut is_get_handler = false;
            let mut is_post_handler = false;
            let mut path_lit = None;

            method.attrs.retain(|attr| {
                if attr.path().is_ident("get") {
                    is_get_handler = true;
                    if let Ok(lit) = attr.parse_args::<LitStr>() {
                        path_lit = Some(lit);
                    }
                    false // Remove the attribute from the method
                } else if attr.path().is_ident("post") {
                    is_post_handler = true;
                    if let Ok(lit) = attr.parse_args::<LitStr>() {
                        path_lit = Some(lit);
                    }
                    false // Remove the attribute from the method
                } else {
                    true // Keep other attributes
                }
            });

            if is_get_handler || is_post_handler {
                if let Some(path) = path_lit {
                    let method_name = &method.sig.ident;
                    let http_method = if is_get_handler { quote! { maden_core::HttpMethod::GET } } else { quote! { maden_core::HttpMethod::POST } };
                    routes_creation.push(quote! {
                        routes.push(maden_core::Route {
                            method: #http_method,
                            path: #path.to_string(),
                            handler: Box::new(|req| #struct_name::#method_name(req)),
                        });
                    });
                }
            }
            original_methods.push(ImplItem::Fn(method.clone()));
        } else {
            original_methods.push(impl_item.clone());
        }
    }

    let expanded = quote! {
        #input_impl

        impl #struct_name {
            pub fn routes() -> Vec<maden_core::Route> {
                let mut routes = Vec::new();
                #(#routes_creation)*
                routes
            }
        }
    };

    expanded.into()
}

#[proc_macro_attribute]
pub fn get(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);

    // This macro will simply pass the function through.
    // The #[handler] macro will be responsible for finding this attribute and collecting the route.
    quote! {
        #func
    }.into()
}

#[proc_macro_attribute]
pub fn post(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);

    // This macro will simply pass the function through.
    // The #[handler] macro will be responsible for finding this attribute and collecting the route.
    quote! {
        #func
    }.into()
}

#[proc_macro_attribute]
pub fn application(attr: TokenStream, item: TokenStream) -> TokenStream {
    let handler_path = parse_macro_input!(attr as Path);
    let mut input_fn = parse_macro_input!(item as ItemFn);

    // Ensure the function is async, and remove the async keyword if it's already there
    input_fn.sig.asyncness = None;

    let original_block = input_fn.block;

    let new_block = parse_quote! {
        {
            let config = maden_config::load().expect("Failed to load server configuration");
            let mut router = maden_core::Router::new();

            // Add routes from the specified handler type
            for route in #handler_path::routes() {
                router.add_route(route.method, &route.path, route.handler);
            }

            maden_core::run(config.server, router).await.expect("Failed to run server");

            // Execute original main function body if any
            #original_block
        }
    };

    input_fn.block = new_block;

    let expanded = quote! {
        #[tokio::main]
        async #input_fn
    };

    expanded.into()
}