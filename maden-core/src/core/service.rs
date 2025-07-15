use std::{
    collections::HashMap,
    convert::Infallible,
    future::Future,
    pin::Pin,
};

use http_body_util::{BodyExt, Full};
use hyper::{
    body::{Bytes, Incoming},
    service::Service,
    Request as HyperRequest, Response as HyperResponse,
};

use crate::core::http::{HttpMethod, Request, Response};
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

        maden_log::info!("Incoming request: {method:?} {path}");

        let (matched_handler, extracted_params) = self.routes.get(&method)
            .and_then(|router| router.at(&path).ok())
            .map(|m| (Some(m.value.clone()), m.params.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()))
            .unwrap_or((None, HashMap::new()));

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