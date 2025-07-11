use std::{
    collections::HashMap,
    convert::Infallible,
    future::Future,
    pin::Pin,
    sync::Arc,
};

use http_body_util::{BodyExt, Full};
use hyper::{
    body::{Bytes, Incoming},
    service::Service,
    Request as HyperRequest, Response as HyperResponse,
};

use crate::core::http::{HttpMethod, Request, Response, RoutePattern};
use crate::MadenRoutes;
use crate::MadenError;
use crate::IntoResponse;

pub type Handler = Box<dyn Fn(Request) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync>;

#[derive(Clone)]
pub struct MadenService {
    pub routes: MadenRoutes,
}

impl Service<HyperRequest<Incoming>> for MadenService {
    type Response = HyperResponse<Full<Bytes>>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, hyper_req: HyperRequest<Incoming>) -> Self::Future {
        let method = HttpMethod::from_hyper(hyper_req.method()).unwrap();
        let path = hyper_req.uri().path().to_string();
        let query_string = hyper_req.uri().query().map(|s| s.to_string());

        maden_log::info!("Incoming request: {method:?} {path}");

        let routes = self.routes.lock().unwrap();
        let mut matched_handler: Option<Arc<Handler>> = None;
        let mut extracted_params: HashMap<String, String> = HashMap::new();

        if let Some(path_map) = routes.get(&method) {
            // Sort routes by specificity (more query conditions first)
            let mut sorted_routes: Vec<(&RoutePattern, &Arc<Handler>)> = path_map.iter().collect();
            sorted_routes.sort_by(|(a, _), (b, _)| b.query_conditions.len().cmp(&a.query_conditions.len()));

            for (route_pattern, handler) in sorted_routes.into_iter() {
                // Check path match
                let mut current_params = HashMap::new();
                let path_matches = if route_pattern.path.contains(':') {
                    let route_parts: Vec<&str> = route_pattern.path.split('/').collect();
                    let request_parts: Vec<&str> = path.split('/').collect();

                    if route_parts.len() == request_parts.len() {
                        let mut is_match = true;
                        for (i, &route_part) in route_parts.iter().enumerate() {
                            if route_part.starts_with(':') {
                                let param_name = route_part.trim_start_matches(':').to_string();
                                current_params.insert(param_name, request_parts[i].to_string());
                            } else if route_part != request_parts[i] {
                                is_match = false;
                                break;
                            }
                        }
                        is_match
                    } else {
                        false
                    }
                } else {
                    route_pattern.path == path
                };

                if path_matches {
                    // Check query parameter match
                    let query_matches = route_pattern.query_conditions.iter().all(|(key, expected_value)| {
                        if let Some(req_query_string) = &query_string {
                            let req_query_params: HashMap<String, Option<String>> = req_query_string
                                .split('&')
                                .filter_map(|pair| {
                                    let mut parts = pair.splitn(2, '=');
                                    Some((parts.next()?.to_string(), parts.next().map(|s| s.to_string())))
                                })
                                .collect();

                            if let Some(actual_value) = req_query_params.get(key) {
                                expected_value.as_ref().is_none_or(|ev| actual_value.as_ref() == Some(ev))
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    });

                    if query_matches {
                        matched_handler = Some(handler.clone());
                        extracted_params = current_params;
                        break;
                    }
                }
            }
        }

        Box::pin(async move {
            let (parts, body) = hyper_req.into_parts();
            let body_bytes = body.collect().await.unwrap().to_bytes();

            let mut headers = HashMap::new();
            for (name, value) in parts.headers.iter() {
                headers.insert(name.to_string(), value.to_str().unwrap_or("").to_string());
            }

            let query_params = parts.uri.query().map_or_else(HashMap::new, |query| {
                query.split('&').filter_map(|pair| {
                    let mut parts = pair.splitn(2, '=');
                    Some((parts.next()?.to_string(), parts.next()?.to_string()))
                }).collect()
            });

            let maden_req = Request::new(
                method,
                path,
                headers,
                extracted_params,
                query_params,
                body_bytes.to_vec(),
            );

            maden_log::debug!("Request details: {{ method: {:?}, path: {:?}, headers: {:?}, path_params: {:?}, query_params: {:?}, body_len: {} }}",
                maden_req.method,
                maden_req.path,
                maden_req.headers,
                maden_req.path_params,
                maden_req.query_params,
                maden_req.body.len()
            );

            let maden_res = match matched_handler {
                Some(handler) => {
                    handler(maden_req).await
                },
                None => MadenError::not_found("Route not found.").into_response(),
            };

            let hyper_res: HyperResponse<Full<Bytes>> = maden_res.into_response().into();
            Ok(hyper_res)
        })
    }
}