use maden_macros::{handler};
use maden_core::{Request, MadenError};
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
    pub async fn hello_world(_req: Request) -> Result<String, MadenError> {
        println!("/");
        Ok("GET / received! This is a new line from hello_world.".to_string())
    }

    #[get("/test")]
    pub async fn get_test(_req: Request) -> Result<String, MadenError> {
        println!("/test");
        Ok("GET /test received! Another line for test.".to_string())
    }

    #[get("/test/:id")]
    pub async fn get_test_id(req: Request) -> Result<String, MadenError> {
        let id = req.path_params.get("id").unwrap_or(&"unknown".to_string()).clone();
        println!("/test/:{id}");
        Ok(format!("GET /test/{id} received! ID: {id} Path parameter test."))
    }

    #[post("/test")]
    pub async fn post_test(req: Request) -> Result<String, MadenError> {
        let body_str = String::from_utf8_lossy(&req.body).to_string();
        println!("/test : {body_str}");
        Ok(format!("POST /test received! Body: {body_str} Body echo test."))
    }

    #[get("/json-example")]
    pub async fn json_example(_req: Request) -> Result<serde_json::Value, MadenError> {
        println!("/json-example");
        Ok(serde_json::json!({
            "status": "success",
            "message": "This is a JSON response from Maden.",
            "data": {
                "key1": "value1",
                "key2": 123,
                "nested": {
                    "item": "example"
                }
            }
        }))
    }

    #[get("/auto-json")]
    pub async fn auto_json_response(_req: Request) -> Result<MyData, MadenError> {
        println!("/auto-json");
        Ok(MyData {
            id: 1,
            name: "Auto JSON Test".to_string(),
            active: true,
        })
    }

    #[get("/error-example")]
    pub async fn error_example(_req: Request) -> Result<MyData, MadenError> {
        println!("/error-example");
        // 400 Bad Request 에러 반환 예시
        if _req.query_params.get("type").is_some_and(|s| s == "bad") {
            return Err(MadenError::bad_request("Invalid request parameter."));
        }
        // 401 Unauthorized 에러 반환 예시
        if _req.query_params.get("type").is_some_and(|s| s == "unauthorized") {
            return Err(MadenError::unauthorized("Authentication required."));
        }
        // 500 Internal Server Error 반환 예시
        if _req.query_params.get("type").is_some_and(|s| s == "internal") {
            return Err(MadenError::internal_server_error("Something went wrong on the server."));
        }

        Ok(MyData {
            id: 2,
            name: "Error Example Success".to_string(),
            active: false,
        })
    }
}