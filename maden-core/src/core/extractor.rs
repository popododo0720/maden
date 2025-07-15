use std::collections::HashMap;
use serde::de::DeserializeOwned;
use crate::core::http::Request;
use crate::core::error::MadenError;

/// Trait for extracting data from HTTP requests
pub trait FromRequest: Sized {
    async fn from_request(req: &Request) -> Result<Self, MadenError>;
}

/// Extract path parameters
pub struct Path<T>(pub T);

impl<T> FromRequest for Path<T>
where
    T: DeserializeOwned,
{
    async fn from_request(req: &Request) -> Result<Self, MadenError> {
        let json_value = serde_json::to_value(&req.path_params)
            .map_err(|e| MadenError::bad_request(format!("Failed to serialize path params: {}", e)))?;
        
        let extracted = serde_json::from_value(json_value)
            .map_err(|e| MadenError::bad_request(format!("Failed to extract path params: {}", e)))?;
        
        Ok(Path(extracted))
    }
}

/// Extract query parameters
pub struct Query<T>(pub T);

impl<T> FromRequest for Query<T>
where
    T: DeserializeOwned,
{
    async fn from_request(req: &Request) -> Result<Self, MadenError> {
        // Convert query parameters to a format that serde can handle
        let mut converted_params = std::collections::HashMap::new();
        
        for (key, value) in &req.query_params {
            // Try to parse as number first, then fall back to string
            if let Ok(num) = value.parse::<i64>() {
                converted_params.insert(key.clone(), serde_json::Value::Number(serde_json::Number::from(num)));
            } else if let Ok(num) = value.parse::<f64>() {
                converted_params.insert(key.clone(), serde_json::Value::Number(serde_json::Number::from_f64(num).unwrap_or(serde_json::Number::from(0))));
            } else if value == "true" || value == "false" {
                converted_params.insert(key.clone(), serde_json::Value::Bool(value == "true"));
            } else {
                converted_params.insert(key.clone(), serde_json::Value::String(value.clone()));
            }
        }
        
        let json_value = serde_json::Value::Object(converted_params.into_iter().collect());
        
        let extracted = serde_json::from_value(json_value)
            .map_err(|e| MadenError::bad_request(format!("Failed to extract query params: {}", e)))?;
        
        Ok(Query(extracted))
    }
}

/// Extract JSON body
pub struct Json<T>(pub T);

impl<T> FromRequest for Json<T>
where
    T: DeserializeOwned,
{
    async fn from_request(req: &Request) -> Result<Self, MadenError> {
        let body_str = String::from_utf8(req.body.clone())
            .map_err(|e| MadenError::bad_request(format!("Invalid UTF-8 in request body: {}", e)))?;
        
        let extracted = serde_json::from_str(&body_str)
            .map_err(|e| MadenError::bad_request(format!("Failed to parse JSON body: {}", e)))?;
        
        Ok(Json(extracted))
    }
}

// Implement FromRequest for primitive types (for path parameters)
impl FromRequest for String {
    async fn from_request(req: &Request) -> Result<Self, MadenError> {
        // This will be handled by the macro for specific path parameters
        Err(MadenError::internal_server_error("String extraction should be handled by macro"))
    }
}

impl FromRequest for u32 {
    async fn from_request(req: &Request) -> Result<Self, MadenError> {
        Err(MadenError::internal_server_error("u32 extraction should be handled by macro"))
    }
}

impl FromRequest for u64 {
    async fn from_request(req: &Request) -> Result<Self, MadenError> {
        Err(MadenError::internal_server_error("u64 extraction should be handled by macro"))
    }
}

impl FromRequest for i32 {
    async fn from_request(req: &Request) -> Result<Self, MadenError> {
        Err(MadenError::internal_server_error("i32 extraction should be handled by macro"))
    }
}

impl FromRequest for i64 {
    async fn from_request(req: &Request) -> Result<Self, MadenError> {
        Err(MadenError::internal_server_error("i64 extraction should be handled by macro"))
    }
}

// Helper function to extract a single path parameter by name
pub fn extract_path_param<T>(req: &Request, param_name: &str) -> Result<T, MadenError>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let param_value = req.path_params.get(param_name)
        .ok_or_else(|| MadenError::bad_request(format!("Missing path parameter: {}", param_name)))?;
    
    param_value.parse::<T>()
        .map_err(|e| MadenError::bad_request(format!("Failed to parse path parameter '{}': {}", param_name, e)))
}