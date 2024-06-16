use axum::{
    debug_handler,
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};

use crate::repositories::role::Role;

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/roles", get(get_roles))
        .route("/roles", post(post_role))
        .route("/roles/:user_id", delete(delete_role))
        .with_state(state)
}

async fn get_roles(State(state): State<AppState>) -> Result<Json<Vec<Role>>, String> {
    let roles = state.role_service.get_all_roles().await;

    if let Err(e) = &roles {
        println!("Error fetching roles: {:?}", e);
    }

    roles.map(Json)
}

#[debug_handler]
async fn post_role(
    State(state): State<AppState>,
    Json(new_role): Json<Role>,
) -> Result<Json<Role>, String> {
    let role = state.role_service.create_role(&new_role.name).await;

    if let Err(e) = &role {
        println!("Error creating role: {:?}", e);
    }

    role.map(Json).map_err(|e| e.to_string())
}

async fn delete_role(
    Path(role_name): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<()>, String> {
    let role = state.role_service.delete_role(&role_name).await;

    if let Err(e) = &role {
        println!("Error deleting role: {:?}", e);
    }

    role.map(Json).map_err(|e| e.to_string())
}
