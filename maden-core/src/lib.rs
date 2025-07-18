pub mod core;

pub use core::http::{HttpMethod, Request, Response, IntoResponse};
pub use core::error::MadenError;
pub use core::extractor::{FromRequest, Path, Query, Json, extract_path_param};
pub use crate::core::server::Maden;
pub use maden_macros::handler;

pub struct HandlerFactory(pub fn(&mut Maden));

inventory::collect!(HandlerFactory);

pub type MadenRoutes = crate::core::server::MadenRoutes;
