use maden_macros::{handler};
use maden_core::{Request};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct MyData {
    pub id: u32,
    pub name: String,
    pub active: bool,
}

pub struct TestHandler;

#[handler]
impl TestHandler {
    #[get("/")]
    pub async fn hello_world(_req: Request) -> String {
        println!("/");
        format!("GET / received! This is a new line from hello_world.")
    }

    #[get("/test")]
    pub async fn get_test(_req: Request) -> String {
        println!("/test");
        format!("GET /test received! Another line for test.")
    }

    #[get("/test/:id")]
    pub async fn get_test_id(req: Request) -> String {
        let id = req.path_params.get("id").unwrap_or(&"unknown".to_string()).clone();
        println!("/test/:{id}");
        format!("GET /test/{id} received! ID: {id} Path parameter test.")
    }

    #[post("/test")]
    pub async fn post_test(req: Request) -> String {
        let body_str = String::from_utf8_lossy(&req.body).to_string();
        println!("/test : {body_str}");
        format!("POST /test received! Body: {body_str} Body echo test.")
    }

    #[get("/json-example")]
    pub async fn json_example(_req: Request) -> serde_json::Value {
        println!("/json-example");
        serde_json::json!({
            "status": "success",
            "message": "This is a JSON response from Maden.",
            "data": {
                "key1": "value1",
                "key2": 123,
                "nested": {
                    "item": "example"
                }
            }
        })
    }

    #[get("/auto-json")]
    pub async fn auto_json_response(_req: Request) -> MyData {
        maden_log::info!("auto_json");
        MyData {
            id: 1,
            name: "Auto JSON Test".to_string(),
            active: true,
        }
    }
}
