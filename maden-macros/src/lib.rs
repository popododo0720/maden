use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ImplItem, ItemFn, ItemImpl, LitStr, Type, Ident, FnArg, Pat, PatType};
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
                if attr.path().is_ident("get") || attr.path().is_ident("post") || attr.path().is_ident("put") || attr.path().is_ident("delete") {
                    if let Ok(args) = attr.parse_args::<HandlerArgs>() {
                        path_str = Some(args.path.value());
                        query_str = args.query.map(|q| q.value());

                        if attr.path().is_ident("get") {
                            http_method = Some(quote! { maden_core::HttpMethod::Get });
                        } else if attr.path().is_ident("post") {
                            http_method = Some(quote! { maden_core::HttpMethod::Post });
                        } else if attr.path().is_ident("put") {
                            http_method = Some(quote! { maden_core::HttpMethod::Put });
                        } else if attr.path().is_ident("delete") {
                            http_method = Some(quote! { maden_core::HttpMethod::Delete });
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

                // Parse function parameters to generate extraction code
                let mut param_extractions = Vec::new();
                let mut param_names = Vec::new();
                let mut path_params = Vec::new();

                // Extract path parameter names from the route path
                let path_param_names: Vec<String> = path.split('/')
                    .filter_map(|segment| {
                        if segment.starts_with('{') && segment.ends_with('}') {
                            Some(segment[1..segment.len()-1].to_string())
                        } else {
                            None
                        }
                    })
                    .collect();

                for input in &method.sig.inputs {
                    if let FnArg::Typed(PatType { pat, ty, .. }) = input {
                        // Handle different pattern types
                        match &**pat {
                            Pat::Ident(pat_ident) => {
                                let param_name = &pat_ident.ident;
                                let param_name_str = param_name.to_string();
                                
                                // Check if it's the Request type - pass it directly
                                if let Type::Path(type_path) = &**ty {
                                    let type_name = &type_path.path.segments.last().unwrap().ident;
                                    
                                    if type_name == "Request" {
                                        // Pass Request directly
                                        param_extractions.push(quote! {
                                            let #param_name = req.clone();
                                        });
                                        param_names.push(param_name.clone());
                                        continue;
                                    }
                                }
                                
                                // Check if this parameter matches a path parameter
                                if path_param_names.contains(&param_name_str) {
                                    // This is a path parameter - extract it directly
                                    path_params.push(param_name.clone());
                                    param_extractions.push(quote! {
                                        let #param_name = maden_core::extract_path_param::<#ty>(&req, #param_name_str)?;
                                    });
                                    param_names.push(param_name.clone());
                                } else {
                                    // Check if it's a wrapper type (Path, Query, Json)
                                    if let Type::Path(type_path) = &**ty {
                                        let type_name = &type_path.path.segments.last().unwrap().ident;
                                        
                                        if type_name == "Path" || type_name == "Query" || type_name == "Json" {
                                            // Extract using FromRequest trait
                                            param_extractions.push(quote! {
                                                let #param_name = <#ty as maden_core::FromRequest>::from_request(&req).await?;
                                            });
                                            param_names.push(param_name.clone());
                                        } else {
                                            // Try to extract as JSON body for custom types
                                            param_extractions.push(quote! {
                                                let #param_name = {
                                                    let body_str = String::from_utf8(req.body.clone())
                                                        .map_err(|e| maden_core::MadenError::bad_request(format!("Invalid UTF-8 in request body: {}", e)))?;
                                                    serde_json::from_str::<#ty>(&body_str)
                                                        .map_err(|e| maden_core::MadenError::bad_request(format!("Failed to parse JSON body: {}", e)))?
                                                };
                                            });
                                            param_names.push(param_name.clone());
                                        }
                                    } else {
                                        // For other types, try to parse as JSON body
                                        param_extractions.push(quote! {
                                            let #param_name = {
                                                let body_str = String::from_utf8(req.body.clone())
                                                    .map_err(|e| maden_core::MadenError::bad_request(format!("Invalid UTF-8 in request body: {}", e)))?;
                                                serde_json::from_str::<#ty>(&body_str)
                                                    .map_err(|e| maden_core::MadenError::bad_request(format!("Failed to parse JSON body: {}", e)))?
                                            };
                                        });
                                        param_names.push(param_name.clone());
                                    }
                                }
                            },
                            Pat::TupleStruct(tuple_struct) => {
                                // Handle patterns like Query(query), Json(data), Path(params)
                                if let Some(wrapper_name) = tuple_struct.path.segments.last() {
                                    let wrapper_ident = &wrapper_name.ident;
                                    
                                    if wrapper_ident == "Query" || wrapper_ident == "Json" || wrapper_ident == "Path" {
                                        // Extract the inner variable name
                                        if let Some(Pat::Ident(inner_pat)) = tuple_struct.elems.first() {
                                            let inner_name = &inner_pat.ident;
                                            
                                            // Extract using FromRequest trait
                                            param_extractions.push(quote! {
                                                let #wrapper_ident(#inner_name) = <#ty as maden_core::FromRequest>::from_request(&req).await?;
                                            });
                                            param_names.push(inner_name.clone());
                                        }
                                    }
                                }
                            },
                            _ => {
                                // Handle other pattern types if needed
                            }
                        }
                    }
                }

                let return_type = &method.sig.output;
                let response_conversion = match return_type {
                    syn::ReturnType::Default => { // -> ()
                        quote! { 
                            #(#param_extractions)*
                            #struct_name::#method_name(#(#param_names),*).await;
                            Ok(maden_core::Response::new(200))
                        }
                    },
                    syn::ReturnType::Type(_, ty) => { // -> Type
                        if let Type::Path(type_path) = &**ty {
                            let last_segment = type_path.path.segments.last().unwrap();
                            let type_name = &last_segment.ident;

                            if type_name == "Response" {
                                quote! { 
                                    #(#param_extractions)*
                                    Ok(#struct_name::#method_name(#(#param_names),*).await)
                                }
                            } else if type_name == "String" {
                                quote! { 
                                    #(#param_extractions)*
                                    let result = #struct_name::#method_name(#(#param_names),*).await;
                                    Ok(maden_core::Response::new(200).text(&result))
                                }
                            } else if type_name == "Result" {
                                quote! {
                                    #(#param_extractions)*
                                    match #struct_name::#method_name(#(#param_names),*).await {
                                        Ok(value) => Ok(maden_core::Response::new(200).json(value)),
                                        Err(err) => Ok(err.into_response()),
                                    }
                                }
                            } else {
                                // Assume it's a serializable type
                                quote! { 
                                    #(#param_extractions)*
                                    let result = #struct_name::#method_name(#(#param_names),*).await;
                                    Ok(maden_core::Response::new(200).json(result))
                                }
                            }
                        } else {
                            // Fallback for other complex types
                            quote! { 
                                #(#param_extractions)*
                                let result = #struct_name::#method_name(#(#param_names),*).await;
                                Ok(maden_core::Response::new(200).json(result))
                            }
                        }
                    }
                };

                routes_registration.push(quote! {
                    maden.add_route(
                        #http_method,
                        &#path,
                        #query_arg,
                        Box::new(|req| Box::pin(async move { 
                            let result: Result<maden_core::Response, maden_core::MadenError> = async {
                                #response_conversion
                            }.await;
                            
                            match result {
                                Ok(response) => response,
                                Err(error) => error.into_response(),
                            }
                        })),
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
            use maden_core::IntoResponse;
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
pub fn put(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn delete(_attr: TokenStream, item: TokenStream) -> TokenStream {
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

            maden.run(config).await;
        }
    };

    expanded.into()
}