use maden_macros::handler;
use maden_core::{Path, Query, Json, MadenError};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: String,
    pub active: bool,
}

#[derive(Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub active: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct SearchQuery {
    pub name: Option<String>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Serialize, Deserialize)]
pub struct UserParams {
    pub id: u32,
}

pub struct AdvancedHandler;

#[handler]
impl AdvancedHandler {
    // 경로 매개변수를 직접 받기
    #[get("/users/{id}")]
    pub async fn get_user(id: u32) -> Result<User, MadenError> {
        println!("Getting user with ID: {}", id);
        Ok(User {
            id,
            name: format!("User {}", id),
            email: format!("user{}@example.com", id),
            active: true,
        })
    }

    // 쿼리 매개변수를 구조체로 받기
    #[get("/users")]
    pub async fn search_users(Query(query): Query<SearchQuery>) -> Result<Vec<User>, MadenError> {
        println!("Searching users with query: {:?}", serde_json::to_string(&query).unwrap());
        
        let page = query.page.unwrap_or(1);
        let limit = query.limit.unwrap_or(10);
        
        let mut users = Vec::new();
        for i in 1..=limit {
            let id = (page - 1) * limit + i;
            users.push(User {
                id,
                name: query.name.clone().unwrap_or_else(|| format!("User {}", id)),
                email: format!("user{}@example.com", id),
                active: true,
            });
        }
        
        Ok(users)
    }

    // JSON 바디를 구조체로 받기
    #[post("/users")]
    pub async fn create_user(Json(user_data): Json<CreateUserRequest>) -> Result<User, MadenError> {
        println!("Creating user: {:?}", serde_json::to_string(&user_data).unwrap());
        
        Ok(User {
            id: 999, // 실제로는 DB에서 생성된 ID
            name: user_data.name,
            email: user_data.email,
            active: true,
        })
    }

    // 경로 매개변수와 JSON 바디를 함께 받기
    #[put("/users/{id}")]
    pub async fn update_user(id: u32, Json(update_data): Json<UpdateUserRequest>) -> Result<User, MadenError> {
        println!("Updating user {} with data: {:?}", id, serde_json::to_string(&update_data).unwrap());
        
        Ok(User {
            id,
            name: update_data.name.unwrap_or_else(|| format!("User {}", id)),
            email: update_data.email.unwrap_or_else(|| format!("user{}@example.com", id)),
            active: update_data.active.unwrap_or(true),
        })
    }

    // 경로 매개변수를 구조체로 받기 (Path wrapper 사용)
    #[delete("/users/{id}")]
    pub async fn delete_user(Path(params): Path<UserParams>) -> Result<String, MadenError> {
        println!("Deleting user with ID: {}", params.id);
        Ok(format!("User {} has been deleted", params.id))
    }

    // 복잡한 경로 매개변수 (여러 개)
    #[get("/users/{user_id}/posts/{post_id}")]
    pub async fn get_user_post(user_id: u32, post_id: u32) -> Result<serde_json::Value, MadenError> {
        println!("Getting post {} for user {}", post_id, user_id);
        
        Ok(serde_json::json!({
            "user_id": user_id,
            "post_id": post_id,
            "title": format!("Post {} by User {}", post_id, user_id),
            "content": "This is a sample post content."
        }))
    }

    // 쿼리 매개변수와 경로 매개변수를 함께 사용
    #[get("/users/{id}/posts")]
    pub async fn get_user_posts(id: u32, Query(query): Query<SearchQuery>) -> Result<Vec<serde_json::Value>, MadenError> {
        println!("Getting posts for user {} with query: {:?}", id, serde_json::to_string(&query).unwrap());
        
        let limit = query.limit.unwrap_or(5);
        let mut posts = Vec::new();
        
        for i in 1..=limit {
            posts.push(serde_json::json!({
                "id": i,
                "user_id": id,
                "title": format!("Post {} by User {}", i, id),
                "content": "Sample post content"
            }));
        }
        
        Ok(posts)
    }
}