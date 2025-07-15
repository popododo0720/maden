use std::collections::HashMap;
use hyper::body::Bytes;
use http_body_util::Full;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Options,
    Head,
}

impl HttpMethod {
    pub fn from_hyper(method: &hyper::Method) -> Option<Self> {
        match *method {
            hyper::Method::GET => Some(HttpMethod::Get),
            hyper::Method::POST => Some(HttpMethod::Post),
            hyper::Method::PUT => Some(HttpMethod::Put),
            hyper::Method::DELETE => Some(HttpMethod::Delete),
            hyper::Method::PATCH => Some(HttpMethod::Patch),
            hyper::Method::OPTIONS => Some(HttpMethod::Options),
            hyper::Method::HEAD => Some(HttpMethod::Head),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct Request {
    pub method: HttpMethod,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub path_params: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn new(
        method: HttpMethod,
        path: String,
        headers: HashMap<String, String>,
        path_params: HashMap<String, String>,
        query_params: HashMap<String, String>,
        body: Vec<u8>,
    ) -> Self {
        Self {
            method,
            path,
            headers,
            path_params,
            query_params,
            body,
        }
    }
}

pub struct Response {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status_code: u16) -> Self {
        Self {
            status_code,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn json(mut self, data: impl serde::Serialize) -> Self {
        self.headers.insert("Content-Type".to_string(), "application/json".to_string());
        let mut json_body = serde_json::to_vec(&data).unwrap_or_default();
        json_body.extend_from_slice(b"\r\n"); // Add CRLF at the end
        self.body = json_body;
        self.headers.insert("Content-Length".to_string(), self.body.len().to_string());
        self
    }

    pub fn text(mut self, text: &str) -> Self {
        self.headers.insert("Content-Type".to_string(), "text/plain".to_string());
        let mut body_string = text.to_string();
        if !body_string.ends_with("\r\n") {
            if body_string.ends_with('\n') {
                body_string.pop(); // Remove existing LF if present
            }
            body_string.push_str("\r\n");
        }
        self.body = body_string.as_bytes().to_vec();
        self.headers.insert("Content-Length".to_string(), self.body.len().to_string());
        self
    }

    pub fn html(mut self, html: &str) -> Self {
        self.headers.insert("Content-Type".to_string(), "text/html".to_string());
        self.body = html.as_bytes().to_vec();
        self
    }
}

pub trait IntoResponse {
    fn into_response(self) -> Response;
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}



impl From<Response> for hyper::Response<Full<Bytes>> {
    fn from(maden_res: Response) -> Self {
        let mut builder = hyper::Response::builder().status(maden_res.status_code);
        for (key, value) in maden_res.headers {
            builder = builder.header(key, value);
        }
        builder
            .body(Full::new(Bytes::from(maden_res.body)))
            .unwrap()
    }
}


