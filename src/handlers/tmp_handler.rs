use maden_macros::{handler};
use maden_core::{Request, IntoResponse};

pub struct TmpHandler;

#[handler]
impl TmpHandler {
    #[get("/tmp")]
    pub async fn gettmp(_req: Request) -> impl IntoResponse {
        println!("/tmp");
        "Hello from TmpHandler!"
    }
}