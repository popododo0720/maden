use maden_macros::handler;
use maden_core::{Request, Query, Json, MadenError};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct SimpleUser {
    pub id: u32,
    pub name: String,
    pub email: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
}

#[derive(Serialize, Deserialize)]
pub struct SearchParams {
    pub name: Option<String>,
    pub limit: Option<u32>,
}

pub struct SimpleHandler;

#[handler]
impl SimpleHandler {
    // 기본 Request 사용 (기존 방식)
    #[get("/simple")]
    pub async fn simple_get(_req: Request) -> Result<String, MadenError> {
        Ok("Simple GET works!".to_string())
    }

    // 경로 매개변수를 직접 받기 (새로운 방식)
    #[get("/simple/{id}")]
    pub async fn get_by_id(id: u32) -> Result<SimpleUser, MadenError> {
        println!("Getting user with ID: {}", id);
        Ok(SimpleUser {
            id,
            name: format!("User {}", id),
            email: Some(format!("user{}@example.com", id)),
        })
    }

    // 여러 경로 매개변수
    #[get("/simple/{user_id}/item/{item_id}")]
    pub async fn get_user_item(user_id: u32, item_id: u32) -> Result<serde_json::Value, MadenError> {
        println!("Getting item {} for user {}", item_id, user_id);
        Ok(serde_json::json!({
            "user_id": user_id,
            "item_id": item_id,
            "message": format!("Item {} belongs to User {}", item_id, user_id)
        }))
    }

    // 쿼리 매개변수를 구조체로 받기
    #[get("/simple/search")]
    pub async fn search_users(query: Query<SearchParams>) -> Result<Vec<SimpleUser>, MadenError> {
        let Query(params) = query;
        println!("Searching with params: {:?}", serde_json::to_string(&params).unwrap());
        
        let limit = params.limit.unwrap_or(5);
        let mut users = Vec::new();
        
        for i in 1..=limit {
            users.push(SimpleUser {
                id: i,
                name: params.name.clone().unwrap_or_else(|| format!("User {}", i)),
                email: Some(format!("user{}@example.com", i)),
            });
        }
        
        Ok(users)
    }

    // JSON 바디를 구조체로 받기
    #[post("/simple/users")]
    pub async fn create_user(user_data: Json<CreateUserRequest>) -> Result<SimpleUser, MadenError> {
        let Json(data) = user_data;
        println!("Creating user: {:?}", serde_json::to_string(&data).unwrap());
        
        Ok(SimpleUser {
            id: 999, // 실제로는 DB에서 생성된 ID
            name: data.name,
            email: Some(data.email),
        })
    }

    // 경로 매개변수와 JSON 바디를 함께 받기
    #[put("/simple/{id}")]
    pub async fn update_user(id: u32, user_data: Json<CreateUserRequest>) -> Result<SimpleUser, MadenError> {
        let Json(data) = user_data;
        println!("Updating user {} with data: {:?}", id, serde_json::to_string(&data).unwrap());
        
        Ok(SimpleUser {
            id,
            name: data.name,
            email: Some(data.email),
        })
    }
}