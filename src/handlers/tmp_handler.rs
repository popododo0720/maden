use maden_macros::{handler, get};
use maden_core::{Request, MadenError};

pub struct TmpHandler;

#[handler]
impl TmpHandler {
    #[get("/tmp")]
    pub async fn get_tmp(_req: Request) -> Result<String, MadenError> {
        println!("/tmp");
        Ok("Hello from TmpHandler!".to_string())
    }
}