pub mod core;

pub use core::http::{HttpMethod, Request, Response, IntoResponse, RoutePattern};
pub use core::error::MadenError;
pub use core::server::Maden;
pub use maden_macros::handler;

pub struct HandlerFactory(pub fn(&mut Maden));

inventory::collect!(HandlerFactory);

pub type MadenRoutes = std::sync::Arc<std::sync::Mutex<std::collections::HashMap<HttpMethod, std::collections::HashMap<RoutePattern, std::sync::Arc<core::service::Handler>>>>>;
