
use axum::{
    debug_handler,
    extract::{Path, State},
    routing::{get, put},
    Extension, Json, Router,
};
use uuid::Uuid;

use crate::{
    middleware::check_member_access, repositories::{
        member::{Member},
        role::RoleMember,
    }
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/members/:user_id", get(get_member))
        .route("/members/:user_id", put(update_member))
        .route("/members/:user_id/roles", get(get_member_roles))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            check_member_access,
        ))
}

#[debug_handler]
async fn get_member(
    Extension(access_token): Extension<Option<String>>,
    State(state): State<AppState>,
    Path((user_id,)): Path<(Uuid,)>,
) -> Result<Json<Member>, axum::http::StatusCode> {
    println!("Access token: {:?}", access_token);
    let member = state
        .member_service
        .get_member(user_id, access_token.unwrap())
        .await;

    if let Err(e) = &member {
        println!("Error fetching member: {:?}", e);
    }

    // TODO: Return proper status code
    member
        .map(Json)
        .map_err(|_e| axum::http::StatusCode::NOT_FOUND)
}

#[debug_handler]
async fn get_member_roles(
    State(state): State<AppState>,
    Path((user_id,)): Path<(Uuid,)>,
) -> Result<Json<Vec<RoleMember>>, axum::http::StatusCode> {
    let roles = state.role_service.get_member_roles(user_id).await;

    if let Err(e) = &roles {
        println!("Error fetching member roles: {:?}", e);
    }

    // TODO return proper status code
    roles
        .map(Json)
        .map_err(|_e| axum::http::StatusCode::NOT_FOUND)
}

#[debug_handler]
async fn update_member(
    Extension(access_token): Extension<Option<String>>,
    State(state): State<AppState>,
    Path((user_id,)): Path<(Uuid,)>,
    Json(updated_member): Json<Member>,
) -> Result<Json<Member>, String> {
    let member = state
        .member_service
        .update_member(updated_member, access_token.unwrap())
        .await;

    if let Err(e) = &member {
        println!("Error updating member: {:?}", e);
    }

    member.map(Json).map_err(|e| e.to_string())
}
