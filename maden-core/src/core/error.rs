use std::fmt;

use hyper::StatusCode;
use serde::Serialize;

use crate::core::http::Response;
use crate::IntoResponse;

#[derive(Debug, Serialize)]
pub struct MadenError {
    pub status: u16,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl MadenError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status: status.as_u16(),
            message: message.into(),
            error: None,
        }
    }

    pub fn with_error(status: StatusCode, message: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            status: status.as_u16(),
            message: message.into(),
            error: Some(error.into()),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    pub fn internal_server_error(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }
}

impl fmt::Display for MadenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MadenError {{ status: {}, message: {} }}", self.status, self.message)
    }
}

impl std::error::Error for MadenError {}

impl IntoResponse for MadenError {
    fn into_response(self) -> Response {
        Response::new(self.status).json(self)
    }
}

impl From<hyper::Error> for MadenError {
    fn from(err: hyper::Error) -> Self {
        Self::internal_server_error(err.to_string())
    }
}

impl From<serde_json::Error> for MadenError {
    fn from(err: serde_json::Error) -> Self {
        Self::bad_request(err.to_string())
    }
}

impl From<std::io::Error> for MadenError {
    fn from(err: std::io::Error) -> Self {
        Self::internal_server_error(err.to_string())
    }
}