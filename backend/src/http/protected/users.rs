use axum::{
    debug_handler,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post, put},
    Extension, Json, Router,
};
use chrono::format;
use uuid::Uuid;

use crate::{
    middleware::check_member_access,
    repositories::{
        member::{Member, NewMember},
        role::RoleMember,
    },
    services::auth0_service::AuthInfo,
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new().route("/me", get(get_me))
}

#[debug_handler]
async fn get_me(
    Extension(user_info): Extension<Option<AuthInfo>>,
    State(state): State<AppState>,
) -> Result<Json<AuthInfo>, StatusCode> {
    match user_info {
        Some(user_info) => Ok(Json(user_info)),
        None => Err(StatusCode::UNAUTHORIZED),
    }
}
