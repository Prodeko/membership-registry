use axum::{
    debug_handler,
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use futures_util::FutureExt;

use crate::{
    http::errors::ApiResult,
    repositories::role::{Role, RoleStats},
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(get_roles))
        .route("/", post(post_role))
        .route("/stats", get(get_roles_stats))
        .with_state(state)
}

async fn get_roles(State(state): State<AppState>) -> ApiResult<Json<Vec<Role>>> {
    let roles = state.role_service.get_all_roles().await.map(Json)?;

    Ok(roles)
}

async fn get_roles_stats(State(state): State<AppState>) -> ApiResult<Json<Vec<RoleStats>>> {
    let result = state.role_service.get_role_stats().await.map(Json)?;

    Ok(result)
}

#[debug_handler]
async fn post_role(
    State(state): State<AppState>,
    Json(new_role): Json<Role>,
) -> ApiResult<Json<Role>> {
    let role = state.role_service.create_role(new_role).await.map(Json)?;

    Ok(role)
}
