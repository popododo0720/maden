use maden_macros::{handler, get, post};
use maden_core::{Request, Response};

pub struct TestHandler;

#[handler]
impl TestHandler {
    #[get("/")]
    pub fn hello_world(_req: Request) -> Response {
        println!("/");
        Response::new(200)
            .with_body("Hello from TestHandler!")
            .with_header("Content-Type", "text/plain")
    }

    #[get("/test")]
    pub fn get_test(_req: Request) -> Response {
        println!("/test");
        Response::new(200)
            .with_body("GET /test received!")
            .with_header("Content-Type", "text/plain")
    }

    #[get("/test/:id")]
    pub fn get_test_id(req: Request) -> Response {
        let id = req.path_params.get("id").unwrap_or(&"unknown".to_string()).clone();
        println!("/test/:{}", id);
        Response::new(200)
            .with_body(format!("GET /test/{{id}} received! ID: {}", id))
            .with_header("Content-Type", "text/plain")
    }

    #[post("/test")]
    pub fn post_test(req: Request) -> Response {
        let body_str = String::from_utf8_lossy(&req.body).to_string();
        println!("/test : {}", body_str);
        Response::new(200)
            .with_body(format!("POST /test received! Body: {}", body_str))
            .with_header("Content-Type", "text/plain")
    }
}
