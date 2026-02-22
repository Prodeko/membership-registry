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
    http::errors::{ApiError, ApiResult},
    middleware::check_member_access,
    repositories::{
        member::{self, Member, NewMember},
        role::RoleMember,
    },
    services::identity_service::AuthInfo,
};

use super::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/:user_id", get(get_member))
        .route("/:user_id", put(update_member))
        .route("/:user_id/roles", get(get_member_roles))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            check_member_access,
        ))
        .route("/", post(post_member))
        .route("/me", get(get_me))
}

#[debug_handler]
async fn get_member(
    Extension(user_info): Extension<Option<AuthInfo>>,
    State(state): State<AppState>,
    Path((user_id,)): Path<(Uuid,)>,
) -> ApiResult<Json<Member>> {
    let member = state.member_service.get_member(user_id).await.map(Json)?;

    Ok(member)
}

#[debug_handler]
async fn get_me(
    Extension(user_info): Extension<Option<AuthInfo>>,
    State(state): State<AppState>,
) -> ApiResult<Json<Member>> {
    match user_info {
        Some(user_info) => {
            let member = state
                .member_service
                .get_member(user_info.user_id)
                .await
                .map(Json)?;

            Ok(member)
        }
        None => Err(ApiError::Unauthorized),
    }
}

#[debug_handler]
async fn get_member_roles(
    State(state): State<AppState>,
    Path((user_id,)): Path<(Uuid,)>,
) -> ApiResult<Json<Vec<RoleMember>>> {
    let roles = state
        .role_service
        .get_member_roles(user_id)
        .await
        .map(Json)?;

    Ok(roles)
}

#[debug_handler]
async fn update_member(
    Extension(user_info): Extension<Option<AuthInfo>>,
    State(state): State<AppState>,
    Path((user_id,)): Path<(Uuid,)>,
    Json(updated_member): Json<Member>,
) -> ApiResult<Json<Member>> {
    let actor_id = user_info.map(|u| u.user_id);
    let member = state
        .member_service
        .update_member(updated_member, actor_id)
        .await
        .map(Json)?;

    Ok(member)
}

#[debug_handler]
async fn post_member(
    Extension(user_info): Extension<Option<AuthInfo>>,
    State(state): State<AppState>,
    Json(new_member): Json<NewMember>,
) -> ApiResult<Json<Member>> {
    let user_info = match user_info {
        None => return Err(ApiError::Unauthorized),
        Some(user_info) => user_info,
    };

    if user_info.user_id != new_member.user_id {
        return Err(ApiError::BadRequest);
    }

    let member = state
        .member_service
        .create_member(new_member, Some(user_info.user_id))
        .await
        .map(Json)?;

    Ok(member)
}
