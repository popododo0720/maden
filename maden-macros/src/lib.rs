use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ImplItem, ItemFn, ItemImpl, LitStr, Type, Ident};
use syn::parse::{Parse, ParseBuffer};

struct HandlerArgs {
    path: LitStr,
    query: Option<LitStr>,
}

impl Parse for HandlerArgs {
    fn parse(input: &ParseBuffer) -> syn::Result<Self> {
        let path: LitStr = input.parse()?;
        let mut query: Option<LitStr> = None;

        if input.peek(syn::Token![,]) {
            input.parse::<syn::Token![,]>()?;
            let nv: syn::MetaNameValue = input.parse()?;
            if nv.path.is_ident("query") {
                if let syn::Expr::Lit(expr_lit) = nv.value {
                    if let syn::Lit::Str(lit_str) = expr_lit.lit {
                        query = Some(lit_str);
                    } else {
                        return Err(input.error("expected string literal for query value"));
                    }
                } else {
                    return Err(input.error("expected literal for query value"));
                }
            } else {
                return Err(input.error("expected `query` argument"));
            }
        }

        Ok(HandlerArgs { path, query })
    }
}

#[proc_macro_attribute]
pub fn handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input_impl = parse_macro_input!(item as ItemImpl);

    let struct_name = if let Type::Path(type_path) = &*input_impl.self_ty {
        type_path
            .path
            .segments
            .last()
            .expect("Expected a struct name")
            .ident
            .clone()
    } else {
        panic!("#[handler] can only be applied to impl blocks for structs");
    };

    let mut routes_registration = Vec::new();

    for impl_item in &mut input_impl.items {
        if let ImplItem::Fn(method) = impl_item {
            let method_name = &method.sig.ident;
            let mut http_method = None;
            let mut path_str = None;
            let mut query_str = None;

            method.attrs.retain(|attr| {
                let mut is_handled = false;
                if attr.path().is_ident("get") || attr.path().is_ident("post") {
                    if let Ok(args) = attr.parse_args::<HandlerArgs>() {
                        path_str = Some(args.path.value());
                        query_str = args.query.map(|q| q.value());

                        if attr.path().is_ident("get") {
                            http_method = Some(quote! { maden_core::HttpMethod::Get });
                        } else {
                            http_method = Some(quote! { maden_core::HttpMethod::Post });
                        }
                        is_handled = true;
                    }
                }
                !is_handled
            });

            if let (Some(http_method), Some(path)) = (http_method, path_str) {
                let query_arg = if let Some(qs) = query_str {
                    quote! { Some(#qs) }
                } else {
                    quote! { None }
                };

                let return_type = &method.sig.output;
                let response_conversion = match return_type {
                    syn::ReturnType::Default => { // -> ()
                        quote! { Ok(().into_response()) }
                    },
                    syn::ReturnType::Type(_, ty) => { // -> Type
                        if let Type::Path(type_path) = &**ty {
                            let last_segment = type_path.path.segments.last().unwrap();
                            let type_name = &last_segment.ident;

                            // Check for maden_core::Response, String, &'static str
                            if type_name == "Response" || type_name == "String" || type_name == "str" {
                                quote! { Ok(#struct_name::#method_name(req).await.into_response()) }
                            } else if let Type::Path(type_path) = &**ty {
                                if type_path.path.segments.last().unwrap().ident == "Result" {
                                    // Handle Result<T, MadenError>
                                    // Extract T from Result<T, MadenError>
                                    let _inner_type = if let syn::PathArguments::AngleBracketed(args) = &type_path.path.segments.last().unwrap().arguments {
                                        if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                                            inner_ty
                                        } else {
                                            panic!("Expected a type argument for Result");
                                        }
                                    } else {
                                        panic!("Expected angle bracketed arguments for Result");
                                    };

                                    quote! {
                                        match #struct_name::#method_name(req).await {
                                            Ok(value) => maden_core::Response::new(200).json(value),
                                            Err(err) => err.into_response(),
                                        }
                                    }
                                } else if type_path.path.segments.last().unwrap().ident == "Response" {
                                    quote! { #struct_name::#method_name(req).await }
                                } else if type_path.path.segments.last().unwrap().ident == "String" {
                                    quote! { maden_core::Response::new(200).text(&#struct_name::#method_name(req).await) }
                                } else if type_path.path.segments.last().unwrap().ident == "str" {
                                    quote! { maden_core::Response::new(200).text(#struct_name::#method_name(req).await) }
                                } else {
                                    // Assume it's a serializable type
                                    quote! { maden_core::Response::new(200).json(#struct_name::#method_name(req).await) }
                                }
                            } else {
                                // Fallback for other complex types or impl Trait
                                quote! { maden_core::Response::new(200).json(#struct_name::#method_name(req).await) }
                            }
                        } else {
                            // For other complex types (e.g., tuples, arrays),
                            // or impl Trait, fallback to IntoResponse
                            quote! { Ok(#struct_name::#method_name(req).await.into_response()) }
                        }
                    }
                };

                routes_registration.push(quote! {
                    maden.add_route(
                        #http_method,
                        &#path,
                        #query_arg,
                        Box::new(|req| Box::pin(async move { #response_conversion })),
                    );
                });
            }
        }
    }

    let add_routes_fn_name = Ident::new(
        &format!("add_routes_{}", struct_name.to_string().to_snake_case()),
        struct_name.span(),
    );

    let expanded = quote! {
        #input_impl

        pub fn #add_routes_fn_name(maden: &mut maden_core::Maden) {
            use maden_core::IntoResponse; // 여기에 추가
            #(#routes_registration)*
        }

        inventory::submit! {
            maden_core::HandlerFactory(#add_routes_fn_name)
        }
    };

    expanded.into()
}

#[proc_macro_attribute]
pub fn get(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn post(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn application(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;

    let expanded = quote! {
        #[tokio::main]
        async fn #fn_name() {
            maden_log::init(); // Initialize the logger
            let config = maden_config::load().expect("Failed to load server configuration");
            let mut maden = maden_core::Maden::new();

            for factory in inventory::iter::<maden_core::HandlerFactory>() {
                (factory.0)(&mut maden);
            }

            maden.run(config.server).await;
        }
    };

    expanded.into()
}