use axum::{
    debug_handler,
    extract::State,
    routing::{get, post},
    Json, Router,
};

use crate::repositories::role::{Role, RoleStats};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(get_roles))
        .route("/", post(post_role))
        .route("/stats", get(get_roles_stats))
        .with_state(state)
}

async fn get_roles(State(state): State<AppState>) -> Result<Json<Vec<Role>>, String> {
    println!("Fetching roles");
    let roles = state.role_service.get_all_roles().await;

    if let Err(e) = &roles {
        println!("Error fetching roles: {:?}", e);
        return Err(e.to_string());
    }

    roles.map(Json)
}

async fn get_roles_stats(State(state): State<AppState>) -> Result<Json<Vec<RoleStats>>, String> {
    let roles = state.role_service.get_role_stats().await;

    if let Err(e) = &roles {
        println!("Error fetching roles: {:?}", e);
        return Err(e.to_string());
    }

    roles.map(Json)
}

#[debug_handler]
async fn post_role(
    State(state): State<AppState>,
    Json(new_role): Json<Role>,
) -> Result<Json<Role>, String> {
    let role = state.role_service.create_role(new_role).await;

    if let Err(e) = &role {
        println!("Error creating role: {:?}", e);
    }

    role.map(Json).map_err(|e| e.to_string())
}
