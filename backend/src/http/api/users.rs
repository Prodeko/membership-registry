use axum::{
    debug_handler,
    extract::State,
    routing::{get, post},
    Json, Router,
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/users", get(get_users))
        .route("/users", post(create_user))
        .with_state(state)
}

#[debug_handler]
async fn get_users(
    State(state): State<AppState>,
) -> Result<Json<Vec<ory_client::models::Identity>>, String> {
    let users = state.user_service.get_all_users().await;

    if let Err(e) = &users {
        println!("Error fetching users: {:?}", e);
    }

    users.map(Json).map_err(|e| e.to_string())
}

#[debug_handler]
async fn create_user(
    State(state): State<AppState>,
) -> Result<Json<ory_client::models::Identity>, String> {
    let user = state
        .user_service
        .create_user("test@test.com", "Test", "User")
        .await;
    if let Err(e) = &user {
        println!("Error creating user: {:?}", e);
    }

    user.map(Json).map_err(|e| e.to_string())
}
